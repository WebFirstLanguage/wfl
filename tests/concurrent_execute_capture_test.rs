// Red→Green regression for issue #642: `execute file ... and read output`
// capture must be handler-local under `main loop concurrently:`.
//
// The capture stack that routes `display` output into the reading variable is
// a single thread-local shared by every handler on the interpreter thread. A
// handler that awaits mid-capture (the child file yields) leaves its buffer on
// top of that shared stack, so an interleaved sibling's output — its own
// captured child, or a plain `display` — lands in the wrong handler's buffer,
// and the LIFO guard pops mismatch when handlers finish out of push order.
//
// Two properties, each a separate test:
//   1. Two handlers capturing concurrently each read exactly their own
//      child's output.
//   2. A sibling's plain `display` (no capture) must not leak into another
//      handler's capture buffer.

use std::fs;
use std::time::Duration;
use tempfile::TempDir;
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

mod common;

fn start_server_thread(code: String) -> std::thread::JoinHandle<()> {
    common::spawn_interpreter_thread(move || {
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

/// Write a child WFL file that displays `first`, yields for `wait_ms`, then
/// displays `second` — the yield is what lets a sibling handler interleave
/// inside the capture window. Returns a forward-slash path safe to embed in a
/// WFL string literal on all platforms.
fn write_child(dir: &TempDir, name: &str, first: &str, second: &str, wait_ms: u32) -> String {
    let path = dir.path().join(name);
    fs::write(
        &path,
        format!("display \"{first}\"\nwait for {wait_ms} milliseconds\ndisplay \"{second}\"\n"),
    )
    .expect("write child wfl file");
    path.display().to_string().replace('\\', "/")
}

#[tokio::test]
async fn test_concurrent_captures_do_not_cross_wire() {
    let dir = TempDir::new().expect("tempdir");
    let child_a = write_child(&dir, "child_a.wfl", "ALPHA-1", "ALPHA-2", 250);
    let child_b = write_child(&dir, "child_b.wfl", "BRAVO-1", "BRAVO-2", 250);

    let port = common::free_tcp_port();
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop concurrently:
            wait for request comes in on srv as req with timeout 20000
            store p as req["path"]
            check if p is equal to "/shutdown":
                respond to req with "bye"
                close server srv
                break
            otherwise:
                check if p is equal to "/a":
                    execute wfl file at "{child_a}" and read output as child_output
                    respond to req with child_output
                otherwise:
                    execute wfl file at "{child_b}" and read output as child_output
                    respond to req with child_output
                end check
            end check
        end loop
    "#
    );
    let server = start_server_thread(code);
    wait_for_server(port).await;

    // Start /a first so its capture window is open, then fire /b so both
    // children are parked in their mid-file waits at the same time.
    let a_url = format!("http://127.0.0.1:{port}/a");
    let a = tokio::spawn(async move {
        reqwest::Client::new()
            .get(&a_url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    });
    tokio::time::sleep(Duration::from_millis(80)).await;
    let b_url = format!("http://127.0.0.1:{port}/b");
    let b = tokio::spawn(async move {
        reqwest::Client::new()
            .get(&b_url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    });

    let a_body = a.await.expect("/a task panicked");
    let b_body = b.await.expect("/b task panicked");

    assert!(
        a_body.contains("ALPHA-1") && a_body.contains("ALPHA-2"),
        "/a must capture its own child's full output, got: {a_body:?}"
    );
    assert!(
        !a_body.contains("BRAVO"),
        "/a captured a sibling handler's output (cross-wired capture): {a_body:?}"
    );
    assert!(
        b_body.contains("BRAVO-1") && b_body.contains("BRAVO-2"),
        "/b must capture its own child's full output, got: {b_body:?}"
    );
    assert!(
        !b_body.contains("ALPHA"),
        "/b captured a sibling handler's output (cross-wired capture): {b_body:?}"
    );

    shutdown(port, server).await;
}

#[tokio::test]
async fn test_sibling_display_does_not_leak_into_capture() {
    let dir = TempDir::new().expect("tempdir");
    let child = write_child(&dir, "child.wfl", "CHILD-1", "CHILD-2", 600);

    let port = common::free_tcp_port();
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop concurrently:
            wait for request comes in on srv as req with timeout 20000
            store p as req["path"]
            check if p is equal to "/shutdown":
                respond to req with "bye"
                close server srv
                break
            otherwise:
                check if p is equal to "/cap":
                    execute wfl file at "{child}" and read output as child_output
                    respond to req with child_output
                otherwise:
                    display "NOISE-FROM-SIBLING"
                    respond to req with "noise-done"
                end check
            end check
        end loop
    "#
    );
    let server = start_server_thread(code);
    wait_for_server(port).await;

    // Open the capture window, then have a sibling display while it is open.
    let cap_url = format!("http://127.0.0.1:{port}/cap");
    let cap = tokio::spawn(async move {
        reqwest::Client::new()
            .get(&cap_url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    let noise = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/noise"))
        .send()
        .await
        .expect("noise request failed");
    assert_eq!(noise.text().await.unwrap(), "noise-done");

    let cap_body = cap.await.expect("/cap task panicked");
    assert!(
        cap_body.contains("CHILD-1") && cap_body.contains("CHILD-2"),
        "/cap must capture its child's full output, got: {cap_body:?}"
    );
    assert!(
        !cap_body.contains("NOISE-FROM-SIBLING"),
        "a sibling handler's display leaked into another handler's capture: {cap_body:?}"
    );

    shutdown(port, server).await;
}
