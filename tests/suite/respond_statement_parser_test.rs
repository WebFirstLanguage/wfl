// Regression tests for parsing `respond to ... with ... and status ... and
// content_type ...`. The status/content_type values must be parsed as primary
// expressions; previously `and status 404 and content_type "text/plain"`
// parsed the status as the boolean expression `404 and content_type`, which
// failed at runtime (undefined variable `content_type`) and left the HTTP
// request unanswered.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, Literal, Statement};

fn parse_respond(code: &str) -> Statement {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .unwrap_or_else(|e| panic!("Failed to parse {code:?}: {e:?}"));
    program
        .statements
        .into_iter()
        .find(|s| matches!(s, Statement::RespondStatement { .. }))
        .expect("No RespondStatement found")
}

fn assert_integer(expr: &Expression, expected: i64, what: &str) {
    match expr {
        Expression::Literal(Literal::Integer(n), ..) => {
            assert_eq!(*n, expected, "{what} literal mismatch")
        }
        other => panic!("{what} should be an integer literal, got {other:?}"),
    }
}

fn assert_string(expr: &Expression, expected: &str, what: &str) {
    match expr {
        Expression::Literal(Literal::String(s), ..) => {
            assert_eq!(s.as_ref(), expected, "{what} literal mismatch")
        }
        other => panic!("{what} should be a string literal, got {other:?}"),
    }
}

#[test]
fn test_respond_with_status_then_content_type() {
    let stmt = parse_respond(
        r#"respond to req with "Not Found" and status 404 and content_type "text/plain""#,
    );
    match stmt {
        Statement::RespondStatement {
            status,
            content_type,
            ..
        } => {
            assert_integer(&status.expect("status missing"), 404, "status");
            assert_string(
                &content_type.expect("content_type missing"),
                "text/plain",
                "content_type",
            );
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_respond_with_content_type_then_status() {
    let stmt = parse_respond(
        r#"respond to req with "Created" and content_type "application/json" and status 201"#,
    );
    match stmt {
        Statement::RespondStatement {
            status,
            content_type,
            ..
        } => {
            assert_integer(&status.expect("status missing"), 201, "status");
            assert_string(
                &content_type.expect("content_type missing"),
                "application/json",
                "content_type",
            );
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_respond_with_status_only() {
    let stmt = parse_respond(r#"respond to req with "" and status 204"#);
    match stmt {
        Statement::RespondStatement {
            status,
            content_type,
            ..
        } => {
            assert_integer(&status.expect("status missing"), 204, "status");
            assert!(content_type.is_none());
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_respond_with_variable_status() {
    let stmt = parse_respond(
        r#"
store code as 404
respond to req with "Not Found" and status code and content_type "text/plain"
"#,
    );
    match stmt {
        Statement::RespondStatement {
            status,
            content_type,
            ..
        } => {
            assert!(
                matches!(status, Some(Expression::Variable(ref name, ..)) if name == "code"),
                "status should be the variable 'code', got {status:?}"
            );
            assert!(content_type.is_some());
        }
        _ => unreachable!(),
    }
}
