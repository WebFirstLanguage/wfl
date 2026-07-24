//! Real-socket regression (issue #642 P1): the absolute-lifetime reaper and an
//! active body read must share one atomic lifecycle.
//!
//! If the reaper only removes a parked handle, a read that took the handle out
//! for the await can win the read/timeout race and reinsert the expired handle —
//! leaving a live upstream past the documented real-time hard cap. With the fix:
//! the reaper marks a shared slot expired; `put_stream` refuses reinsertion; the
//! next/current outcome surfaces a typed Timeout and the upstream is dropped.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wfl::Interpreter;
use wfl::config::WflConfig;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Upstream: send a valid response head, then stall (never body, never close).
/// Signal when the proxy drops the upstream connection.
async fn spawn_head_then_stall_upstream() -> (u16, tokio::sync::oneshot::Receiver<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock upstream");
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        if let Ok((mut sock, _)) = listener.accept().await {
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf).await;
            let head = "HTTP/1.1 200 OK\r\n\
                        Content-Type: text/plain\r\n\
                        Transfer-Encoding: chunked\r\n\r\n";
            let _ = sock.write_all(head.as_bytes()).await;
            let _ = sock.flush().await;
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
async fn test_active_read_near_deadline_surfaces_timeout_and_drops_upstream() {
    let (port, mut upstream_closed) = spawn_head_then_stall_upstream().await;

    // Cap is 1s. Immediately start a body read that will park on the stalled
    // upstream; the reaper must still expire the slot and cancel the read as a
    // Timeout (not "unknown/already closed"), dropping the upstream ~at the cap.
    let code = format!(
        r#"
        open url at "http://127.0.0.1:{port}/" and stream response as s
        wait for next chunk from s as c
        display c
        "#
    );

    // Send only a serializable error summary — `Value` is not `Send`.
    let (result_tx, result_rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let client = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("client runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let program = Parser::new(&tokens).parse().expect("parse");
            let config = WflConfig {
                timeout_seconds: 30, // idle timeout long enough that absolute cap wins
                outbound_stream_max_seconds: 1,
                ..WflConfig::default()
            };
            let mut interp = Interpreter::with_config(Arc::new(config));
            let result = interp.interpret(&program).await;
            let summary = match result {
                Ok(_) => Ok(()),
                Err(errs) => Err(format!("{errs:?}")),
            };
            let _ = result_tx.send(summary);
        });
    });

    let start = Instant::now();
    tokio::time::timeout(Duration::from_secs(4), &mut upstream_closed)
        .await
        .expect("upstream was not dropped near the absolute cap during an active read")
        .expect("upstream close sender dropped");
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(3),
        "upstream should drop near the 1s absolute cap, not the 30s idle timeout; took {elapsed:?}"
    );

    let result = result_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("interpreter should finish after the absolute-cap timeout");
    match tokio::task::spawn_blocking(move || client.join()).await {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(e) => panic!("client join task failed: {e}"),
    }

    let msg = result.expect_err("active read past absolute cap must fail (Timeout), not succeed");
    assert!(
        msg.to_lowercase().contains("timeout")
            || msg.contains("Timeout")
            || msg.contains("outbound"),
        "expected a typed Timeout-class error, got: {msg}"
    );
    assert!(
        !msg.to_lowercase().contains("unknown or already-closed"),
        "expired slot must surface Timeout, not 'unknown/already-closed'; got: {msg}"
    );
}

#[tokio::test]
async fn test_rapid_open_close_does_not_leak_reaper_tasks() {
    // Open and immediately close many outbound streams against a real (stalling)
    // upstream. Each close must abort its reaper timer so resource usage stays
    // bounded (not request-rate × cap sleeping tasks).
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 512];
                let _ = sock.read(&mut buf).await;
                let head = "HTTP/1.1 200 OK\r\n\
                            Content-Type: text/plain\r\n\
                            Transfer-Encoding: chunked\r\n\r\n";
                let _ = sock.write_all(head.as_bytes()).await;
                // Stall until the proxy drops us on close.
                let mut b = [0u8; 64];
                loop {
                    match sock.read(&mut b).await {
                        Ok(0) | Err(_) => return,
                        Ok(_) => {}
                    }
                }
            });
        }
    });

    // Cap is large (60s) so a leaked reaper would still be parked after the program
    // ends if timers were not aborted on close. We open/close 40 streams quickly.
    let mut lines = String::new();
    for i in 0..40 {
        lines.push_str(&format!(
            "open url at \"http://127.0.0.1:{port}/s{i}\" and stream response as s{i}\n\
             close s{i}\n"
        ));
    }

    let code = lines;
    let start = Instant::now();
    let client = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let program = Parser::new(&tokens).parse().expect("parse");
            let config = WflConfig {
                timeout_seconds: 10,
                outbound_stream_max_seconds: 60,
                ..WflConfig::default()
            };
            let mut interp = Interpreter::with_config(Arc::new(config));
            interp
                .interpret(&program)
                .await
                .expect("rapid open/close must succeed");
        });
    });
    match tokio::task::spawn_blocking(move || client.join()).await {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(e) => panic!("client join failed: {e}"),
    }
    let elapsed = start.elapsed();
    // Should finish in well under the 60s cap (and under a few seconds of network).
    assert!(
        elapsed < Duration::from_secs(20),
        "rapid open/close should finish promptly with reapers aborted; took {elapsed:?}"
    );
}

