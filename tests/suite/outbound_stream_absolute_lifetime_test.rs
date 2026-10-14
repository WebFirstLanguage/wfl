//! Real-socket regression for P1 (#3): `outbound_stream_max_seconds` must be a
//! TRUE absolute lifetime — enforced even when a read is served from bytes that
//! were already buffered locally by an earlier read.
//!
//! `stream_pull` already fails a network read once the absolute deadline has
//! elapsed, so an empty-buffer read after the deadline errors correctly. But
//! `next_line`/`next_chunk` serve buffered bytes BEFORE consulting the deadline,
//! so a proxy that pulled a multi-line chunk could keep draining that buffer long
//! after the stream's absolute lifetime expired. This proves a buffered read
//! taken past the deadline now fails (and the upstream is dropped), instead of
//! succeeding.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wfl::Interpreter;
use wfl::config::WflConfig;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Upstream: send a chunked head + ONE body chunk framing two newline-terminated
/// lines ("line1\nline2\n"), then STALL (keep the socket open, send nothing more).
/// Signal on the returned receiver when the proxy drops the upstream connection.
async fn spawn_two_lines_then_stall_upstream() -> (u16, tokio::sync::oneshot::Receiver<()>) {
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
            // One chunk carrying both lines: 0xC = 12 bytes = "line1\nline2\n".
            let _ = sock.write_all(b"C\r\nline1\nline2\n\r\n").await;
            let _ = sock.flush().await;
            // Stall: never send more, never close. Detect the proxy dropping the
            // upstream (its handle expired) via a blocking read returning 0/Err.
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
async fn test_buffered_read_after_absolute_deadline_expires() {
    let (port, mut upstream_closed) = spawn_two_lines_then_stall_upstream().await;

    // Read the first line (which buffers "line2"), sleep past the 1s absolute
    // stream lifetime, then read again. The second line comes from the local
    // buffer — it must NOT be served after the absolute deadline.
    let code = format!(
        r#"open url at "http://127.0.0.1:{port}/" and stream response as s
wait for next line from s as a
wait for 1500 milliseconds
wait for next line from s as b"#
    );
    let tokens = lex_wfl_with_positions(&code);
    let program = Parser::new(&tokens).parse().expect("parse");

    // Idle/run timeout 10s; absolute stream lifetime 1s.
    let config = WflConfig {
        timeout_seconds: 10,
        outbound_stream_max_seconds: 1,
        ..WflConfig::default()
    };
    let mut interp = Interpreter::with_config(Arc::new(config));

    let start = Instant::now();
    let result = interp.interpret(&program).await;
    let elapsed = start.elapsed();

    assert!(
        result.is_err(),
        "a buffered `wait for next line` taken ~1.5s after opening (past the 1s \
         absolute stream lifetime) must fail, not be served from the local buffer"
    );
    // It must fail at the buffered read (~1.5s in), not hang out to the 10s
    // run timeout or the mock's 30s stall.
    assert!(
        elapsed < Duration::from_secs(5),
        "the expired buffered read should fail promptly (took {elapsed:?})"
    );

    // Expiring the handle must drop the upstream (cancel the request), so the
    // mock observes its connection close.
    tokio::time::timeout(Duration::from_secs(5), &mut upstream_closed)
        .await
        .expect("upstream was not closed after the stream's absolute lifetime expired")
        .expect("upstream close sender dropped");
}
