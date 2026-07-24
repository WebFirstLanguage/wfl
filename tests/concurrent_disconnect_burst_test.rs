//! Real-socket regression for P1 (#2): a BURST of downstream (browser)
//! disconnects must NOT tear down the concurrent `main loop`.
//!
//! A client disconnect is an EXPECTED, normal cancellation of one handler — not a
//! handler *failure*. The concurrent loop keeps a single global consecutive-
//! failure counter that backs off after every failed handler and breaks the whole
//! loop once it reaches `MAX_CONSECUTIVE_FAILURES` (256). If each disconnect is
//! (mis)counted as a failure, then 256 disconnects with no interleaved success
//! trip that structural breaker and the server stops serving entirely — so an
//! ordinary "client hung up" event, repeated, becomes a denial of service.
//!
//! Topology: a stalling mock upstream <- WFL concurrent proxy -> many short-lived
//! clients. Each client makes the proxy open the (stalling) upstream, start a
//! streaming response, and block reading the upstream; the client then reads the
//! response head and disconnects, cancelling that handler. After a burst of >256
//! such disconnects (more than the breaker threshold, with NO successful request
//! in between), an unrelated `/ping` request must still be served — proving the
//! loop survived the burst.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Semaphore;
use tokio::sync::mpsc;
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// How many disconnecting clients to fire. Must exceed the concurrent loop's
/// `MAX_CONSECUTIVE_FAILURES` (256) so, under the buggy behavior, the burst trips
/// the structural breaker.
const DISCONNECT_BURST: usize = 270;
/// Wait for at least this many upstream closes (== handler disconnects) before
/// probing, guaranteeing the burst has driven the breaker past its threshold.
const CLOSES_BEFORE_PROBE: usize = 256;
/// Bounded client concurrency: well under the 256 handler cap and the request
/// queue bound, so no request is shed with 503.
const CLIENT_CONCURRENCY: usize = 48;

/// Mock upstream: for each connection, send a chunked head then STALL (send no
/// body). Signal on `closes` every time a connection is observed closing — which
/// only happens when the proxy handler cancels its upstream (client disconnect).
async fn spawn_counting_stall_upstream() -> (u16, mpsc::UnboundedReceiver<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock upstream");
    let port = listener.local_addr().unwrap().port();
    let (closes_tx, closes_rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let closes_tx = closes_tx.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = sock.read(&mut buf).await; // request head
                let head = "HTTP/1.1 200 OK\r\n\
                            Content-Type: text/plain\r\n\
                            Transfer-Encoding: chunked\r\n\r\n";
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.flush().await;
                // Stall: never send a body chunk. Block reading; when the proxy
                // cancels the upstream (its client disconnected), the peer close
                // surfaces here as Ok(0)/Err.
                loop {
                    match sock.read(&mut buf).await {
                        Ok(0) | Err(_) => {
                            let _ = closes_tx.send(());
                            return;
                        }
                        Ok(_) => {}
                    }
                }
            });
        }
    });
    (port, closes_rx)
}

fn start_proxy_server(code: String) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let ast = Parser::new(&tokens).parse().expect("parse");
            let mut interp = Interpreter::new();
            if let Err(errors) = interp.interpret(&ast).await {
                panic!("proxy interpreter failed: {errors:?}");
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
    panic!("proxy server on {addr} did not become ready");
}

