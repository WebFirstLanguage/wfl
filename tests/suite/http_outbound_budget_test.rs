//! Security regressions for bounded outbound HTTP responses.
//!
//! These tests use a minimal local TCP peer so they are deterministic and do
//! not require internet access. Together they exercise all three runtime paths:
//! legacy GET, legacy POST, and the arbitrary-method/full-response statement.

use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

use wfl::config::WflConfig;
use wfl::exec::budget::{BudgetLimits, ExecutionBudget};
use wfl::interpreter::Interpreter;
use wfl::interpreter::error::ErrorKind;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, Literal, Program, Statement};

fn parse(source: &str) -> Program {
    let tokens = lex_wfl_with_positions(source);
    Parser::new(&tokens)
        .parse()
        .unwrap_or_else(|errors| panic!("WFL source should parse: {errors:?}"))
}

async fn read_request_headers(socket: &mut TcpStream) {
    let mut request = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let read = socket
            .read(&mut chunk)
            .await
            .expect("read local HTTP request");
        assert!(read > 0, "client closed before completing HTTP headers");
        request.extend_from_slice(&chunk[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return;
        }
        assert!(
            request.len() <= 64 * 1024,
            "unexpectedly large test request"
        );
    }
}

/// Spawn a one-shot HTTP peer that writes `response_prefix`. When `stall` is
/// true it keeps the connection open afterward instead of completing the body.
async fn spawn_http_peer(
    response_prefix: &'static [u8],
    stall: bool,
) -> (
    String,
    tokio::task::JoinHandle<()>,
    oneshot::Receiver<Result<(), std::io::ErrorKind>>,
) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local HTTP peer");
    let address = listener.local_addr().expect("local HTTP peer address");
    let (response_attempted, response_attempted_rx) = oneshot::channel();
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept HTTP request");
        read_request_headers(&mut socket).await;
        let write_result = socket
            .write_all(response_prefix)
            .await
            .map_err(|error| error.kind());
        let _ = response_attempted.send(write_result);
        if stall {
            std::future::pending::<()>().await;
        }
        let _ = socket.shutdown().await;
    });
    (format!("http://{address}"), handle, response_attempted_rx)
}

async fn await_http_peer(server: tokio::task::JoinHandle<()>) {
    tokio::time::timeout(Duration::from_secs(1), server)
        .await
        .expect("local HTTP peer must not hang")
        .expect("local HTTP peer task must not panic");
}

fn assert_response_limit(errors: &[wfl::interpreter::error::RuntimeError], limit: usize) {
    let error = errors.first().expect("one runtime error");
    assert_eq!(error.kind, ErrorKind::ResourceLimit);
    assert!(
        error.message.contains("Response body too large"),
        "expected response-size diagnostic, got: {error:?}"
    );
    assert!(
        error.message.contains(&format!("limit: {limit} bytes")),
        "diagnostic should include the configured limit: {error:?}"
    );
}

#[tokio::test]
async fn legacy_get_rejects_oversized_content_length() {
    let response = b"HTTP/1.1 200 OK\r\n\
Content-Type: text/plain\r\n\
Content-Length: 32\r\n\
Connection: close\r\n\
\r\n\
0123456789abcdef0123456789abcdef";
    let (url, server, _response_attempted) = spawn_http_peer(response, false).await;

    let config = WflConfig {
        web_server_max_response_size: 8,
        ..Default::default()
    };
    let mut interpreter = Interpreter::with_config(Arc::new(config));
    let program = parse(&format!(
        r#"open url at "{url}" and read content as content"#
    ));

    let errors = interpreter
        .interpret(&program)
        .await
        .expect_err("advertised response above the cap must fail");
    assert_response_limit(&errors, 8);
    await_http_peer(server).await;
}

#[tokio::test]
async fn full_response_request_rejects_oversized_chunked_body() {
    // The response has no Content-Length, so only incremental accounting can
    // catch that the decoded body grows from five to nine bytes over an 8-byte
    // cap.
    let response = b"HTTP/1.1 200 OK\r\n\
Content-Type: text/plain\r\n\
Transfer-Encoding: chunked\r\n\
Connection: close\r\n\
\r\n\
5\r\n12345\r\n\
4\r\n6789\r\n\
0\r\n\r\n";
    let (url, server, _response_attempted) = spawn_http_peer(response, false).await;

    let config = WflConfig {
        web_server_max_response_size: 8,
        ..Default::default()
    };
    let mut interpreter = Interpreter::with_config(Arc::new(config));
    let program = parse(&format!(
        r#"open url at "{url}" and read response as reply"#
    ));

    let errors = interpreter
        .interpret(&program)
        .await
        .expect_err("chunked response above the cap must fail");
    assert_response_limit(&errors, 8);
    await_http_peer(server).await;
}

