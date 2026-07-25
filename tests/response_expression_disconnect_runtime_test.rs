//! Real-socket coverage for response-expression cancellation (issue #642).
//!
//! A buffered response body and each fallible streaming-head operand (status,
//! content type, and headers) are evaluated before the response commits. If the
//! browser disconnects during one of those evaluations, the handler must cancel
//! the evaluation, release its real outbound stream, and leave the concurrent
//! server able to serve unrelated work.
//!
//! Each case uses two real upstream sockets. The data upstream sends one chunk
//! and then withholds the rest of its body. After consuming that first chunk, the
//! WFL action opens a checkpoint stream, consumes its marker, explicitly closes
//! it, and only then blocks on the data upstream again. The test waits for the
//! checkpoint socket to close before dropping the browser socket, so the
//! synchronization is event-controlled rather than based on a handler sleep.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

mod common;

const PROBE_DEADLINE: Duration = Duration::from_secs(10);

struct EvaluationProbe {
    data_port: u16,
    checkpoint_port: u16,
    checkpoint_passed: oneshot::Receiver<Result<(), String>>,
    data_closed: oneshot::Receiver<Result<(), String>>,
}

async fn read_http_head(socket: &mut tokio::net::TcpStream) -> Result<Vec<u8>, String> {
    let mut request = Vec::new();
    let mut byte = [0u8; 1];
    while !request.ends_with(b"\r\n\r\n") {
        let count = socket
            .read(&mut byte)
            .await
            .map_err(|error| format!("read request head: {error}"))?;
        if count == 0 {
            return Err("peer closed before sending a complete request head".to_string());
        }
        request.push(byte[0]);
        if request.len() > 16 * 1024 {
            return Err("request head exceeded 16 KiB".to_string());
        }
    }
    Ok(request)
}

async fn wait_for_peer_close(socket: &mut tokio::net::TcpStream) {
    let mut byte = [0u8; 1];
    loop {
        match socket.read(&mut byte).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
    }
}

async fn spawn_evaluation_probe(label: &'static str) -> EvaluationProbe {
    let data_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("{label}: bind data upstream: {error}"));
    let data_port = data_listener
        .local_addr()
        .unwrap_or_else(|error| panic!("{label}: inspect data upstream address: {error}"))
        .port();
    let (data_closed_tx, data_closed) = oneshot::channel();
    tokio::spawn(async move {
        let result = async {
            let (mut socket, _) = data_listener
                .accept()
                .await
                .map_err(|error| format!("{label}: accept data upstream: {error}"))?;
            read_http_head(&mut socket).await?;
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\n\
                      Content-Type: application/octet-stream\r\n\
                      Transfer-Encoding: chunked\r\n\
                      Connection: keep-alive\r\n\r\n\
                      5\r\nready\r\n",
                )
                .await
                .map_err(|error| format!("{label}: write data marker: {error}"))?;
            socket
                .flush()
                .await
                .map_err(|error| format!("{label}: flush data marker: {error}"))?;
            wait_for_peer_close(&mut socket).await;
            Ok(())
        }
        .await;
        let _ = data_closed_tx.send(result);
    });

    let checkpoint_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("{label}: bind checkpoint upstream: {error}"));
    let checkpoint_port = checkpoint_listener
        .local_addr()
        .unwrap_or_else(|error| panic!("{label}: inspect checkpoint address: {error}"))
        .port();
    let (checkpoint_passed_tx, checkpoint_passed) = oneshot::channel();
    tokio::spawn(async move {
        let result = async {
            let (mut socket, _) = checkpoint_listener
                .accept()
                .await
                .map_err(|error| format!("{label}: accept checkpoint: {error}"))?;
            read_http_head(&mut socket).await?;
            // Deliberately omit the terminating zero-length chunk. The WFL
            // action reads this marker and then closes the still-live stream.
            // Observing that close proves the action passed the checkpoint.
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\n\
                      Content-Type: application/octet-stream\r\n\
                      Transfer-Encoding: chunked\r\n\
                      Connection: keep-alive\r\n\r\n\
                      1\r\nA\r\n",
                )
                .await
                .map_err(|error| format!("{label}: write checkpoint marker: {error}"))?;
            socket
                .flush()
                .await
                .map_err(|error| format!("{label}: flush checkpoint marker: {error}"))?;
            wait_for_peer_close(&mut socket).await;
            Ok(())
        }
        .await;
        let _ = checkpoint_passed_tx.send(result);
    });

    EvaluationProbe {
        data_port,
        checkpoint_port,
        checkpoint_passed,
        data_closed,
    }
}