/// One disconnecting client: open `/proxy`, read the response head (proving the
/// handler reached `start streaming response` and is now blocked on the upstream),
/// then drop the socket to disconnect.
async fn fire_disconnect(proxy_port: u16) {
    let Ok(mut sock) = tokio::net::TcpStream::connect(("127.0.0.1", proxy_port)).await else {
        return;
    };
    let req = "GET /proxy HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
    if sock.write_all(req.as_bytes()).await.is_err() {
        return;
    }
    // Read until the end of the response head (\r\n\r\n) so the handler has an
    // open response stream when we disconnect (that is what makes the disconnect
    // observable to the blocked upstream read).
    let mut acc = Vec::new();
    let mut tmp = [0u8; 256];
    loop {
        match tokio::time::timeout(Duration::from_secs(5), sock.read(&mut tmp)).await {
            Ok(Ok(0)) | Err(_) => break,
            Ok(Ok(n)) => {
                acc.extend_from_slice(&tmp[..n]);
                if acc.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            Ok(Err(_)) => break,
        }
    }
    // Drop `sock` -> disconnect while the handler is blocked reading the upstream.
}

#[tokio::test]
async fn test_disconnect_burst_does_not_kill_concurrent_loop() {
    let (upstream_port, mut upstream_closes) = spawn_counting_stall_upstream().await;

    let proxy_port = 8362;
    let code = format!(
        r#"
        listen on port {proxy_port} as srv
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
                    open url at "http://127.0.0.1:{upstream_port}/" and stream response as up
                    start streaming response to req with status 200 and content type "text/plain" as down
                    count from 1 to 100000:
                        wait for next chunk from up as c
                        check if c is nothing:
                            break
                        otherwise:
                            write chunk c to down
                        end check
                    end count
                end check
            end check
        end loop
    "#
    );
    let server = start_proxy_server(code);
    wait_for_server(proxy_port).await;

    // Fire a burst of disconnecting clients, bounded so none is shed with 503.
    let sem = Arc::new(Semaphore::new(CLIENT_CONCURRENCY));
    let fired = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::with_capacity(DISCONNECT_BURST);
    for _ in 0..DISCONNECT_BURST {
        let sem = Arc::clone(&sem);
        let fired = Arc::clone(&fired);
        tasks.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore");
            fire_disconnect(proxy_port).await;
            fired.fetch_add(1, Ordering::Relaxed);
        }));
    }

    // Wait until the mock has seen enough upstream closes to guarantee the burst
    // drove the buggy breaker past its 256-failure threshold (no `/ping` sent yet,
    // so every one of these is a "consecutive failure" under the old behavior).
    let mut closed = 0usize;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    while closed < CLOSES_BEFORE_PROBE {
        match tokio::time::timeout_at(deadline, upstream_closes.recv()).await {
            Ok(Some(())) => closed += 1,
            Ok(None) => break,
            Err(_) => panic!(
                "only observed {closed} upstream closes before timeout (expected {CLOSES_BEFORE_PROBE}); \
                 the disconnect burst did not fully drive the handlers"
            ),
        }
    }
    assert!(
        closed >= CLOSES_BEFORE_PROBE,
        "expected at least {CLOSES_BEFORE_PROBE} upstream closes, saw {closed}"
    );

    // Grace so the loop finishes counting the final failure (and, under the bug,
    // actually breaks) before we probe.
    tokio::time::sleep(Duration::from_millis(750)).await;

    // The unrelated `/ping` MUST still be served. Under the bug the loop has torn
    // itself down after 256 "failures" and this hangs / is refused.
    let ping = tokio::time::timeout(
        Duration::from_secs(10),
        reqwest::Client::new()
            .get(format!("http://127.0.0.1:{proxy_port}/ping"))
            .send(),
    )
    .await
    .expect("`/ping` timed out after the disconnect burst — concurrent loop was torn down")
    .expect("`/ping` request failed after the disconnect burst");
    assert_eq!(
        ping.status().as_u16(),
        200,
        "`/ping` should be served after the disconnect burst"
    );
    let body = ping.text().await.expect("read /ping body");
    assert_eq!(
        body, "pong",
        "`/ping` should return the live handler's response"
    );

    // Shut the server down and join everything.
    let _ = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{proxy_port}/shutdown"))
        .send()
        .await;
    for t in tasks {
        let _ = t.await;
    }
    match tokio::task::spawn_blocking(move || server.join()).await {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(e) => panic!("server join task failed: {e}"),
    }
}
