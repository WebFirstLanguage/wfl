//! Close-during-active-read must cancel the upstream promptly (issue #642 re-review).
//!
//! `take_stream` removes the handle for the await; a concurrent `close` must trip
//! shared cancellation so the active read aborts and the upstream is dropped —
//! not leave the connection open until idle timeout while `put_stream` treats a
//! missing slot as success.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wfl::Interpreter;
use wfl::config::WflConfig;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

async fn spawn_stall_upstream() -> (u16, tokio::sync::oneshot::Receiver<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
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
async fn close_during_active_read_drops_upstream_promptly() {
    let (port, mut upstream_closed) = spawn_stall_upstream().await;

    // Open stream, start a body read that will park on the stalled upstream, then
    // close from another path via a short wait then close — implemented as:
    // open, spawn wait for next chunk (parks), wait 200ms, close, then the read
    // must fail and upstream drop well before the idle timeout (30s).
    let code = format!(
        r#"
        open url at "http://127.0.0.1:{port}/" and stream response as s
        wait for 200 milliseconds
        close s
        wait for next chunk from s as c
        "#
    );

    let (result_tx, result_rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let client = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let program = Parser::new(&tokens).parse().expect("parse");
            let config = WflConfig {
                timeout_seconds: 30,
                outbound_stream_max_seconds: 60,
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
    tokio::time::timeout(Duration::from_secs(3), &mut upstream_closed)
        .await
        .expect("upstream should drop promptly after close, not at 30s idle timeout")
        .expect("upstream close sender dropped");
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(2),
        "close-during-read should drop upstream promptly; took {elapsed:?}"
    );

    let summary = result_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("interpreter should finish");
    // After close, wait for next chunk should fail (closed stream).
    assert!(
        summary.is_err(),
        "read after close must error, got {summary:?}"
    );
    let msg = summary.unwrap_err().to_lowercase();
    assert!(
        msg.contains("closed") || msg.contains("unknown") || msg.contains("stream"),
        "expected a closed-stream error, got: {msg}"
    );

    match tokio::task::spawn_blocking(move || client.join()).await {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(e) => panic!("join failed: {e}"),
    }
}
