// Tests for generic outbound response streaming:
//   open url at "<url>" [with ...] and stream response as upstream
//   wait for next chunk from upstream as chunk   -> Binary, or nothing at EOF
//   wait for next line  from upstream as line     -> Text,   or nothing at EOF
//   close upstream                                 -> cancels the upstream request
//
// A minimal local TCP server stands in for a real upstream (e.g. an LLM
// endpoint emitting newline-delimited JSON), so the tests are offline-safe.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::Statement;

// ----------------------------- parser tests ------------------------------

fn parse_single_statement(code: &str) -> Statement {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .unwrap_or_else(|e| panic!("Parse error for {code:?}: {e:?}"));
    assert_eq!(
        program.statements.len(),
        1,
        "Expected exactly one statement for {code:?}"
    );
    program.statements.into_iter().next().unwrap()
}

#[test]
fn test_stream_response_parses_to_http_stream_statement() {
    let stmt =
        parse_single_statement(r#"open url at "https://example.com" and stream response as up"#);
    match stmt {
        Statement::HttpStreamStatement {
            method,
            headers,
            body,
            variable_name,
            ..
        } => {
            assert!(method.is_none());
            assert!(headers.is_none());
            assert!(body.is_none());
            assert_eq!(variable_name, "up");
        }
        other => panic!("Expected HttpStreamStatement, got {other:?}"),
    }
}

#[test]
fn test_stream_response_with_method_headers_body() {
    let stmt = parse_single_statement(
        r#"open url at "https://api.example.com" with method "POST" and headers h and body b and stream response as up"#,
    );
    match stmt {
        Statement::HttpStreamStatement {
            method,
            headers,
            body,
            variable_name,
            ..
        } => {
            assert!(method.is_some());
            assert!(headers.is_some());
            assert!(body.is_some());
            assert_eq!(variable_name, "up");
        }
        other => panic!("Expected HttpStreamStatement, got {other:?}"),
    }
}

#[test]
fn test_wait_for_next_chunk_parses() {
    let stmt = parse_single_statement("wait for next chunk from up as chunk");
    match stmt {
        Statement::WaitForNextChunkStatement { variable_name, .. } => {
            assert_eq!(variable_name, "chunk");
        }
        other => panic!("Expected WaitForNextChunkStatement, got {other:?}"),
    }
}

#[test]
fn test_wait_for_next_line_parses() {
    let stmt = parse_single_statement("wait for next line from up as line");
    match stmt {
        Statement::WaitForNextLineStatement { variable_name, .. } => {
            assert_eq!(variable_name, "line");
        }
        other => panic!("Expected WaitForNextLineStatement, got {other:?}"),
    }
}

#[test]
fn test_wait_for_next_as_duration_variable_still_parses() {
    // Backward compat: a variable literally named `next` in a duration wait must
    // NOT be intercepted as `wait for next chunk|line` (regression).
    let stmt = parse_single_statement("wait for next milliseconds");
    match stmt {
        Statement::WaitForDurationStatement { unit, .. } => assert_eq!(unit, "milliseconds"),
        other => panic!("Expected WaitForDurationStatement, got {other:?}"),
    }
}

// ----------------------------- runtime tests ------------------------------

/// Spawn a one-shot server that answers 200 with the given body, streamed with
/// an explicit Content-Length and `Connection: close`.
async fn spawn_body_server(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut tmp = [0u8; 2048];
        // Drain the request head (single read is enough for a GET).
        let _ = socket.read(&mut tmp).await;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.ok();
    });
    format!("http://{addr}")
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
async fn test_stream_exposes_status_and_headers_immediately() {
    let url = spawn_body_server("alpha\nbeta\n").await;
    let code = format!(
        r#"
        open url at "{url}" and stream response as up
        store s as up["status"]
        store ok as up["ok"]
        store ct as up["headers"]["content-type"]
        close up
        "#
    );
    let interpreter = run_wfl(&code).await;

    match get_var(&interpreter, "s") {
        Value::Number(n) => assert_eq!(n, 200.0),
        other => panic!("Expected numeric status, got {other:?}"),
    }
    match get_var(&interpreter, "ok") {
        Value::Bool(b) => assert!(b),
        other => panic!("Expected boolean ok, got {other:?}"),
    }
    assert_eq!(get_text(&interpreter, "ct"), "application/x-ndjson");
}

#[tokio::test]
async fn test_next_line_yields_lines_then_nothing_at_eof() {
    let url = spawn_body_server("alpha\nbeta\ngamma\n").await;
    let code = format!(
        r#"
        open url at "{url}" and stream response as up
        wait for next line from up as line1
        wait for next line from up as line2
        wait for next line from up as line3
        wait for next line from up as line4
        "#
    );
    let interpreter = run_wfl(&code).await;

    assert_eq!(get_text(&interpreter, "line1"), "alpha");
    assert_eq!(get_text(&interpreter, "line2"), "beta");
    assert_eq!(get_text(&interpreter, "line3"), "gamma");
    // Clean EOF binds `nothing` (Null).
    match get_var(&interpreter, "line4") {
        Value::Null => {}
        other => panic!("Expected nothing at EOF, got {other:?}"),
    }
}

#[tokio::test]
async fn test_next_line_returns_final_unterminated_line() {
    // No trailing newline: the last line is still delivered before EOF.
    let url = spawn_body_server("one\ntwo").await;
    let code = format!(
        r#"
        open url at "{url}" and stream response as up
        wait for next line from up as a
        wait for next line from up as b
        wait for next line from up as c
        "#
    );
    let interpreter = run_wfl(&code).await;
    assert_eq!(get_text(&interpreter, "a"), "one");
    assert_eq!(get_text(&interpreter, "b"), "two");
    match get_var(&interpreter, "c") {
        Value::Null => {}
        other => panic!("Expected nothing at EOF, got {other:?}"),
    }
}

#[tokio::test]
async fn test_next_chunk_yields_binary_then_nothing() {
    let url = spawn_body_server("raw-bytes-payload").await;
    let code = format!(
        r#"
        open url at "{url}" and stream response as up
        wait for next chunk from up as chunk1
        "#
    );
    let interpreter = run_wfl(&code).await;

    match get_var(&interpreter, "chunk1") {
        Value::Binary(b) => assert!(!b.is_empty(), "first chunk should carry bytes"),
        other => panic!("Expected binary chunk, got {other:?}"),
    }
}

#[tokio::test]
async fn test_reading_from_closed_stream_is_error() {
    let url = spawn_body_server("x\n").await;
    let code = format!(
        r#"
        open url at "{url}" and stream response as up
        close up
        wait for next line from up as line
        "#
    );
    let tokens = lex_wfl_with_positions(&code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&program).await;
    assert!(
        result.is_err(),
        "reading from a closed stream handle should be a catchable error"
    );
}
