//! Real-socket regression (maintainer re-review, P1): when the `interpret()` future
//! is DROPPED after a handler started a streaming response, the interpret-scoped
//! cleanup guard must close the server response stream (ending the client's body)
//! and 500 any unanswered request — even though the reusable `Interpreter` itself
//! stays alive.
//!
//! Previously the drop guard covered only outbound streams, so a dropped run left
//! `server_response_streams` alive on the still-alive interpreter and the client
//! hung. To prove it is the GUARD (not the interpreter's eventual drop) that closes
//! the body, the interpreter is held alive for well after the future is dropped: the
//! client body must end shortly after the drop, not when the interpreter drops.

use std::sync::Arc;
use std::time::{Duration, Instant};
use wfl::Interpreter;
use wfl::config::WflConfig;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

mod common;

async fn wait_for_server(port: u16) {
    let addr = format!("127.0.0.1:{port}");
    for _ in 0..300 {
        if tokio::net::TcpStream::connect(&addr).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server on {addr} did not become ready");
}

#[tokio::test]
async fn test_dropped_run_closes_server_stream_while_interpreter_stays_alive() {
    let port = common::free_tcp_port();

    // Handler: start streaming, send a chunk, flush, then park for a long time. The
    // run is dropped while it is parked here, before it ever closes the stream.
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop:
            wait for request comes in on srv as req with timeout 60000
            start streaming response to req with status 200 and content type "text/plain" as out
            write chunk "hello" to out
            flush out
            wait for 60000 milliseconds
        end loop
    "#
    );

    // The server runs in its own thread. It runs `interpret()` under a 3s timeout —
    // when that elapses the FUTURE is dropped (moved into `timeout`) — then holds
    // the interpreter ALIVE for 10 more seconds. So during [3s, 13s] the future is
    // gone but the interpreter lives: only the drop guard can end the client body.
    let server = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("server runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let program = Parser::new(&tokens).parse().expect("parse");
            let config = WflConfig {
                timeout_seconds: 60,
                ..WflConfig::default()
            };
            let mut interp = Interpreter::with_config(Arc::new(config));
            {
                let fut = interp.interpret(&program);
                let _ = tokio::time::timeout(Duration::from_secs(3), fut).await;
                // `fut` is dropped here (timeout took ownership and elapsed).
            }
            // Interpreter deliberately kept alive well past the drop.
            tokio::time::sleep(Duration::from_secs(10)).await;
            drop(interp);
        });
    });

    wait_for_server(port).await;

    // Client: read the streaming body. It must deliver "hello" and then END shortly
    // after the 3s drop (the guard closing the stream), NOT hang until the 13s
    // interpreter drop or the 60s handler park.
    let mut resp = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/"))
        .send()
        .await
        .expect("request send");
    assert_eq!(resp.status().as_u16(), 200);

    let start = Instant::now();
    let mut body = Vec::new();
    let mut ended = false;
    loop {
        match tokio::time::timeout(Duration::from_secs(8), resp.chunk()).await {
            Ok(Ok(Some(bytes))) => body.extend_from_slice(&bytes),
            // Clean EOF or a transport end both mean the body finished.
            Ok(Ok(None)) | Ok(Err(_)) => {
                ended = true;
                break;
            }
            Err(_) => break, // outer timeout: body never ended
        }
    }
    let elapsed = start.elapsed();

    assert!(
        String::from_utf8_lossy(&body).contains("hello"),
        "the streamed chunk should have been delivered before the drop; body: {:?}",
        String::from_utf8_lossy(&body)
    );
    assert!(
        ended,
        "the client body must END after the run was dropped (the guard closing the \
         server response stream), not hang"
    );
    assert!(
        elapsed < Duration::from_secs(7),
        "the body should end shortly after the ~3s drop (the guard), not at the ~13s \
         interpreter drop; took {elapsed:?}"
    );

    match tokio::task::spawn_blocking(move || server.join()).await {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(e) => panic!("server join task failed: {e}"),
    }
}

#[tokio::test]
async fn test_dropped_run_answers_pending_request_with_500() {
    // Exercise the dropped-run pending-request 500 branch: handler dequeues a
    // request and parks WITHOUT ever responding or starting a streaming response,
    // so the pending oneshot is still in `pending_responses` when interpret() is
    // dropped. The cleanup guard must answer 500 promptly (issue #642 R3).
    let port = common::free_tcp_port();
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop:
            wait for request comes in on srv as req with timeout 60000
            wait for 60000 milliseconds
        end loop
    "#
    );

    let server = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("server runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let program = Parser::new(&tokens).parse().expect("parse");
            let config = WflConfig {
                timeout_seconds: 60,
                ..WflConfig::default()
            };
            let mut interp = Interpreter::with_config(Arc::new(config));
            {
                let fut = interp.interpret(&program);
                // Drop after the handler has had time to dequeue and park.
                let _ = tokio::time::timeout(Duration::from_secs(2), fut).await;
            }
            // Keep the interpreter alive so only the drop guard can 500 the request.
            tokio::time::sleep(Duration::from_secs(8)).await;
            drop(interp);
        });
    });

    wait_for_server(port).await;

    let start = Instant::now();
    let resp = tokio::time::timeout(
        Duration::from_secs(6),
        reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/"))
            .send(),
    )
    .await
    .expect("client should not hang waiting for a pending request after interpret() drop")
    .expect("request failed");
    let elapsed = start.elapsed();

    assert_eq!(
        resp.status().as_u16(),
        500,
        "dropped run must answer the still-pending request with 500, got {}",
        resp.status()
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "500 should arrive shortly after the ~2s drop, not at the 60s request timeout; took {elapsed:?}"
    );

    match tokio::task::spawn_blocking(move || server.join()).await {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(e) => panic!("server join task failed: {e}"),
    }
}
