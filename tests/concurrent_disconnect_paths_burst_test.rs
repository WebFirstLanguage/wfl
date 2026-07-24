//! Real-socket regression (maintainer re-review, P1): EVERY transport-confirmed
//! client disconnect must be classified as a cancellation, not a handler failure —
//! including the buffered `respond` send and the streaming-response head/`write`
//! paths, not only a cancelled upstream chunk read.
//!
//! The concurrent loop breaks after `MAX_CONSECUTIVE_FAILURES` (256) consecutive
//! failed handlers. If a disconnect at these paths returns a General runtime error
//! (as before), a burst of >256 disconnects trips that breaker and the server stops
//! serving — turning "the client hung up" into a denial of service. These bursts
//! disconnect after dequeue (before the buffered reply) and after the streaming head
//! (before/at the first write); an unrelated `/ping` must still be served afterward.

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
/// the handler is inside its pre-reply work, then disconnect. `read_head` waits for
/// the streaming response head first (so the disconnect lands after the head, at the
/// write path) when the route streams.
async fn fire_disconnect(port: u16, path: &str, read_head: bool) {
    let Ok(mut sock) = tokio::net::TcpStream::connect(("127.0.0.1", port)).await else {
        return;
    };
    let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    if sock.write_all(req.as_bytes()).await.is_err() {
        return;
    }
    let _ = sock.flush().await;
    if read_head {
        let mut acc = Vec::new();
        let mut tmp = [0u8; 256];
        loop {
            match tokio::time::timeout(Duration::from_secs(5), sock.read(&mut tmp)).await {
                Ok(Ok(0)) | Err(_) | Ok(Err(_)) => break,
                Ok(Ok(n)) => {
                    acc.extend_from_slice(&tmp[..n]);
                    if acc.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
            }
        }
    } else {
        // Give the server time to enqueue + dequeue the request and enter the
        // handler's pre-reply wait, so the disconnect lands before `respond`.
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    // Drop `sock` -> disconnect.
}

async fn fire_burst(port: u16, path: &'static str, read_head: bool) {
    let sem = Arc::new(Semaphore::new(CLIENT_CONCURRENCY));
    let fired = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::with_capacity(DISCONNECT_BURST);
    for _ in 0..DISCONNECT_BURST {
        let sem = Arc::clone(&sem);
        let fired = Arc::clone(&fired);
        tasks.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore");
            fire_disconnect(port, path, read_head).await;
            fired.fetch_add(1, Ordering::Relaxed);
        }));
    }
    for t in tasks {
        let _ = t.await;
    }
    // Grace so all >256 handlers finish failing (Cancelled, under the fix) before we
    // send the first successful request — guaranteeing the failures are consecutive.
    tokio::time::sleep(Duration::from_secs(3)).await;
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
    // buffered `respond` send fails. That must be a cancellation, not a failure.
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
    fire_burst(port, "/slow", false).await;
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
    fire_burst(port, "/stream", true).await;
    assert_ping_survives(port, "stream-write disconnect").await;
    shutdown(port, server).await;
}
