//! Real-socket regression (maintainer re-review, P1): `outbound_stream_max_seconds`
//! must be a TRUE absolute lifetime enforced in real time — even when the handler
//! NEVER reads the opened stream.
//!
//! The deadline was previously consulted only on the next read, so a handler that
//! opened an outbound stream and then parked (or did other work) without reading
//! kept the upstream connection alive indefinitely past the cap — contradicting the
//! documented "can never live past this hard cap". This proves the upstream is
//! dropped at the cap with NO read performed, well before the program ends.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wfl::Interpreter;
use wfl::config::WflConfig;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Upstream: send a valid response head, then STALL (never send more, never close).
/// Signal on the returned receiver when the proxy drops the upstream connection.
async fn spawn_head_then_stall_upstream() -> (u16, tokio::sync::oneshot::Receiver<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock upstream");
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        if let Ok((mut sock, _)) = listener.accept().await {
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf).await; // request head
            let head = "HTTP/1.1 200 OK\r\n\
                        Content-Type: text/plain\r\n\
                        Transfer-Encoding: chunked\r\n\r\n";
            let _ = sock.write_all(head.as_bytes()).await;
            let _ = sock.flush().await;
            // Stall: never send a body, never close. Detect the proxy dropping the
            // upstream (its handle reaped) via a blocking read returning 0/Err.
            loop {
                match sock.read(&mut buf).await {
                    Ok(0) | Err(_) => {
                        let _ = tx.send(());
                        return;
                    }
                    Ok(_) => {}
                }
            }
        }
    });
    (port, rx)
}

#[tokio::test]
async fn test_opened_but_unread_stream_expires_at_the_absolute_cap() {
    let (port, mut upstream_closed) = spawn_head_then_stall_upstream().await;

    // Open the stream and then just WAIT — never read a chunk/line. With a 1s
    // absolute cap the upstream must be dropped ~1s in, long before the 6s wait
    // (and the program-end cleanup that would otherwise mask a missing reaper).
    let code = format!(
        r#"open url at "http://127.0.0.1:{port}/" and stream response as s
wait for 6000 milliseconds"#
    );

    let client = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("client runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let program = Parser::new(&tokens).parse().expect("parse");
            let config = WflConfig {
                timeout_seconds: 30,
                outbound_stream_max_seconds: 1,
                ..WflConfig::default()
            };
            let mut interp = Interpreter::with_config(Arc::new(config));
            let _ = interp.interpret(&program).await;
        });
    });

    let start = Instant::now();
    tokio::time::timeout(Duration::from_secs(4), &mut upstream_closed)
        .await
        .expect("the opened-but-unread upstream was not dropped at the absolute cap")
        .expect("upstream close sender dropped");
    let elapsed = start.elapsed();

    // ~1s (the cap). If enforcement were still read-triggered, the upstream would
    // only close at the 6s program end — so a close well before then proves the
    // real-time reaper fired.
    assert!(
        elapsed < Duration::from_secs(3),
        "the upstream should be reaped at the ~1s absolute cap, not at program end; \
         took {elapsed:?}"
    );

    match tokio::task::spawn_blocking(move || client.join()).await {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(e) => panic!("client join task failed: {e}"),
    }
}