struct ProxyServer {
    thread: Option<std::thread::JoinHandle<()>>,
    abort: Option<oneshot::Sender<()>>,
}

impl Drop for ProxyServer {
    fn drop(&mut self) {
        if let Some(abort) = self.abort.take() {
            let _ = abort.send(());
        }
    }
}

fn start_proxy_server(code: String) -> ProxyServer {
    let (abort, abort_rx) = oneshot::channel();
    let thread = std::thread::Builder::new()
        .name("response-expression-disconnect-proxy".to_string())
        .stack_size(wfl::INTERPRETER_STACK_SIZE)
        .spawn(move || {
            let runtime = tokio::runtime::Runtime::new().expect("create proxy runtime");
            runtime.block_on(async {
                let tokens = lex_wfl_with_positions(&code);
                let ast = Parser::new(&tokens)
                    .parse()
                    .unwrap_or_else(|errors| panic!("parse proxy program: {errors:?}"));
                let mut interpreter = Interpreter::new();
                tokio::select! {
                    result = interpreter.interpret(&ast) => {
                        if let Err(errors) = result {
                            panic!("proxy interpreter failed: {errors:?}");
                        }
                    }
                    _ = abort_rx => {
                        // Test cleanup drops the interpreter and all live listeners.
                    }
                }
            });
        })
        .expect("spawn proxy interpreter thread");
    ProxyServer {
        thread: Some(thread),
        abort: Some(abort),
    }
}

