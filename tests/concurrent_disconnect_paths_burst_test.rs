//! Real-socket regression (maintainer re-review, P1 / #642): EVERY transport-confirmed
//! client disconnect must be classified as a cancellation, not a handler failure —
//! including the buffered `respond` send, the streaming-response head path (before the
//! head is sent), and the streaming write path — not only a cancelled upstream chunk
//! read.
//!
//! The concurrent loop breaks after `MAX_CONSECUTIVE_FAILURES` (256) consecutive
//! *structural* failures. Request-local outcomes (disconnects, wait timeouts, errors
//! after a request was accepted) must never feed that breaker. These bursts drive
//! more than 256 disconnects of each kind with no successful request in between; an
//! unrelated `/ping` must still be served afterward.
//!
//! Also: every client that is intended to exercise a path must actually connect and
//! reach that lifecycle point (no silent early-return that leaves the burst under the
//! breaker threshold), and the test waits long enough for every handler result to be
//! consumed before probing `/ping` (so a General-classified disconnect cannot race
//! past a premature success that resets the counter).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Semaphore;
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

mod common;

/// More than the breaker threshold (256), with NO successful request in between.
const DISCONNECT_BURST: usize = 270;
const CLIENT_CONCURRENCY: usize = 40;
/// Upper bound on how long the General-failure backoff would take to consume 256
/// failures (~11.5s) plus handler work. Waiting past that guarantees the counter
/// would have tripped if any of the disconnects were misclassified as structural.
const DRAIN_AFTER_BURST: Duration = Duration::from_secs(15);

fn start_proxy_server(code: String) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let ast = Parser::new(&tokens).parse().expect("parse");
            let mut interp = Interpreter::new();
            if let Err(errors) = interp.interpret(&ast).await {
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
    panic!("server on {addr} did not become ready");
}

/// Connect, send the request so the server enqueues and dequeues it, briefly hold so
/// the handler is inside its pre-reply work, then disconnect.
///
/// - `read_head == false`: disconnect after a short hold so the disconnect lands
///   before `respond` / before the streaming head is sent.
/// - `read_head == true`: wait for the streaming response head first, so the
///   disconnect lands after the head, at the write path.
///
/// Returns whether the client successfully connected and sent the request (so the
/// burst can assert every intended disconnect actually reached the server).
async fn fire_disconnect(port: u16, path: &str, read_head: bool) -> bool {
    let Ok(mut sock) = tokio::net::TcpStream::connect(("127.0.0.1", port)).await else {
        return false;
    };
    let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    if sock.write_all(req.as_bytes()).await.is_err() {
        return false;
    }
    if sock.flush().await.is_err() {
        return false;
    }
    if read_head {
        let mut acc = Vec::new();
        let mut tmp = [0u8; 256];
        let mut saw_head = false;
        loop {
            match tokio::time::timeout(Duration::from_secs(5), sock.read(&mut tmp)).await {
                Ok(Ok(0)) | Err(_) | Ok(Err(_)) => break,
                Ok(Ok(n)) => {
                    acc.extend_from_slice(&tmp[..n]);
                    if acc.windows(4).any(|w| w == b"\r\n\r\n") {
                        saw_head = true;
                        break;
                    }
                }
            }
        }
        if !saw_head {
            return false;
        }
    } else {
        // Give the server time to enqueue + dequeue the request and enter the
        // handler's pre-reply wait, so the disconnect lands before `respond` /
        // before the streaming head.
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    // Drop `sock` -> disconnect.
    true
}

async fn fire_burst(port: u16, path: &'static str, read_head: bool) -> usize {
    let sem = Arc::new(Semaphore::new(CLIENT_CONCURRENCY));
    let connected = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::with_capacity(DISCONNECT_BURST);
    for _ in 0..DISCONNECT_BURST {
        let sem = Arc::clone(&sem);
        let connected = Arc::clone(&connected);
        tasks.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore");
            if fire_disconnect(port, path, read_head).await {
                connected.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }
    for t in tasks {
        let _ = t.await;
    }
    let n = connected.load(Ordering::Relaxed);
    assert!(
        n > 256,
        "expected more than 256 clients to actually connect and reach the intended \
         lifecycle point (so the burst exceeds the structural breaker threshold); \
         only {n} of {DISCONNECT_BURST} succeeded (path={path}, read_head={read_head})"
    );
    // Drain past the General-failure backoff window so every intended handler
    // result is consumed before `/ping`. If any disconnect were still classified
    // as structural General, the breaker would trip during this wait.
    tokio::time::sleep(DRAIN_AFTER_BURST).await;
    n
}

async fn assert_ping_survives(port: u16, context: &str) {
    let ping = tokio::time::timeout(
        Duration::from_secs(10),
        reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/ping"))
            .send(),
    )
    .await
    .unwrap_or_else(|_| panic!("`/ping` timed out after the {context} burst — loop torn down"))
    .expect("`/ping` request failed");
    assert_eq!(
        ping.status().as_u16(),
        200,
        "`/ping` should be served after the {context} burst"
    );
    assert_eq!(ping.text().await.unwrap(), "pong");
}

async fn shutdown(port: u16, server: std::thread::JoinHandle<()>) {
    let _ = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/shutdown"))
        .send()
        .await;
    match tokio::task::spawn_blocking(move || server.join()).await {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(e) => panic!("server join task failed: {e}"),
    }
}

#[tokio::test]
async fn test_disconnect_before_buffered_respond_does_not_kill_the_loop() {
    let port = common::free_tcp_port();
    // `/slow` waits, then responds — the client disconnects during the wait, so the
    // buffered `respond` send fails (or the pending entry is sibling-pruned). That
    // must be a cancellation, not a structural failure.
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop concurrently:
            wait for request comes in on srv as req with timeout 30000
            store p as req["path"]
            check if p is equal to "/ping":
                respond to req with "pong"
            otherwise:
                check if p is equal to "/shutdown":
                    respond to req with "bye"
                    close server srv
                    break
                otherwise:
                    wait for 500 milliseconds
                    respond to req with "late"
                end check
            end check
        end loop
    "#
    );
    let server = start_proxy_server(code);
    wait_for_server(port).await;
    let _ = fire_burst(port, "/slow", false).await;
    assert_ping_survives(port, "buffered-respond disconnect").await;
    shutdown(port, server).await;
}

#[tokio::test]
async fn test_disconnect_before_stream_write_does_not_kill_the_loop() {
    let port = common::free_tcp_port();
    // `/stream` sends the head, waits (the client reads the head then disconnects),
    // then writes — the write send fails. That must be a cancellation, not a failure.
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop concurrently:
            wait for request comes in on srv as req with timeout 30000
            store p as req["path"]
            check if p is equal to "/ping":
                respond to req with "pong"
            otherwise:
                check if p is equal to "/shutdown":
                    respond to req with "bye"
                    close server srv
                    break
                otherwise:
                    start streaming response to req with status 200 and content type "text/plain" as out
                    wait for 300 milliseconds
                    store payload as "0123456789"
                    count from 1 to 9:
                        store payload as payload with payload
                    end count
                    count from 1 to 400:
                        write chunk payload to out
                    end count
                    close out
                end check
            end check
        end loop
    "#
    );
    let server = start_proxy_server(code);
    wait_for_server(port).await;
    let _ = fire_burst(port, "/stream", true).await;
    assert_ping_survives(port, "stream-write disconnect").await;
    shutdown(port, server).await;
}

