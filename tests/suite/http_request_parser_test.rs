// TDD Tests for the extended outbound HTTP client syntax (issue #558)
//
// New syntax (all clauses optional, order flexible, joined by `with`/`and`):
//   open url at "<url>" with method "POST" and headers h and body b and read response as resp
//
// `read response as` binds the full response object (status/ok/body/headers);
// `read content as` keeps the existing behavior of binding the body text.

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

fn parse_should_fail(code: &str) {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    assert!(parser.parse().is_err(), "Expected parse error for {code:?}");
}

#[test]
fn test_plain_get_still_uses_http_get_statement() {
    // Backward compatibility: the original form must keep parsing as before
    let stmt =
        parse_single_statement(r#"open url at "https://example.com" and read content as content"#);
    match stmt {
        Statement::HttpGetStatement { variable_name, .. } => {
            assert_eq!(variable_name, "content");
        }
        other => panic!("Expected HttpGetStatement, got {other:?}"),
    }
}

#[test]
fn test_plain_get_as_variable_still_uses_http_get_statement() {
    let stmt = parse_single_statement(r#"open url at "https://example.com" as page"#);
    match stmt {
        Statement::HttpGetStatement { variable_name, .. } => {
            assert_eq!(variable_name, "page");
        }
        other => panic!("Expected HttpGetStatement, got {other:?}"),
    }
}

#[test]
fn test_post_with_method_body_and_read_response() {
    let stmt = parse_single_statement(
        r#"open url at "https://api.stripe.com/v1/checkout/sessions" with method "POST" and body "mode=payment" and read response as resp"#,
    );
    match stmt {
        Statement::HttpRequestStatement {
            method,
            headers,
            body,
            variable_name,
            full_response,
            ..
        } => {
            assert!(method.is_some(), "method clause should be captured");
            assert!(headers.is_none(), "no headers clause was given");
            assert!(body.is_some(), "body clause should be captured");
            assert_eq!(variable_name, "resp");
            assert!(full_response, "read response as -> full response object");
        }
        other => panic!("Expected HttpRequestStatement, got {other:?}"),
    }
}

#[test]
fn test_request_with_headers_variable() {
    let stmt = parse_single_statement(
        r#"open url at "https://api.example.com" and headers auth_headers and read response as resp"#,
    );
    match stmt {
        Statement::HttpRequestStatement {
            method,
            headers,
            body,
            full_response,
            ..
        } => {
            assert!(method.is_none());
            assert!(headers.is_some(), "headers clause should be captured");
            assert!(body.is_none());
            assert!(full_response);
        }
        other => panic!("Expected HttpRequestStatement, got {other:?}"),
    }
}

#[test]
fn test_get_with_read_response_uses_request_statement() {
    // Even a plain GET can ask for the full response object
    let stmt =
        parse_single_statement(r#"open url at "https://example.com" and read response as resp"#);
    match stmt {
        Statement::HttpRequestStatement {
            method,
            headers,
            body,
            variable_name,
            full_response,
            ..
        } => {
            assert!(method.is_none());
            assert!(headers.is_none());
            assert!(body.is_none());
            assert_eq!(variable_name, "resp");
            assert!(full_response);
        }
        other => panic!("Expected HttpRequestStatement, got {other:?}"),
    }
}

#[test]
fn test_post_with_read_content_binds_body_text() {
    let stmt = parse_single_statement(
        r#"open url at "https://api.example.com" with method "POST" and body "x=1" and read content as reply"#,
    );
    match stmt {
        Statement::HttpRequestStatement {
            variable_name,
            full_response,
            ..
        } => {
            assert_eq!(variable_name, "reply");
            assert!(!full_response, "read content as -> body text only");
        }
        other => panic!("Expected HttpRequestStatement, got {other:?}"),
    }
}

#[test]
fn test_all_clauses_with_and_connectors() {
    let stmt = parse_single_statement(
        r#"open url at "https://api.example.com" and method "PUT" and headers h and body payload and read response as r"#,
    );
    match stmt {
        Statement::HttpRequestStatement {
            method,
            headers,
            body,
            ..
        } => {
            assert!(method.is_some());
            assert!(headers.is_some());
            assert!(body.is_some());
        }
        other => panic!("Expected HttpRequestStatement, got {other:?}"),
    }
}

#[test]
fn test_body_supports_with_concatenation() {
    // `with` inside the body value builds concatenation, while `and body` /
    // `and read` still terminate the clause correctly.
    let stmt = parse_single_statement(
        r#"open url at "https://api.example.com" with method "POST" and body "amount=" with amount and read response as r"#,
    );
    match stmt {
        Statement::HttpRequestStatement { body, .. } => {
            assert!(body.is_some());
        }
        other => panic!("Expected HttpRequestStatement, got {other:?}"),
    }
}

#[test]
fn test_multiline_request_statement() {
    let code = r#"open url at "https://api.stripe.com/v1/checkout/sessions"
    with method "POST"
    and headers request_headers
    and body "mode=payment"
    and read response as resp"#;
    let stmt = parse_single_statement(code);
    match stmt {
        Statement::HttpRequestStatement {
            method,
            headers,
            body,
            variable_name,
            full_response,
            ..
        } => {
            assert!(method.is_some());
            assert!(headers.is_some());
            assert!(body.is_some());
            assert_eq!(variable_name, "resp");
            assert!(full_response);
        }
        other => panic!("Expected HttpRequestStatement, got {other:?}"),
    }
}

#[test]
fn test_plain_get_missing_terminator_is_an_error() {
    // A statement that ends after the URL (no `as` / `read` clause) must
    // fail even when another statement follows on the next line.
    parse_should_fail("open url at \"https://x.com\"\ndisplay \"next\"");
}

#[test]
fn test_duplicate_method_clause_is_an_error() {
    parse_should_fail(
        r#"open url at "https://x.com" with method "POST" and method "PUT" and read response as r"#,
    );
}

#[test]
fn test_missing_read_clause_is_an_error() {
    parse_should_fail(r#"open url at "https://x.com" with method "POST""#);
}
