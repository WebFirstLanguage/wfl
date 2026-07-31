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
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wfl::Interpreter;
use wfl::config::WflConfig;
use wfl::interpreter::error::ErrorKind;
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

async fn read_response_head(sock: &mut tokio::net::TcpStream) -> Vec<u8> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut head = Vec::new();
    let mut buf = [0u8; 512];
    loop {
        let n = tokio::time::timeout_at(deadline, sock.read(&mut buf))
            .await
            .expect("timed out waiting for streaming response head")
            .expect("failed to read streaming response head");
        assert_ne!(n, 0, "connection closed before the complete response head");
        head.extend_from_slice(&buf[..n]);
        if head.windows(4).any(|window| window == b"\r\n\r\n") {
            return head;
        }
        assert!(
            head.len() <= 16 * 1024,
            "streaming response head exceeded 16 KiB"
        );
    }
}

async fn join_server(server: std::thread::JoinHandle<()>) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !server.is_finished() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("server thread did not stop within 10 seconds");
    if let Err(panic) = server.join() {
        std::panic::resume_unwind(panic);
    }
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

    // Reduce the result to Send error fields before crossing the server-thread
    // boundary. Keeping the typed kind separate prevents a broad text assertion
    // from accepting an unrelated cancellation or pre-head failure.
    let (done_tx, done_rx) =
        tokio::sync::oneshot::channel::<(Duration, Result<(), Vec<(ErrorKind, String)>>)>();
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
            let result = interp.interpret(&program).await;
            let summary = match result {
                Ok(_) => Ok(()),
                Err(errs) => Err(errs
                    .into_iter()
                    .map(|error| (error.kind, error.message))
                    .collect()),
            };
            let _ = done_tx.send((start.elapsed(), summary));
        });
    });

    wait_for_server(port).await;

    // Connect and confirm the 200 streaming head first. Only then stop reading and
    // hold the socket open. This excludes a pending-request 504, a pre-head
    // cancellation, or any other early exit from false-greening the stall test.
    let mut sock = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    sock.write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .await
        .expect("send request");
    sock.flush().await.expect("flush request");
    let head = read_response_head(&mut sock).await;
    let head_text = String::from_utf8_lossy(&head);
    assert!(
        head_text.starts_with("HTTP/1.1 200"),
        "expected a successful streaming response head, got {head_text:?}"
    );
    assert!(
        head_text
            .to_ascii_lowercase()
            .contains("content-type: text/plain"),
        "expected the streaming content type in the response head, got {head_text:?}"
    );
    // `interpret()` must return once the stalled write times out (~2s). If the write
    // were unbounded it would pin the handler and this never fires.
    let (elapsed, summary) = tokio::time::timeout(Duration::from_secs(12), done_rx)
        .await
        .expect("interpret() never returned — the backpressured write pinned the handler forever")
        .expect("done sender dropped");
    // Exact typed contract: only the bounded backpressure branch is acceptable.
    // A Cancelled disconnect, a generic write error, or any other Timeout is not.
    let errors = summary.expect_err(
        "backpressured write to a non-reading client must error (write timeout), not succeed",
    );
    assert_eq!(
        errors.len(),
        1,
        "expected exactly one backpressure error, got {errors:?}"
    );
    assert_eq!(
        errors[0].0,
        ErrorKind::Timeout,
        "backpressured connected client must produce ErrorKind::Timeout, got {errors:?}"
    );
    assert_eq!(
        errors[0].1,
        "Cannot write to response stream: the client stopped reading (write timed out)",
        "backpressure must report the exact stalled-client diagnostic"
    );
    // The exact kind/message above identifies the backpressure timeout branch.
    // Measure its lower bound from interpreter start: measuring from when the test
    // thread happens to observe the head can false-fail if the server had already
    // started the write timer before that observation.
    assert!(
        elapsed >= Duration::from_millis(1500),
        "the configured 2s write timeout fired implausibly early; took only {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(9),
        "the stalled write should time out at ~2s (web_server_response_timeout_seconds), \
         took {elapsed:?}"
    );

    drop(sock); // keep the client connected until the assertion above
    join_server(server).await;
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

    let (done_tx, done_rx) =
        tokio::sync::oneshot::channel::<Result<(), Vec<(ErrorKind, String)>>>();
    let server = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("server runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let program = Parser::new(&tokens).parse().expect("parse");
            let mut interp = Interpreter::new();
            let result = match interp.interpret(&program).await {
                Ok(_) => Ok(()),
                Err(errors) => Err(errors
                    .into_iter()
                    .map(|error| (error.kind, error.message))
                    .collect()),
            };
            let _ = done_tx.send(result);
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
    tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            match resp.chunk().await {
                Ok(Some(bytes)) => {
                    acc.push_str(&String::from_utf8_lossy(&bytes));
                    if early_at.is_none() && acc.contains("EARLY") {
                        early_at = Some(start.elapsed());
                    }
                    if late_at.is_none() && acc.contains("LATE") {
                        late_at = Some(start.elapsed());
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    panic!("stream transport failed instead of ending cleanly: {error}")
                }
            }
        }
    })
    .await
    .expect("streaming body stalled");

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

    let interpreter_result = tokio::time::timeout(Duration::from_secs(3), done_rx)
        .await
        .expect("interpreter did not finish after the streaming body closed")
        .expect("interpreter result sender dropped");
    interpreter_result.unwrap_or_else(|errors| {
        panic!("streaming visibility program must finish successfully, got {errors:?}")
    });
    join_server(server).await;
}

