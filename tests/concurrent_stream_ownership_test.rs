// Red→Green regression for issue #642: server response streams must be
// handler-owned. A handler that obtains another handler's stream handle
// (e.g. through a shared global) must not be able to write to, flush, or
// close that stream — otherwise it can inject bytes into a sibling's
// response body or truncate it by closing the stream mid-response.
//
// Scenario: /stream starts a streaming response, publishes its handle in a
// global, writes one line, parks 500ms, writes a second line, closes. While
// it is parked, /intrude grabs the global handle and attempts write + flush +
// close, each in its own try block, reporting each outcome.
//   - Fixed: all three attempts error (caught), and the streamed body is
//     exactly the owner's two lines.
//   - Buggy: the write injects "INTRUDER" into the owner's body and the close
//     truncates it (the owner's second write then errors).

use std::time::Duration;
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

mod common;

fn start_server_thread(code: String) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let mut parser = Parser::new(&tokens);
            let ast = parser.parse().expect("parse");
            let mut interpreter = Interpreter::new();
            if let Err(errors) = interpreter.interpret(&ast).await {
                panic!("server interpreter failed: {errors:?}");
            }
        });
    })
}

async fn wait_for_server(port: u16) {
    let addr = format!("127.0.0.1:{port}");
    for _ in 0..300 {
        if tokio::net::TcpStream::connect(&addr).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server on {addr} did not become ready in time");
}

async fn shutdown(port: u16, server: std::thread::JoinHandle<()>) {
    let _ = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/shutdown"))
        .send()
        .await;
    match tokio::task::spawn_blocking(move || server.join()).await {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(join_err) => panic!("server join task failed: {join_err}"),
    }
}

#[tokio::test]
async fn test_sibling_handler_cannot_write_flush_or_close_anothers_stream() {
    let port = common::free_tcp_port();
    let code = format!(
        r#"
        store shared_handle as ""

        listen on port {port} as srv
        main loop concurrently:
            wait for request comes in on srv as req with timeout 20000
            store p as req["path"]
            check if p is equal to "/shutdown":
                respond to req with "bye"
                close server srv
                break
            otherwise:
                check if p is equal to "/stream":
                    start streaming response to req with status 200 as out
                    change shared_handle to out
                    write line "OWNER-1" to out
                    wait for 500 milliseconds
                    write line "OWNER-2" to out
                    close out
                otherwise:
                    store write_outcome as "write:allowed"
                    try:
                        write line "INTRUDER" to shared_handle
                    when error:
                        change write_outcome to "write:denied"
                    end try
                    store flush_outcome as "flush:allowed"
                    try:
                        flush shared_handle
                    when error:
                        change flush_outcome to "flush:denied"
                    end try
                    store close_outcome as "close:allowed"
                    try:
                        close shared_handle
                    when error:
                        change close_outcome to "close:denied"
                    end try
                    store outcome_summary as write_outcome with " " with flush_outcome with " " with close_outcome
                    respond to req with outcome_summary
                end check
            end check
        end loop
    "#
    );
    let server = start_server_thread(code);
    wait_for_server(port).await;

    // Open the streaming response, then intrude while its handler is parked.
    let stream_url = format!("http://127.0.0.1:{port}/stream");
    let stream = tokio::spawn(async move {
        reqwest::Client::new()
            .get(&stream_url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    });
    tokio::time::sleep(Duration::from_millis(150)).await;

    let intrude = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/intrude"))
        .send()
        .await
        .expect("/intrude request failed");
    let intrude_body = intrude.text().await.unwrap();
    assert_eq!(
        intrude_body, "write:denied flush:denied close:denied",
        "a sibling handler operated on a stream it does not own"
    );

    let stream_body = stream.await.expect("/stream task panicked");
    assert_eq!(
        stream_body, "OWNER-1\nOWNER-2\n",
        "the owner's streamed body was corrupted or truncated by a sibling"
    );

    shutdown(port, server).await;
}
