// TDD tests for custom response headers on the `respond` statement.
//
// Motivation: RFC 10008 (The HTTP QUERY Method) requires servers to be able to
// advertise `Accept-Query` and to point at query results with `Content-Location`
// / `Location`. Until now `respond to` could only set the status and content
// type, so a WFL server had no way to emit those headers. This adds an optional
// `and headers <map>` clause that mirrors the outbound client's `with headers`
// form (same concept, same syntax — nothing to unlearn).
//
// New syntax:
//   respond to req with "body" and content_type "application/json" and headers h
//
// where `h` is a map of header name -> value.

use std::time::Duration;
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::Statement;

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
fn test_respond_without_headers_leaves_headers_none() {
    // Backward compatibility: the existing form must keep parsing with no headers.
    let stmt =
        parse_single_statement(r#"respond to req with "hello" and content_type "text/plain""#);
    match stmt {
        Statement::RespondStatement {
            headers,
            content_type,
            ..
        } => {
            assert!(headers.is_none(), "Expected no headers clause");
            assert!(content_type.is_some(), "content_type should still parse");
        }
        other => panic!("Expected RespondStatement, got {other:?}"),
    }
}

#[test]
fn test_respond_with_headers_clause_captures_headers() {
    let stmt = parse_single_statement(
        r#"respond to req with "hello" and content_type "text/plain" and headers response_headers"#,
    );
    match stmt {
        Statement::RespondStatement { headers, .. } => {
            assert!(
                headers.is_some(),
                "Expected the `and headers <map>` clause to be captured"
            );
        }
        other => panic!("Expected RespondStatement, got {other:?}"),
    }
}

#[test]
fn test_respond_with_headers_clause_order_independent() {
    // `headers` may appear before `status`/`content_type`.
    let stmt = parse_single_statement(
        r#"respond to req with "hello" and headers response_headers and status 200 and content_type "text/plain""#,
    );
    match stmt {
        Statement::RespondStatement {
            headers,
            status,
            content_type,
            ..
        } => {
            assert!(headers.is_some(), "headers should parse");
            assert!(status.is_some(), "status should parse");
            assert!(content_type.is_some(), "content_type should parse");
        }
        other => panic!("Expected RespondStatement, got {other:?}"),
    }
}

/// Helper to start a WFL server in a separate thread with its own runtime.
/// Mirrors the harness in `web_server_content_length_test.rs`.
fn start_server_thread(code: String) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let mut parser = Parser::new(&tokens);
            let ast = parser.parse().expect("Failed to parse WFL server code");
            let mut interpreter = Interpreter::new();
            let _ = interpreter.interpret(&ast).await;
        });
    })
}

// End-to-end RFC 10008 flow: a WFL server receives a QUERY request, echoes the
// method it saw as the body, and advertises `Accept-Query` via a custom header
// map. A reqwest client sends the QUERY and verifies both.
#[tokio::test]
async fn test_query_response_sets_custom_headers() {
    let port = 8123;
    let server_code = format!(
        r#"
        create map query_headers:
            "Accept-Query" is "application/jsonpath"
            "Content-Location" is "/data/results/1"
        end map
        listen on port {port} as query_server
        wait for request comes in on query_server as req with timeout 10000
        store seen_method as req["method"]
        respond to req with seen_method and content_type "application/json" and headers query_headers
        close server query_server
    "#
    );

    let server_handle = start_server_thread(server_code);

    // Give the server time to bind.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let response = client
        .request(
            reqwest::Method::from_bytes(b"QUERY").unwrap(),
            format!("http://127.0.0.1:{port}/data"),
        )
        .header("Content-Type", "application/jsonpath")
        .body("$.items[*]")
        .send()
        .await
        .expect("Failed to send QUERY request");

    let status = response.status();

    let accept_query = response
        .headers()
        .get("accept-query")
        .expect("Accept-Query header missing")
        .to_str()
        .expect("Invalid Accept-Query value")
        .to_string();

    let content_location = response
        .headers()
        .get("content-location")
        .expect("Content-Location header missing")
        .to_str()
        .expect("Invalid Content-Location value")
        .to_string();

    let content_type = response
        .headers()
        .get("content-type")
        .expect("Content-Type header missing")
        .to_str()
        .unwrap()
        .to_string();

    let body = response.text().await.expect("Failed to read body");

    assert!(status.is_success(), "QUERY should succeed, got {status}");
    assert_eq!(
        accept_query, "application/jsonpath",
        "Accept-Query header should be set from the response headers map"
    );
    assert_eq!(
        content_location, "/data/results/1",
        "Content-Location header should be set from the response headers map"
    );
    assert_eq!(
        content_type, "application/json",
        "content_type clause should still set Content-Type"
    );
    assert_eq!(
        body, "QUERY",
        "Server should have observed the QUERY method"
    );

    let _ = server_handle.join();
}