#[tokio::test]
async fn test_disconnect_before_streaming_head_does_not_kill_the_loop() {
    let port = common::free_tcp_port();
    // Client disconnects *before* the streaming head is sent (no head read). The
    // handler parks, then reaches `start streaming response` with a missing/closed
    // pending entry — must be Cancelled, not a structural General that trips the
    // breaker after >256 instances.
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop concurrently:
            wait for request comes in on srv as req with timeout 30000
            store p as req["path"]
            check if p is equal to "/ping":
                respond to req with "pong"
            otherwise:
                check if p is equal to "/shutdown":
                    respond to req with "bye"
                    close server srv
                    break
                otherwise:
                    wait for 500 milliseconds
                    start streaming response to req with status 200 and content type "text/plain" as out
                    write line "late" to out
                    close out
                end check
            end check
        end loop
    "#
    );
    let server = start_proxy_server(code);
    wait_for_server(port).await;
    let _ = fire_burst(port, "/prehead", false).await;
    assert_ping_survives(port, "pre-streaming-head disconnect").await;
    shutdown(port, server).await;
}

#[tokio::test]
async fn test_repeated_wait_timeouts_do_not_kill_the_loop() {
    let port = common::free_tcp_port();
    // Finite `wait for request ... with timeout` that repeatedly expires with no
    // client traffic must not trip the structural breaker. After many idle
    // timeouts a real `/ping` must still be served.
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop concurrently:
            wait for request comes in on srv as req with timeout 50
            store p as req["path"]
            check if p is equal to "/ping":
                respond to req with "pong"
            otherwise:
                check if p is equal to "/shutdown":
                    respond to req with "bye"
                    close server srv
                    break
                otherwise:
                    respond to req with "ok"
                end check
            end check
        end loop
    "#
    );
    let server = start_proxy_server(code);
    wait_for_server(port).await;
    // Idle long enough for well over 256 consecutive wait timeouts (50ms each;
    // concurrency multiplies the rate). 8s >> 256 * structural backoff would
    // also have completed if they were misclassified.
    tokio::time::sleep(Duration::from_secs(8)).await;
    assert_ping_survives(port, "repeated wait timeouts").await;
    shutdown(port, server).await;
}