async fn wait_for_server(port: u16) {
    let address = format!("127.0.0.1:{port}");
    for _ in 0..300 {
        if tokio::net::TcpStream::connect(&address).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("proxy server on {address} did not become ready");
}

async fn assert_ping_survives(port: u16, context: &str) {
    let response = tokio::time::timeout(
        PROBE_DEADLINE,
        reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/ping"))
            .send(),
    )
    .await
    .unwrap_or_else(|_| panic!("{context}: /ping timed out after expression cancellation"))
    .unwrap_or_else(|error| panic!("{context}: /ping failed: {error}"));
    assert_eq!(
        response.status().as_u16(),
        200,
        "{context}: server returned a non-success /ping status"
    );
    assert_eq!(
        response
            .text()
            .await
            .unwrap_or_else(|error| panic!("{context}: read /ping body: {error}")),
        "pong",
        "{context}: server returned the wrong /ping body"
    );
}

async fn disconnect_at_checkpoint(
    proxy_port: u16,
    path: &'static str,
    label: &'static str,
    mut probe: EvaluationProbe,
) {
    let mut browser = tokio::net::TcpStream::connect(("127.0.0.1", proxy_port))
        .await
        .unwrap_or_else(|error| panic!("{label}: connect browser socket: {error}"));
    browser
        .write_all(
            format!(
                "GET {path} HTTP/1.1\r\n\
                 Host: 127.0.0.1\r\n\
                 Connection: close\r\n\r\n"
            )
            .as_bytes(),
        )
        .await
        .unwrap_or_else(|error| panic!("{label}: send browser request: {error}"));
    browser
        .flush()
        .await
        .unwrap_or_else(|error| panic!("{label}: flush browser request: {error}"));

    tokio::time::timeout(PROBE_DEADLINE, &mut probe.checkpoint_passed)
        .await
        .unwrap_or_else(|_| {
            panic!("{label}: response expression never passed its upstream checkpoint")
        })
        .unwrap_or_else(|_| panic!("{label}: checkpoint task ended without a result"))
        .unwrap_or_else(|error| panic!("{error}"));

    assert!(
        matches!(
            probe.data_closed.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ),
        "{label}: data upstream closed before the browser disconnected"
    );

    // The action has consumed its first real upstream chunk, passed and closed
    // its marker stream, and is now blocked reading the unfinished data body.
    // Dropping this socket is the only release signal for that evaluation.
    drop(browser);

    tokio::time::timeout(PROBE_DEADLINE, &mut probe.data_closed)
        .await
        .unwrap_or_else(|_| {
            panic!("{label}: stalled upstream remained open after browser disconnect")
        })
        .unwrap_or_else(|_| panic!("{label}: data upstream task ended without a result"))
        .unwrap_or_else(|error| panic!("{error}"));

    assert_ping_survives(proxy_port, label).await;
}

async fn shutdown_proxy(port: u16, mut server: ProxyServer) {
    let _ = tokio::time::timeout(
        PROBE_DEADLINE,
        reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/shutdown"))
            .send(),
    )
    .await;

    let graceful = tokio::time::timeout(PROBE_DEADLINE, async {
        while !server
            .thread
            .as_ref()
            .expect("proxy thread exists until shutdown")
            .is_finished()
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .is_ok();
    if !graceful {
        let _ = server
            .abort
            .take()
            .expect("proxy abort signal is sent at most once")
            .send(());
    }

    let thread = server
        .thread
        .take()
        .expect("proxy thread is joined at most once");
    match tokio::task::spawn_blocking(move || thread.join()).await {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(error) => panic!("proxy join task failed: {error}"),
    }
}

fn stalled_action(
    name: &str,
    prefix: &str,
    probe: &EvaluationProbe,
    return_statements: &str,
) -> String {
    format!(
        r#"
define action called {name}:
    open url at "http://127.0.0.1:{data_port}/data" and stream response as {prefix}_source
    wait for next chunk from {prefix}_source as {prefix}_ready
    open url at "http://127.0.0.1:{checkpoint_port}/checkpoint" and stream response as {prefix}_checkpoint
    wait for next chunk from {prefix}_checkpoint as {prefix}_acknowledged
    close {prefix}_checkpoint
    wait for next chunk from {prefix}_source as {prefix}_blocked
{return_statements}
end action
"#,
        data_port = probe.data_port,
        checkpoint_port = probe.checkpoint_port,
    )
}

#[tokio::test]
async fn response_expression_disconnects_cancel_upstreams_and_preserve_server_liveness() {
    let buffered = spawn_evaluation_probe("buffered response content").await;
    let stream_status = spawn_evaluation_probe("streaming response status").await;
    let stream_content_type = spawn_evaluation_probe("streaming response content type").await;
    let stream_headers = spawn_evaluation_probe("streaming response headers").await;
    let proxy_port = common::free_tcp_port();

    let actions = [
        stalled_action(
            "stalled_buffered_content",
            "buffered_value",
            &buffered,
            "    return \"late body\"",
        ),
        stalled_action(
            "stalled_stream_status",
            "stream_status",
            &stream_status,
            "    return 201",
        ),
        stalled_action(
            "stalled_stream_content_type",
            "stream_content_type",
            &stream_content_type,
            "    return \"text/plain\"",
        ),
        stalled_action(
            "stalled_stream_headers",
            "stream_headers",
            &stream_headers,
            "    create map delayed_headers:\n        \"X-Probe\" is \"late\"\n    end map\n    return delayed_headers",
        ),
    ]
    .join("\n");

    let program = format!(
        r#"
{actions}
listen on port {proxy_port} as srv
main loop concurrently:
    wait for request comes in on srv as req with timeout 30000
    store request_path as req["path"]
    check if request_path is equal to "/ping":
        respond to req with "pong"
    otherwise:
        check if request_path is equal to "/shutdown":
            respond to req with "bye"
            close server srv
            break
        otherwise:
            check if request_path is equal to "/buffered":
                respond to req with call stalled_buffered_content
            otherwise:
                check if request_path is equal to "/stream-status":
                    start streaming response to req with status call stalled_stream_status as out
                    close out
                otherwise:
                    check if request_path is equal to "/stream-content-type":
                        start streaming response to req with status 200 and content type call stalled_stream_content_type as out
                        close out
                    otherwise:
                        start streaming response to req with status 200 and content type "text/plain" and headers call stalled_stream_headers as out
                        close out
                    end check
                end check
            end check
        end check
    end check
end loop
"#
    );

    let server = start_proxy_server(program);
    wait_for_server(proxy_port).await;

    disconnect_at_checkpoint(
        proxy_port,
        "/buffered",
        "buffered response content",
        buffered,
    )
    .await;
    disconnect_at_checkpoint(
        proxy_port,
        "/stream-status",
        "streaming response status",
        stream_status,
    )
    .await;
    disconnect_at_checkpoint(
        proxy_port,
        "/stream-content-type",
        "streaming response content type",
        stream_content_type,
    )
    .await;
    disconnect_at_checkpoint(
        proxy_port,
        "/stream-headers",
        "streaming response headers",
        stream_headers,
    )
    .await;

    shutdown_proxy(proxy_port, server).await;
}
