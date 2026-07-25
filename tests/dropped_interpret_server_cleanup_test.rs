//! Real-socket regression (maintainer re-review, P1): when the `interpret()` future
//! is DROPPED after a handler starts a streaming response, the interpret-scoped
//! cleanup guard must close the server response stream (ending the client's body)
//! and 500 any unanswered request — even though the reusable `Interpreter` itself
//! stays alive.
//!
//! The tests use explicit drop/release channels. They first observe the exact
//! lifecycle checkpoint, drop only the `interpret()` future, assert the client
//! outcome while the interpreter remains alive, and release the interpreter
//! immediately afterward. No fixed multi-second sleep stands in for dequeue or
//! cleanup completion.

use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use wfl::Interpreter;
use wfl::config::WflConfig;
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

struct ControlledServer {
    thread: std::thread::JoinHandle<()>,
    drop_run: Option<tokio::sync::oneshot::Sender<()>>,
    dropped: tokio::sync::oneshot::Receiver<()>,
    release_interpreter: Option<tokio::sync::oneshot::Sender<()>>,
}

/// Run `interpret()` until the test explicitly drops that future, then keep the
/// same interpreter alive until the client-side assertions finish.
fn start_controlled_server(code: String) -> ControlledServer {
    let (drop_run, drop_rx) = tokio::sync::oneshot::channel();
    let (dropped_tx, dropped) = tokio::sync::oneshot::channel();
    let (release_interpreter, release_rx) = tokio::sync::oneshot::channel();
    let thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("server runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let program = Parser::new(&tokens).parse().expect("parse");
            let config = WflConfig {
                timeout_seconds: 60,
                ..WflConfig::default()
            };
            let mut interp = Interpreter::with_config(Arc::new(config));
            {
                let future = interp.interpret(&program);
                tokio::pin!(future);
                tokio::select! {
                    result = &mut future => {
                        panic!(
                            "interpret() returned before the test requested its drop: {result:?}"
                        );
                    }
                    signal = drop_rx => {
                        signal.expect("drop-run controller disappeared");
                    }
                }
            }
            // The pinned future and its cleanup guard have now been dropped, while
            // the reusable interpreter remains alive below.
            let _ = dropped_tx.send(());
            // Dropping the sender during a test panic releases this immediately;
            // the timeout is only a final leak backstop.
            let _ = tokio::time::timeout(Duration::from_secs(10), release_rx).await;
            drop(interp);
        });
    });

    ControlledServer {
        thread,
        drop_run: Some(drop_run),
        dropped,
        release_interpreter: Some(release_interpreter),
    }
}

async fn request_run_drop(server: &mut ControlledServer) {
    server
        .drop_run
        .take()
        .expect("drop signal is sent exactly once")
        .send(())
        .expect("controlled server still waits for the drop signal");
    tokio::time::timeout(Duration::from_secs(3), &mut server.dropped)
        .await
        .expect("interpret() future was not dropped promptly")
        .expect("controlled server ended before reporting the drop");
}