/// #680: when a program's last act is `close out` then exit, the client must
/// still receive a complete body (including the terminating chunk). Dropping the
/// sender alone is not enough — the interpreter must wait for the transport to
/// finish the body before returning, otherwise runtime teardown aborts the
/// connection mid-flush and the client reports a decode/truncation error.
#[tokio::test]
async fn test_body_is_complete_when_program_ends_immediately_after_close() {
    let port = common::free_tcp_port();

    // No post-close wait, no `close server` — the program finishes the moment
    // the handler breaks. This is the product path that used to race: close
    // drops the sender, interpret returns, the host drops the Runtime, and the
    // in-flight connection task is aborted before the terminating chunk lands.
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop:
            wait for request comes in on srv as req with timeout 30000
            start streaming response to req with status 200 and content type "text/plain" as out
            write chunk "COMPLETE" to out
            close out
            break
        end loop
    "#
    );

    let (done_tx, done_rx) =
        tokio::sync::oneshot::channel::<Result<(), Vec<(ErrorKind, String)>>>();
    let server = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("server runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let program = Parser::new(&tokens).parse().expect("parse");
            let mut interp = Interpreter::new();
            let result = match interp.interpret(&program).await {
                Ok(_) => Ok(()),
                Err(errors) => Err(errors
                    .into_iter()
                    .map(|error| (error.kind, error.message))
                    .collect()),
            };
            let _ = done_tx.send(result);
        });
    });

    wait_for_server(port).await;

    let mut resp = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/"))
        .send()
        .await
        .expect("request send");
    assert_eq!(resp.status().as_u16(), 200);

    let mut body = String::new();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match resp.chunk().await {
                Ok(Some(bytes)) => body.push_str(&String::from_utf8_lossy(&bytes)),
                Ok(None) => break,
                Err(error) => {
                    panic!(
                        "stream transport failed instead of ending cleanly after close out: {error}"
                    )
                }
            }
        }
    })
    .await
    .expect("streaming body stalled after close out");

    assert_eq!(
        body, "COMPLETE",
        "client must receive the full body when the program ends immediately after close out"
    );

    let interpreter_result = tokio::time::timeout(Duration::from_secs(3), done_rx)
        .await
        .expect("interpreter did not finish after close out")
        .expect("interpreter result sender dropped");
    interpreter_result.unwrap_or_else(|errors| {
        panic!("close-out exit program must finish successfully, got {errors:?}")
    });
    join_server(server).await;
}

/// Concurrent counterpart to #680: a `main loop concurrently:` handler that
/// streams without an explicit `close out` and then `break`s must still deliver
/// a complete body. Stream ids live on the handler's isolated RunState, so the
/// top-level serial drain never sees them — finish must be awaited before the
/// IsolatedHandler reports Ready, not only fire-and-forget in Drop.
#[tokio::test]
async fn test_concurrent_handler_body_is_complete_without_explicit_close_on_break() {
    let port = common::free_tcp_port();

    let code = format!(
        r#"
        listen on port {port} as srv
        main loop concurrently:
            wait for request comes in on srv as req with timeout 30000
            start streaming response to req with status 200 and content type "text/plain" as out
            write chunk "COMPLETE" to out
            break
        end loop
    "#
    );

    let (done_tx, done_rx) =
        tokio::sync::oneshot::channel::<Result<(), Vec<(ErrorKind, String)>>>();
    let server = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("server runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let program = Parser::new(&tokens).parse().expect("parse");
            let mut interp = Interpreter::new();
            let result = match interp.interpret(&program).await {
                Ok(_) => Ok(()),
                Err(errors) => Err(errors
                    .into_iter()
                    .map(|error| (error.kind, error.message))
                    .collect()),
            };
            let _ = done_tx.send(result);
        });
    });

    wait_for_server(port).await;

    let mut resp = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/"))
        .send()
        .await
        .expect("request send");
    assert_eq!(resp.status().as_u16(), 200);

    let mut body = String::new();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match resp.chunk().await {
                Ok(Some(bytes)) => body.push_str(&String::from_utf8_lossy(&bytes)),
                Ok(None) => break,
                Err(error) => {
                    panic!("concurrent stream transport failed instead of ending cleanly: {error}")
                }
            }
        }
    })
    .await
    .expect("concurrent streaming body stalled");

    assert_eq!(
        body, "COMPLETE",
        "concurrent handler must deliver the full body when it breaks without close out"
    );

    let interpreter_result = tokio::time::timeout(Duration::from_secs(3), done_rx)
        .await
        .expect("interpreter did not finish after concurrent break")
        .expect("interpreter result sender dropped");
    interpreter_result.unwrap_or_else(|errors| {
        panic!("concurrent stream-break program must finish successfully, got {errors:?}")
    });
    join_server(server).await;
}