#[tokio::test]
async fn decoded_text_cannot_expand_past_the_response_limit() {
    // Four malformed UTF-8 bytes decode to four three-byte replacement
    // characters. The wire body fits the four-byte limit; the decoded text
    // must still be rejected before it can expand beyond that same ceiling.
    let response = b"HTTP/1.1 200 OK\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
Content-Length: 4\r\n\
Connection: close\r\n\
\r\n\
\xff\xff\xff\xff";
    let (url, server, _response_attempted) = spawn_http_peer(response, false).await;

    let config = WflConfig {
        web_server_max_response_size: 4,
        ..Default::default()
    };
    let mut interpreter = Interpreter::with_config(Arc::new(config));
    let program = parse(&format!(
        r#"open url at "{url}" and read content as content"#
    ));

    let errors = interpreter
        .interpret(&program)
        .await
        .expect_err("decoded response expansion above the cap must fail");
    assert_response_limit(&errors, 4);
    await_http_peer(server).await;
}

#[tokio::test]
async fn legacy_post_body_read_observes_cooperative_cancellation() {
    // Headers promise five bytes, but the peer never sends them. Before the
    // fix, Response::text() could remain parked here indefinitely.
    let response = b"HTTP/1.1 200 OK\r\n\
Content-Type: text/plain\r\n\
Content-Length: 5\r\n\
Connection: close\r\n\
\r\n";
    let (url, server, response_attempted) = spawn_http_peer(response, true).await;

    let mut interpreter = Interpreter::new();
    let budget = interpreter.budget();
    // Construct the legacy node directly: `with data` is no longer accepted by
    // the current grammar, but embedded/previously parsed programs still reach
    // the dedicated HttpPostStatement execution path.
    let program = Program {
        statements: vec![Statement::HttpPostStatement {
            url: Expression::Literal(Literal::String(url.into()), 1, 1),
            data: Expression::Literal(Literal::String("x=1".into()), 1, 1),
            variable_name: "reply".to_string(),
            line: 1,
            column: 1,
        }],
    };
    let interpret = interpreter.interpret(&program);
    tokio::pin!(interpret);
    tokio::time::timeout(Duration::from_secs(1), async {
        tokio::select! {
            result = response_attempted => {
                assert_eq!(
                    result.expect("local HTTP peer reports its response write"),
                    Ok(()),
                    "the cancellation regression must reach the stalled body read"
                );
                budget.cancel();
            }
            result = &mut interpret => {
                panic!("request completed before the peer entered its stalled body: {result:?}");
            }
        }
    })
    .await
    .expect("request must reach the peer's stalled response promptly");

    let errors = tokio::time::timeout(Duration::from_secs(1), &mut interpret)
        .await
        .expect("in-flight POST should observe cancellation promptly")
        .expect_err("cancelled POST must fail");
    let error = errors.first().expect("one runtime error");
    assert_eq!(error.kind, ErrorKind::ResourceLimit);
    assert!(
        error.message.contains("Execution was cancelled"),
        "expected cancellation diagnostic, got: {error:?}"
    );

    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn main_loop_gives_each_outbound_request_a_finite_timeout() {
    // A main loop is deliberately exempt from the run-lifetime deadline. An
    // individual outbound request inside it must still reuse that configured
    // duration as a fresh per-request limit.
    let response = b"HTTP/1.1 200 OK\r\n\
Content-Type: text/plain\r\n\
Content-Length: 5\r\n\
Connection: close\r\n\
\r\n";
    let (url, server, _response_attempted) = spawn_http_peer(response, true).await;

    let config = Arc::new(WflConfig::default());
    let mut interpreter = Interpreter::with_config(Arc::clone(&config));
    let program = parse(&format!(
        r#"
main loop:
    open url at "{url}" and read content as content
    break
end loop
"#
    ));

    // Install the deliberately short budget only after building the HTTP
    // client and parsing the fixture. The budget's start instant covers the
    // whole run, so including unrelated setup here can exhaust it before the
    // interpreter enters the main loop on slower CI hosts. This test is about
    // the fresh per-request deadline applied *inside* that loop.
    let limits = BudgetLimits {
        max_duration: Some(Duration::from_millis(250)),
        ..BudgetLimits::from_config(&config)
    };
    interpreter.set_budget(Arc::new(ExecutionBudget::new(limits)));

    let errors = tokio::time::timeout(Duration::from_secs(2), interpreter.interpret(&program))
        .await
        .expect("main-loop request should have a finite timeout")
        .expect_err("stalled main-loop request must time out");
    let error = errors.first().expect("one runtime error");
    assert_eq!(error.kind, ErrorKind::Timeout);
    assert!(
        error
            .message
            .contains("Outbound HTTP request exceeded timeout"),
        "expected outbound timeout diagnostic, got: {error:?}"
    );

    server.abort();
    let _ = server.await;
}
