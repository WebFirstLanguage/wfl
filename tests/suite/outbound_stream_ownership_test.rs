//! Real-socket regression for P1: an outbound stream is handler-OWNED — when the
//! run/handler ends with the stream still open (no explicit `close`), the handle
//! is dropped, cancelling the in-flight upstream request. Otherwise an abandoned
//! proxy read leaks the upstream connection until the whole interpreter tears
//! down.
//!
//! The mock upstream streams one chunk and then keeps trying to write; when the
//! client drops the connection its write fails and it signals a oneshot. The WFL
//! program reads one chunk and ends WITHOUT `close`, while the interpreter is
//! still alive — so a leak would keep the connection open and the test times out.

use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wfl::Interpreter;
use wfl::config::WflConfig;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// One-connection upstream: send a chunked head + one body chunk, then keep
/// writing keepalive chunks. When the client has disconnected, a write fails and
/// we signal via the oneshot.
async fn spawn_streaming_upstream() -> (u16, tokio::sync::oneshot::Receiver<()>) {
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
            let _ = sock.write_all(b"5\r\nhello\r\n").await; // one chunk: "hello"
            let _ = sock.flush().await;

            // Keep sending; the first failed write means the client dropped.
            for _ in 0..100 {
                tokio::time::sleep(Duration::from_millis(100)).await;
                if sock.write_all(b"1\r\nx\r\n").await.is_err() || sock.flush().await.is_err() {
                    let _ = tx.send(());
                    return;
                }
            }
        }
    });
    (port, rx)
}

#[tokio::test]
async fn test_outbound_stream_closed_when_run_ends_without_close() {
    let (port, disconnect_rx) = spawn_streaming_upstream().await;

    // Reads one chunk, then the program ENDS without `close s`.
    let code = format!(
        r#"open url at "http://127.0.0.1:{port}/" and stream response as s
wait for next chunk from s as first"#
    );
    let tokens = lex_wfl_with_positions(&code);
    let program = Parser::new(&tokens).parse().expect("parse");

    let mut interp = Interpreter::with_config(Arc::new(WflConfig::default()));
    interp.interpret(&program).await.expect("interpret");

    // The interpreter is still alive here, so only handler-exit cleanup (not the
    // interpreter's own teardown) can have dropped the outbound handle. The mock
    // therefore only sees its client disconnect if that cleanup cancelled the
    // upstream — a leak would keep the connection open and this times out.
    tokio::time::timeout(Duration::from_secs(5), disconnect_rx)
        .await
        .expect("upstream not disconnected after the run ended — outbound handle leaked")
        .expect("disconnect sender dropped unexpectedly");

    drop(interp);
}
