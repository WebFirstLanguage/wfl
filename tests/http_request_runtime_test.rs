// Runtime tests for the extended outbound HTTP client (issue #558).
// A minimal local TCP server stands in for a real API (e.g. Stripe), so the
// tests are offline-safe: they verify that method, headers, and body reach
// the server, and that status/ok/body/headers come back in the response
// object.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Captured request: (request line + headers, body)
type CapturedRequest = (String, String);

/// Spawns a one-shot HTTP server that captures the request and answers
/// 201 Created with a fixed body and an X-Test-Header.
async fn spawn_echo_server() -> (String, tokio::task::JoinHandle<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf: Vec<u8> = Vec::new();
        let mut tmp = [0u8; 1024];

        // Read until the header block is complete
        let header_end = loop {
            let n = socket.read(&mut tmp).await.unwrap();
            if n == 0 {
                panic!("Connection closed before headers were complete");
            }
            buf.extend_from_slice(&tmp[..n]);
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                break pos;
            }
        };

        let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
        let content_length = head
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);

        // Read the rest of the body if it hasn't arrived yet
        let body_start = header_end + 4;
        while buf.len() < body_start + content_length {
            let n = socket.read(&mut tmp).await.unwrap();
            if n == 0 {
                panic!("Connection closed before body was complete");
            }
            buf.extend_from_slice(&tmp[..n]);
        }
        let body =
            String::from_utf8_lossy(&buf[body_start..body_start + content_length]).to_string();

        let response_body = "session created";
        let response = format!(
            "HTTP/1.1 201 Created\r\nContent-Type: text/plain\r\nX-Test-Header: hello\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.ok();

        (head, body)
    });

    (format!("http://{addr}"), handle)
}

async fn run_wfl(code: &str) -> Interpreter {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .unwrap_or_else(|e| panic!("Parse error: {e:?}"));

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .unwrap_or_else(|e| panic!("Runtime error: {e:?}"));
    interpreter
}

fn get_var(interpreter: &Interpreter, name: &str) -> Value {
    interpreter
        .global_env()
        .borrow()
        .get(name)
        .unwrap_or_else(|| panic!("Variable '{name}' not found"))
}

fn get_text(interpreter: &Interpreter, name: &str) -> String {
    match get_var(interpreter, name) {
        Value::Text(t) => t.to_string(),
        other => panic!("Expected '{name}' to be text, got {other:?}"),
    }
}

#[tokio::test]
async fn test_post_with_headers_and_body_full_response() {
    let (url, captured) = spawn_echo_server().await;

    let code = format!(
        r#"
        create map request_headers:
            Authorization is "Bearer sk_test_123"
            "Content-Type" is "application/x-www-form-urlencoded"
        end map

        open url at "{url}" with method "POST" and headers request_headers and body "mode=payment&amount=1000" and read response as resp

        store response_status as resp["status"]
        store response_ok as resp["ok"]
        store response_body as resp["body"]
        store response_headers as resp["headers"]
        store test_header as response_headers["x-test-header"]
        "#
    );

    let interpreter = run_wfl(&code).await;
    let (head, body) = captured.await.unwrap();

    // The server saw the method, header, and body we sent
    assert!(
        head.starts_with("POST / HTTP/1.1"),
        "Expected POST request line, got: {head}"
    );
    let head_lower = head.to_lowercase();
    assert!(
        head_lower.contains("authorization: bearer sk_test_123"),
        "Authorization header missing from request: {head}"
    );
    assert!(
        head_lower.contains("content-type: application/x-www-form-urlencoded"),
        "Content-Type header missing from request: {head}"
    );
    assert_eq!(body, "mode=payment&amount=1000");

    // The WFL program saw the full response
    match get_var(&interpreter, "response_status") {
        Value::Number(n) => assert_eq!(n, 201.0),
        other => panic!("Expected numeric status, got {other:?}"),
    }
    match get_var(&interpreter, "response_ok") {
        Value::Bool(b) => assert!(b, "201 should be ok"),
        other => panic!("Expected boolean ok, got {other:?}"),
    }
    assert_eq!(get_text(&interpreter, "response_body"), "session created");
    assert_eq!(get_text(&interpreter, "test_header"), "hello");
}

#[tokio::test]
async fn test_response_object_supports_dot_access() {
    let (url, captured) = spawn_echo_server().await;

    let code = format!(
        r#"
        open url at "{url}" and read response as resp
        store dot_status as resp.status
        store dot_ok as resp.ok
        store dot_body as resp.body
        "#
    );

    let interpreter = run_wfl(&code).await;
    captured.await.unwrap();

    match get_var(&interpreter, "dot_status") {
        Value::Number(n) => assert_eq!(n, 201.0),
        other => panic!("Expected numeric status, got {other:?}"),
    }
    match get_var(&interpreter, "dot_ok") {
        Value::Bool(b) => assert!(b),
        other => panic!("Expected boolean ok, got {other:?}"),
    }
    assert_eq!(get_text(&interpreter, "dot_body"), "session created");
}

#[tokio::test]
async fn test_post_read_content_binds_body_text() {
    let (url, captured) = spawn_echo_server().await;

    let code = format!(
        r#"
        open url at "{url}" with method "POST" and body "x=1" and read content as reply
        "#
    );

    let interpreter = run_wfl(&code).await;
    let (head, body) = captured.await.unwrap();

    assert!(head.starts_with("POST / HTTP/1.1"));
    assert_eq!(body, "x=1");
    assert_eq!(get_text(&interpreter, "reply"), "session created");
}

#[tokio::test]
async fn test_body_concatenation_with_expression() {
    let (url, captured) = spawn_echo_server().await;

    let code = format!(
        r#"
        store amount as "1000"
        open url at "{url}" with method "PUT" and body "amount=" with amount and read response as resp
        "#
    );

    let _ = run_wfl(&code).await;
    let (head, body) = captured.await.unwrap();

    assert!(
        head.starts_with("PUT / HTTP/1.1"),
        "Expected PUT request line, got: {head}"
    );
    assert_eq!(body, "amount=1000");
}

#[tokio::test]
async fn test_invalid_method_is_runtime_error() {
    let code = r#"
        open url at "http://127.0.0.1:1" with method "NOT A METHOD" and read response as resp
    "#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();

    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&program).await;
    assert!(result.is_err(), "Invalid HTTP method should be an error");
}

#[tokio::test]
async fn test_non_2xx_status_is_not_an_error() {
    // A 404 response should come back as data, not raise a runtime error
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut tmp = [0u8; 2048];
        let _ = socket.read(&mut tmp).await;
        let response =
            "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nnot found";
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.ok();
    });

    let code = format!(
        r#"
        open url at "http://{addr}" and read response as resp
        store response_status as resp.status
        store response_ok as resp.ok
        "#
    );

    let interpreter = run_wfl(&code).await;

    match get_var(&interpreter, "response_status") {
        Value::Number(n) => assert_eq!(n, 404.0),
        other => panic!("Expected numeric status, got {other:?}"),
    }
    match get_var(&interpreter, "response_ok") {
        Value::Bool(b) => assert!(!b, "404 should not be ok"),
        other => panic!("Expected boolean ok, got {other:?}"),
    }
}
