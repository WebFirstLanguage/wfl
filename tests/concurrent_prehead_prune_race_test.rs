//! Real-socket regression (maintainer re-review, P1): a sibling handler's
//! `wait for request` global prune must NOT erase a parked handler's pre-head
//! cancellation signal.
//!
//! Handler A blocks opening a header-stalled upstream BEFORE `start streaming
//! response`, so its only disconnect signal is its pending request's oneshot. When
//! A's client disconnects, A's pending entry becomes closed — but any later
//! `wait for request` prunes ALL closed entries. If a sibling prunes A's entry
//! before A's ~20ms poll notices, A's owned id is then simply absent from the map;
//! treating "absent" as "still connected" left A parked until its read timeout.
//!
//! This drives the race: a continuous stream of pruning `/kick` requests runs while
//! A's client disconnects, so a prune reliably removes A's closed entry in the poll
//! gap. With the fix (absent owned id == disconnected), A is cancelled promptly and
//! the upstream closes well before the idle timeout; without it, A hangs to timeout.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wfl::Interpreter;
use wfl::config::WflConfig;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

mod common;

/// Upstream: accept ONE connection, read the request, then WITHHOLD the response
/// head. Signal when the proxy drops the connection (peer close => read 0/Err).
async fn spawn_header_withholding_upstream() -> (u16, tokio::sync::oneshot::Receiver<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock upstream");
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        if let Ok((mut sock, _)) = listener.accept().await {
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf).await; // request head
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

#[tokio::test]
async fn test_sibling_prune_does_not_strand_a_pre_head_disconnect() {
    let (upstream_port, mut upstream_closed) = spawn_header_withholding_upstream().await;
    let proxy_port = common::free_tcp_port();

    let code = format!(
        r#"
        listen on port {proxy_port} as srv
        main loop concurrently:
            wait for request comes in on srv as req with timeout 30000
            store p as req["path"]
            check if p is equal to "/kick":
                respond to req with "ok"
            otherwise:
                check if p is equal to "/shutdown":
                    respond to req with "bye"
                    close server srv
                    break
                otherwise:
                    open url at "http://127.0.0.1:{upstream_port}/" and stream response as up
                    start streaming response to req with status 200 and content type "text/plain" as down
                    wait for next chunk from up as c
                    close down
                end check
            end check
        end loop
    "#
    );

    // 4s idle timeout: with the fix the stranded handler is cancelled in ~20ms once
    // its entry is pruned; without it, it hangs until this timeout.
    let server = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let ast = Parser::new(&tokens).parse().expect("parse");
            let config = WflConfig {
                timeout_seconds: 4,
                ..WflConfig::default()
            };
            let mut interp = Interpreter::with_config(Arc::new(config));
            let _ = interp.interpret(&ast).await;
        });
    });
    wait_for_server(proxy_port).await;

    // Continuous pruning traffic: every `/kick` runs `wait for request`, which prunes
    // all closed pending entries. Keep it dense so a prune lands in A's poll gap.
    let kick_stop = Arc::new(AtomicBool::new(false));
    let kick_stop2 = Arc::clone(&kick_stop);
    let kicker = tokio::spawn(async move {
        let client = reqwest::Client::new();
        while !kick_stop2.load(Ordering::Relaxed) {
            let _ = client
                .get(format!("http://127.0.0.1:{proxy_port}/kick"))
                .timeout(Duration::from_secs(2))
                .send()
                .await;
        }
    });

    // Let the pruning traffic ramp up, then connect A, let it block opening the
    // header-stalled upstream, and disconnect it while the prunes are flowing.
    tokio::time::sleep(Duration::from_millis(200)).await;
    {
        let mut sock = tokio::net::TcpStream::connect(("127.0.0.1", proxy_port))
            .await
            .expect("connect A");
        sock.write_all(b"GET /proxy HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
            .await
            .expect("send A request");
        sock.flush().await.ok();
        tokio::time::sleep(Duration::from_millis(300)).await;
        // `sock` drops -> A disconnects while pruning traffic flows.
    }

    // A's upstream must close PROMPTLY (cancelled), not at the 4s idle timeout.
    let start = Instant::now();
    tokio::time::timeout(Duration::from_secs(3), &mut upstream_closed)
        .await
        .expect("A's upstream was not cancelled after a sibling pruned its disconnected entry")
        .expect("upstream close sender dropped");
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(3),
        "the stranded pre-head handler should be cancelled promptly once pruned, \
         not hang to the idle timeout; took {elapsed:?}"
    );

    kick_stop.store(true, Ordering::Relaxed);
    let _ = kicker.await;

    let _ = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{proxy_port}/shutdown"))
        .timeout(Duration::from_secs(2))
        .send()
        .await;
    match tokio::task::spawn_blocking(move || server.join()).await {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(e) => panic!("server join task failed: {e}"),
    }
}
