//! Real-socket regression for P1: a downstream (browser) disconnect cancels a
//! handler BLOCKED in an upstream `wait for next chunk`, closing the upstream TCP
//! connection and recovering the handler — instead of the handler hanging until
//! the absolute stream deadline.
//!
//! Topology: mock upstream (sends one chunk, then stalls) <- WFL proxy server ->
//! reqwest client. The client reads the first proxied chunk, then disconnects
//! while the handler is blocked reading the (stalled) upstream. The mock detects
//! its own connection closing (a blocking read returns 0 at peer close) only if
//! the handler's blocked upstream read was actually cancelled.

use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Upstream: send a chunked head + one body chunk, then STALL (send nothing
/// more, so the proxy's next read blocks). Detect the proxy dropping the
/// connection via a blocking read that returns 0 at peer close.
async fn spawn_one_chunk_then_stall_upstream() -> (u16, tokio::sync::oneshot::Receiver<()>) {
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
            let _ = sock.write_all(b"5\r\nhello\r\n").await;
            let _ = sock.flush().await;
            // Stall: send nothing more. Block on read; when the proxy drops the
            // upstream (its blocked read cancelled by the client disconnect), the
            // peer close surfaces here as Ok(0) / Err.
            loop {
                match sock.read(&mut buf).await {
                    Ok(0) | Err(_) => {
                        let _ = tx.send(());
                        return;
                    }
                    Ok(_) => {} // unexpected client->server data; ignore
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
async fn test_downstream_disconnect_cancels_blocked_upstream_read() {
    let (upstream_port, mut upstream_disconnect) = spawn_one_chunk_then_stall_upstream().await;

    let proxy_port = 8351;
    // The handler proxies: read chunks from upstream and write them downstream.
    // After the first chunk it blocks on the stalled upstream. `outbound_stream_max_seconds`
    // is the default (300s), so ONLY a disconnect can cancel that blocked read
    // within the test window.
    let code = format!(
        r#"
        listen on port {proxy_port} as srv
        main loop concurrently:
            wait for request comes in on srv as req with timeout 20000
            store p as req["path"]
            check if p is equal to "/shutdown":
                respond to req with "bye"
                close server srv
                break
            otherwise:
                open url at "http://127.0.0.1:{upstream_port}/" and stream response as up
                start streaming response to req with status 200 and content type "text/plain" as down
                count from 1 to 100:
                    wait for next chunk from up as c
                    check if c is nothing:
                        break
                    otherwise:
                        write chunk c to down
                    end check
                end count
            end check
        end loop
    "#
    );
    let server = start_proxy_server(code);
    wait_for_server(proxy_port).await;

    // Client: read the first proxied chunk, then DISCONNECT (drop the response)
    // while the handler is blocked reading the stalled upstream.
    {
        let resp = reqwest::Client::new()
            .get(format!("http://127.0.0.1:{proxy_port}/proxy"))
            .send()
            .await
            .expect("proxy request failed");
        assert_eq!(resp.status().as_u16(), 200);
        let mut resp = resp;
        let first = resp.chunk().await.expect("read first chunk");
        assert_eq!(
            first.as_deref(),
            Some(&b"hello"[..]),
            "expected the first proxied chunk"
        );
        // Drop `resp` here -> client disconnects.
    }

    // The mock upstream must observe ITS connection close promptly — proving the
    // handler's blocked upstream read was cancelled by the disconnect (not left
    // hanging until the absolute deadline).
    tokio::time::timeout(Duration::from_secs(5), &mut upstream_disconnect)
        .await
        .expect(
            "upstream was not closed after the client disconnected — blocked read not cancelled",
        )
        .expect("upstream disconnect sender dropped");

    // Stop the proxy server.
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
