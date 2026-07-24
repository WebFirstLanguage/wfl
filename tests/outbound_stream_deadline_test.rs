//! Real-socket regression for P1: `outbound_stream_max_seconds` must bound an
//! ACTIVE body read, not be overridden by the broader run/budget duration.
//!
//! A mock upstream sends the response head immediately and then stalls (never
//! sends a body chunk). With `timeout_seconds = 10` but
//! `outbound_stream_max_seconds = 1`, a `wait for next chunk` must fail in about
//! one second (the absolute stream deadline), not wait out the ten-second run
//! timeout. Before the fix, `run_http_with_budget` derived its timeout purely
//! from the budget/run duration and discarded the stream's shorter
//! `min(idle, remaining_total)`, so the read waited ~10s.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wfl::Interpreter;
use wfl::config::WflConfig;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Bind an ephemeral port and serve exactly one connection: read the request,
/// send a chunked-encoding response head, then hold the socket open WITHOUT
/// sending any body chunk (a head-then-stall upstream).
async fn spawn_head_then_stall_upstream() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock upstream");
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        if let Ok((mut sock, _)) = listener.accept().await {
            // Consume the request head so the client's write completes.
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf).await;
            let head = "HTTP/1.1 200 OK\r\n\
                        Content-Type: text/plain\r\n\
                        Transfer-Encoding: chunked\r\n\r\n";
            let _ = sock.write_all(head.as_bytes()).await;
            let _ = sock.flush().await;
            // Stall: keep the connection open but never send a body chunk.
            tokio::time::sleep(Duration::from_secs(30)).await;
            drop(sock);
        }
    });
    port
}

#[tokio::test]
async fn test_outbound_stream_absolute_deadline_bounds_active_read() {
    let port = spawn_head_then_stall_upstream().await;

    let code = format!(
        r#"open url at "http://127.0.0.1:{port}/" and stream response as s
wait for next chunk from s as c"#
    );
    let tokens = lex_wfl_with_positions(&code);
    let program = Parser::new(&tokens).parse().expect("parse");

    // Run/idle timeout 10s, absolute stream lifetime 1s.
    let mut config = WflConfig::default();
    config.timeout_seconds = 10;
    config.outbound_stream_max_seconds = 1;
    let mut interp = Interpreter::with_config(Arc::new(config));

    let start = Instant::now();
    let result = interp.interpret(&program).await;
    let elapsed = start.elapsed();

    assert!(
        result.is_err(),
        "a stalled stream read must fail, not hang or succeed"
    );
    assert!(
        elapsed < Duration::from_secs(4),
        "`wait for next chunk` must fail near the 1s absolute stream deadline, \
         not the 10s run timeout (took {elapsed:?})"
    );
    // And it must not fail instantly either — the head arrived and the 1s clock
    // had to elapse.
    assert!(
        elapsed >= Duration::from_millis(500),
        "the read failed too early to be the ~1s absolute deadline (took {elapsed:?})"
    );
}
