//! Real-socket regressions (maintainer re-review, P1):
//!
//! 1. A backpressured response-stream `write` must be BOUNDED: once the 64-slot
//!    channel fills, a client that stays connected but stops reading would pin the
//!    handler forever (`main loop` is deadline-exempt). It must instead time out at
//!    `web_server_response_timeout_seconds` and error, releasing the handler.
//!
//! 2. Streaming must actually STREAM: the head and an early chunk must be visible on
//!    the wire BEFORE the body completes/closes — not buffered until the end.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
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
async fn test_backpressured_write_to_a_non_reading_client_is_bounded() {
    let port = common::free_tcp_port();

    // Handler streams FAR more data than any OS send buffer can hold to a
    // connected-but-non-reading client: the socket buffer fills, then the 64-slot
    // channel fills, and the next `write` parks on backpressure. With a 2s response
    // timeout the parked write must fail (not hang forever); the serial main loop
    // propagates that error, so `interpret()` RETURNS instead of pinning the handler.
    // The payload is grown by doubling to ~40 KB so a few hundred chunks overflow
    // the buffer regardless of its autotuned size; the byte ceiling is raised so the
    // write blocks (not a budget rejection) first.
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop:
            wait for request comes in on srv as req with timeout 60000
            store payload as "0123456789"
            count from 1 to 12:
                store payload as payload with payload
            end count
            start streaming response to req with status 200 and content type "text/plain" as out
            count from 1 to 100000:
                write chunk payload to out
            end count
            close out
            break
        end loop
    "#
    );

    let (done_tx, done_rx) = tokio::sync::oneshot::channel::<Duration>();
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
                web_server_response_timeout_seconds: 2,
                web_server_max_response_size: 512 * 1024 * 1024,
                ..WflConfig::default()
            };
            let mut interp = Interpreter::with_config(Arc::new(config));
            let start = Instant::now();
            let _ = interp.interpret(&program).await;
            let _ = done_tx.send(start.elapsed());
        });
    });

    wait_for_server(port).await;

    // Connect, send the request, then NEVER read. Hold the socket open so the write
    // stalls on backpressure rather than a disconnect.
    let mut sock = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    sock.write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .await
        .expect("send request");
    sock.flush().await.ok();

    // `interpret()` must return once the stalled write times out (~2s). If the write
    // were unbounded it would pin the handler and this never fires.
    let elapsed = tokio::time::timeout(Duration::from_secs(12), done_rx)
        .await
        .expect("interpret() never returned — the backpressured write pinned the handler forever")
        .expect("done sender dropped");
    assert!(
        elapsed < Duration::from_secs(9),
        "the stalled write should time out at ~2s (web_server_response_timeout_seconds), \
         took {elapsed:?}"
    );

    drop(sock); // keep the client connected until the assertion above
    match tokio::task::spawn_blocking(move || server.join()).await {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(e) => panic!("server join task failed: {e}"),
    }
}

#[tokio::test]
async fn test_early_chunk_is_visible_before_the_body_completes() {
    let port = common::free_tcp_port();

    // Handler sends an early chunk + flush, WAITS 2s, then sends a late chunk and
    // closes. A streaming client must SEE the early chunk well before the late one —
    // proving head/first-chunk visibility before body completion, not buffer-to-end.
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop:
            wait for request comes in on srv as req with timeout 30000
            start streaming response to req with status 200 and content type "text/plain" as out
            write chunk "EARLY" to out
            flush out
            wait for 2000 milliseconds
            write chunk "LATE" to out
            close out
            break
        end loop
    "#
    );

    let server = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("server runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let program = Parser::new(&tokens).parse().expect("parse");
            let mut interp = Interpreter::new();
            let _ = interp.interpret(&program).await;
        });
    });

    wait_for_server(port).await;

    let mut resp = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/"))
        .send()
        .await
        .expect("request send");
    assert_eq!(resp.status().as_u16(), 200);

    let start = Instant::now();
    let mut early_at = None;
    let mut late_at = None;
    let mut acc = String::new();
    loop {
        match tokio::time::timeout(Duration::from_secs(6), resp.chunk()).await {
            Ok(Ok(Some(bytes))) => {
                acc.push_str(&String::from_utf8_lossy(&bytes));
                if early_at.is_none() && acc.contains("EARLY") {
                    early_at = Some(start.elapsed());
                }
                if late_at.is_none() && acc.contains("LATE") {
                    late_at = Some(start.elapsed());
                }
            }
            Ok(Ok(None)) | Ok(Err(_)) => break,
            Err(_) => panic!("streaming body stalled"),
        }
    }

    let early = early_at.expect("the EARLY chunk was never received");
    let late = late_at.expect("the LATE chunk was never received");
    // The early chunk must arrive well before the late one — proving it was flushed
    // to the wire while the body was still open, not buffered until close.
    assert!(
        early < Duration::from_millis(1500),
        "the EARLY chunk should be visible almost immediately, arrived at {early:?}"
    );
    assert!(
        late - early > Duration::from_millis(1000),
        "the LATE chunk should arrive ~2s after EARLY (early={early:?}, late={late:?}); \
         a small gap means the body was buffered to completion instead of streamed"
    );

    match tokio::task::spawn_blocking(move || server.join()).await {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(e) => panic!("server join task failed: {e}"),
    }
}