async fn finish_controlled_server(mut server: ControlledServer) {
    let _ = server
        .release_interpreter
        .take()
        .expect("interpreter release is sent exactly once")
        .send(());
    tokio::time::timeout(Duration::from_secs(12), async {
        while !server.thread.is_finished() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("controlled server did not stop within 12 seconds");
    if let Err(panic) = server.thread.join() {
        std::panic::resume_unwind(panic);
    }
}

/// A handler calls this endpoint only after it dequeues the real client request.
/// The mock reports that exact checkpoint and withholds its response, keeping the
/// handler's pending response parked until the test drops `interpret()`.
async fn spawn_dequeue_checkpoint() -> (u16, tokio::sync::oneshot::Receiver<Result<(), String>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dequeue checkpoint");
    let port = listener
        .local_addr()
        .expect("dequeue checkpoint address")
        .port();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let setup = async {
            let (mut socket, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
                .await
                .map_err(|_| "timed out waiting for dequeue checkpoint".to_string())?
                .map_err(|error| format!("accept dequeue checkpoint: {error}"))?;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            let mut head = Vec::new();
            let mut buffer = [0u8; 512];
            loop {
                let count = tokio::time::timeout_at(deadline, socket.read(&mut buffer))
                    .await
                    .map_err(|_| "timed out reading dequeue checkpoint".to_string())?
                    .map_err(|error| format!("read dequeue checkpoint: {error}"))?;
                if count == 0 {
                    return Err("handler closed before sending the dequeue checkpoint".to_string());
                }
                head.extend_from_slice(&buffer[..count]);
                if head.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
                if head.len() > 16 * 1024 {
                    return Err("dequeue checkpoint head exceeded 16 KiB".to_string());
                }
            }
            let request = String::from_utf8_lossy(&head);
            if !request.starts_with("GET /dequeued ") {
                return Err(format!(
                    "unexpected dequeue checkpoint request: {:?}",
                    request.lines().next()
                ));
            }
            Ok(socket)
        }
        .await;

        match setup {
            Ok(mut socket) => {
                let _ = ready_tx.send(Ok(()));
                // Keep the outbound request parked. Dropping `interpret()` cancels
                // it and closes this socket; no HTTP response is intentionally sent.
                let _ = tokio::time::timeout(Duration::from_secs(10), async {
                    let mut byte = [0u8; 1];
                    loop {
                        match socket.read(&mut byte).await {
                            Ok(0) | Err(_) => break,
                            Ok(_) => {}
                        }
                    }
                })
                .await;
            }
            Err(error) => {
                let _ = ready_tx.send(Err(error));
            }
        }
    });
    (port, ready_rx)
}

#[tokio::test]
async fn test_dropped_run_closes_server_stream_while_interpreter_stays_alive() {
    let port = common::free_tcp_port();

    // Handler: start streaming, send a chunk, flush, then park. Receiving `hello`
    // is the exact checkpoint that the server stream exists before the run is
    // explicitly dropped.
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop:
            wait for request comes in on srv as req with timeout 60000
            start streaming response to req with status 200 and content type "text/plain" as out
            write chunk "hello" to out
            flush out
            wait for 60000 milliseconds
        end loop
    "#
    );

    let mut server = start_controlled_server(code);
    wait_for_server(port).await;

    let mut response = tokio::time::timeout(
        Duration::from_secs(5),
        reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/"))
            .send(),
    )
    .await
    .expect("timed out waiting for streaming response")
    .expect("streaming request failed");
    assert_eq!(response.status().as_u16(), 200);

    let mut body = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !String::from_utf8_lossy(&body).contains("hello") {
            match response.chunk().await {
                Ok(Some(bytes)) => body.extend_from_slice(&bytes),
                Ok(None) => panic!("stream ended before the pre-drop chunk arrived"),
                Err(error) => panic!("stream failed before the run was dropped: {error}"),
            }
        }
    })
    .await
    .expect("timed out waiting for the pre-drop streamed chunk");

    request_run_drop(&mut server).await;

    // The interpreter is deliberately still alive here. Only the run's cleanup
    // guard can close the body, and it must be a clean end rather than a transport
    // failure accepted as equivalent.
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            match response.chunk().await {
                Ok(Some(bytes)) => body.extend_from_slice(&bytes),
                Ok(None) => break,
                Err(error) => {
                    panic!("drop-guard stream cleanup must end with clean EOF: {error}")
                }
            }
        }
    })
    .await
    .expect("client body did not end while the interpreter was kept alive");
    assert!(
        String::from_utf8_lossy(&body).contains("hello"),
        "the pre-drop streamed chunk disappeared: {:?}",
        String::from_utf8_lossy(&body)
    );

    finish_controlled_server(server).await;
}

#[tokio::test]
async fn test_dropped_run_answers_pending_request_with_500() {
    let (checkpoint_port, checkpoint_ready) = spawn_dequeue_checkpoint().await;
    let port = common::free_tcp_port();
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop:
            wait for request comes in on srv as req with timeout 60000
            open url at "http://127.0.0.1:{checkpoint_port}/dequeued" and stream response as checkpoint
            wait for 60000 milliseconds
        end loop
    "#
    );

    let mut server = start_controlled_server(code);
    wait_for_server(port).await;

    let url = format!("http://127.0.0.1:{port}/");
    let request = tokio::spawn(async move { reqwest::Client::new().get(url).send().await });
    tokio::time::timeout(Duration::from_secs(5), checkpoint_ready)
        .await
        .expect("handler did not reach the post-dequeue checkpoint")
        .expect("dequeue checkpoint task ended without a result")
        .expect("dequeue checkpoint failed");

    request_run_drop(&mut server).await;

    let response = tokio::time::timeout(Duration::from_secs(5), request)
        .await
        .expect("client hung waiting for the dropped-run 500")
        .expect("pending request task panicked")
        .expect("pending request failed");
    assert_eq!(
        response.status().as_u16(),
        500,
        "dropped run must answer the still-pending request with 500, got {}",
        response.status()
    );
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/plain; charset=utf-8"),
        "dropped-run cleanup must use the explicit plain-text 500 response"
    );
    let body = tokio::time::timeout(Duration::from_secs(3), response.bytes())
        .await
        .expect("timed out reading dropped-run cleanup response body")
        .expect("read dropped-run cleanup response body");
    assert_eq!(
        body.as_ref(),
        b"Internal Server Error\n",
        "dropped-run cleanup must resolve the pending request with its exact 500 body"
    );

    finish_controlled_server(server).await;
}
