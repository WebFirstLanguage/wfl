// Tests for streamed server responses (item 3):
//   start streaming response to <req> [with status <e>] [and content type <e>] as <out>
//   write line <value> to <out>     // frames a line (newline appended)
//   write chunk <value> to <out>    // raw bytes/text, verbatim
//   flush <out>
//   close <out>                     // ends the response body
//
// A WFL web server streams a response; a reqwest client reads it back.

use std::time::Duration;
use wfl::Interpreter;
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
fn test_start_streaming_response_parses() {
    let stmt = parse_single_statement(
        r#"start streaming response to req with status 200 and content type "application/x-ndjson" as out"#,
    );
    match stmt {
        Statement::StartStreamingResponseStatement {
            status,
            content_type,
            variable_name,
            ..
        } => {
            assert!(status.is_some());
            assert!(content_type.is_some());
            assert_eq!(variable_name, "out");
        }
        other => panic!("Expected StartStreamingResponseStatement, got {other:?}"),
    }
}

#[test]
fn test_write_line_parses() {
    let stmt = parse_single_statement(r#"write line payload to out"#);
    match stmt {
        Statement::StreamWriteStatement { is_line, .. } => assert!(is_line),
        other => panic!("Expected StreamWriteStatement, got {other:?}"),
    }
}

#[test]
fn test_write_chunk_parses() {
    let stmt = parse_single_statement(r#"write chunk payload to out"#);
    match stmt {
        Statement::StreamWriteStatement { is_line, .. } => assert!(!is_line),
        other => panic!("Expected StreamWriteStatement, got {other:?}"),
    }
}

#[test]
fn test_flush_parses() {
    let stmt = parse_single_statement("flush out");
    match stmt {
        Statement::FlushStreamStatement { .. } => {}
        other => panic!("Expected FlushStreamStatement, got {other:?}"),
    }
}

// ----------------------------- runtime tests ------------------------------

fn start_server_thread(code: String) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let mut parser = Parser::new(&tokens);
            let ast = parser.parse().expect("Failed to parse WFL code");
            let mut interpreter = Interpreter::new();
            let _ = interpreter.interpret(&ast).await;
        });
    })
}

#[tokio::test]
async fn test_streamed_response_lines_and_headers() {
    let port = 8231;
    let server_code = format!(
        r#"
        listen on port {port} as s
        wait for request comes in on s as req with timeout 10000
        start streaming response to req with status 200 and content type "application/x-ndjson" as out
        write line "alpha" to out
        write line "beta" to out
        flush out
        write line "gamma" to out
        close out
        close server s
    "#
    );

    let server_handle = start_server_thread(server_code);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{port}/stream"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status().as_u16(), 200);
    let ct = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(ct, "application/x-ndjson");

    let body = response.text().await.expect("Failed to read body");
    assert_eq!(body, "alpha\nbeta\ngamma\n");

    let _ = server_handle.join();
}

#[tokio::test]
async fn test_streamed_response_write_chunk_verbatim() {
    let port = 8232;
    let server_code = format!(
        r#"
        listen on port {port} as s
        wait for request comes in on s as req with timeout 10000
        start streaming response to req with status 201 and content type "text/plain" as out
        write chunk "one" to out
        write chunk "two" to out
        close out
        close server s
    "#
    );

    let server_handle = start_server_thread(server_code);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{port}/raw"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status().as_u16(), 201);
    let body = response.text().await.expect("Failed to read body");
    // write chunk does not append newlines.
    assert_eq!(body, "onetwo");

    let _ = server_handle.join();
}
