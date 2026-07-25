//! Real-socket regression for P1 (#1): a downstream (browser) disconnect must
//! cancel a proxy handler blocked in the UPSTREAM HEAD phase (`open url ... and
//! stream response`), before any `start streaming response`, not only a blocked
//! body read.
//!
//! Topology: an upstream that WITHHOLDS its response head <- WFL concurrent proxy
//! -> a client that connects and disconnects while the handler is blocked opening
//! the upstream. The upstream must observe its connection close promptly (the
//! handler cancelled the head open because the client went away), and an unrelated
//! `/ping` must still be served throughout.

use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

mod common;

/// Upstream: accept, read the request, then WITHHOLD the response head (send
/// nothing). Signal when the proxy drops the connection (peer close => read 0/Err).
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
            // Withhold the response head entirely; just wait for the proxy to drop
            // the connection when its client disconnects.
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

#[tokio::test]
async fn test_disconnect_cancels_blocked_upstream_head_open() {
    let (upstream_port, mut upstream_closed) = spawn_header_withholding_upstream().await;
    let proxy_port = common::free_tcp_port();

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
                    wait for next chunk from up as c
                    close down
                end check
            end check
        end loop
    "#
    );
    let server = start_proxy_server(code);
    wait_for_server(proxy_port).await;

    // Client: connect, send the request, then DISCONNECT while the handler is
    // blocked opening the (header-withholding) upstream.
    {
        let mut sock = tokio::net::TcpStream::connect(("127.0.0.1", proxy_port))
            .await
            .expect("connect to proxy");
        sock.write_all(b"GET /proxy HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
            .await
            .expect("send request");
        sock.flush().await.ok();
        // Give the handler a moment to dequeue and reach the blocked head open,
        // then drop the socket to disconnect.
        tokio::time::sleep(Duration::from_millis(300)).await;
        // `sock` drops here -> client disconnects.
    }

    // The upstream must observe its connection close promptly — the blocked head
    // open was cancelled by the disconnect, not left to wait out the idle timeout.
    tokio::time::timeout(Duration::from_secs(4), &mut upstream_closed)
        .await
        .expect("upstream head open was not cancelled after the client disconnected")
        .expect("upstream close sender dropped");

    // The concurrent loop stayed alive: an unrelated request is still served.
    let ping = tokio::time::timeout(
        Duration::from_secs(5),
        reqwest::Client::new()
            .get(format!("http://127.0.0.1:{proxy_port}/ping"))
            .send(),
    )
    .await
    .expect("/ping timed out")
    .expect("/ping failed");
    assert_eq!(ping.status().as_u16(), 200);
    assert_eq!(ping.text().await.unwrap(), "pong");

    let _ = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{proxy_port}/shutdown"))
        .send()
        .await;
    match tokio::task::spawn_blocking(move || server.join()).await {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(e) => panic!("server join task failed: {e}"),
    }
}
