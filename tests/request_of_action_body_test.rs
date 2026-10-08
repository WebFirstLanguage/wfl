//! Issue #647: documented `path`/`method`/`body` `of req` inside action
//! bodies is a fatal ANALYZE-SEMANTIC "not defined", so `wfl` exits 3
//! before the runtime can read those request-object fields. The runtime
//! already treats a one-argument `of` form as a property read on an
//! object; the analyzer must agree.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;
use wfl::analyzer::Analyzer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::{TypeCheckError, TypeChecker};

fn wfl(args: &[&str], path: &Path) -> (Option<i32>, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .args(args)
        .arg(path)
        .env("NO_COLOR", "1")
        .current_dir(path.parent().expect("file has a parent"))
        .output()
        .expect("run WFL");
    (
        output.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

fn analyze_result(source: &str) -> Result<Vec<String>, Vec<String>> {
    let tokens = lex_wfl_with_positions(source);
    let program = Parser::new(&tokens)
        .parse()
        .unwrap_or_else(|errors| panic!("test program must parse: {errors:?}\n{source}"));
    let mut analyzer = Analyzer::new();
    match analyzer.analyze(&program) {
        Ok(()) => Ok(analyzer
            .get_warnings()
            .iter()
            .map(|error| error.message.clone())
            .collect()),
        Err(errors) => Err(errors.into_iter().map(|error| error.message).collect()),
    }
}

fn typecheck_diagnostics(source: &str) -> Result<(), Vec<String>> {
    let tokens = lex_wfl_with_positions(source);
    let program = Parser::new(&tokens)
        .parse()
        .unwrap_or_else(|errors| panic!("test program must parse: {errors:?}\n{source}"));
    match TypeChecker::new().check_types(&program) {
        Ok(()) => Ok(()),
        Err(TypeCheckError::Types(errors)) => {
            Err(errors.into_iter().map(|error| error.message).collect())
        }
        Err(other) => panic!("unexpected type-check failure: {other:?}"),
    }
}

const DOCUMENTED_ACTION: &str = r#"
define action called handle with parameters req:
    store p as path of req
    store m as method of req
    store q as query of req
    store b as body of req
    store bytes as body_bytes of req
    store ua as header "User-Agent" of req
    store reply as m with " " with p with " " with q with " " with ua with " " with b
    respond to req with reply
    store byte_count as length of bytes
    display byte_count
end action

listen on port 8080 as web_server
wait for request comes in on web_server as req
call handle with req
"#;

/// The docs snippet must analyze: `path`/`method`/`body` of `req` inside
/// an action is property access, not an undefined variable.
#[test]
fn documented_path_method_body_of_req_inside_action_analyzes() {
    let result = analyze_result(DOCUMENTED_ACTION);
    assert!(
        result.is_ok(),
        "documented request-property of-form must not be fatal: {result:?}"
    );
    let messages = result.expect("ok");
    assert!(
        messages.iter().all(|message| {
            !message.contains("Variable 'path' is not defined")
                && !message.contains("Variable 'method' is not defined")
                && !message.contains("Variable 'body' is not defined")
        }),
        "must not warn those names as undefined variables: {messages:?}"
    );
}

/// `wfl --analyze` on the documented pattern must exit 0.
#[test]
fn documented_request_of_action_analyze_cli() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("server.wfl");
    fs::write(&path, DOCUMENTED_ACTION).unwrap();

    let (status, output) = wfl(&["--analyze"], &path);
    assert_eq!(
        status,
        Some(0),
        "documented path/method/body of req must analyze: {output}"
    );
    assert!(
        !output.contains("Variable 'path' is not defined")
            && !output.contains("Variable 'method' is not defined")
            && !output.contains("Variable 'body' is not defined"),
        "must not report request properties as undefined: {output}"
    );
}

/// Bare `path` inside an action stays undefined — only the `of` form is
/// request-object access.
#[test]
fn bare_path_inside_action_is_still_undefined() {
    let errors = analyze_result(
        r#"
define action called handle with parameters req:
    store p as path
    respond to req with p
end action
"#,
    )
    .expect_err("bare path in an action must stay fatal");
    assert!(
        errors
            .iter()
            .any(|message| message.contains("Variable 'path' is not defined")),
        "bare path must remain undefined inside actions: {errors:?}"
    );
}

/// An unknown `of` callee inside an action stays fatal.
#[test]
fn unknown_of_form_inside_action_is_still_undefined() {
    let errors = analyze_result(
        r#"
define action called handle with parameters req:
    store p as missing_field of req
    respond to req with p
end action
"#,
    )
    .expect_err("unknown of-form must stay fatal");
    assert!(
        errors.iter().any(|message| {
            message.contains("Variable 'missing_field' is not defined")
                || message.contains("Undefined action 'missing_field'")
        }),
        "unknown of-form must still be reported: {errors:?}"
    );
}

/// Normal type checking (the CLI path, not `--analyze`) must not warn
/// `Cannot call Text` when `wait for request` has bound `path` as Text.
#[test]
fn documented_request_of_action_typechecks_without_warnings() {
    let result = typecheck_diagnostics(DOCUMENTED_ACTION);
    assert!(
        result.is_ok(),
        "documented path/method/body of req must type-check with no warnings: {result:?}"
    );
}

/// `ambiguous_auth_headers` is on the request object; `of req` inside an
/// action must analyze and type-check as Boolean, not as undefined.
#[test]
fn ambiguous_auth_headers_of_req_inside_action() {
    let source = r#"
define action called handle with parameters req:
    store ambiguous as ambiguous_auth_headers of req
    check if ambiguous:
        respond to req with "ambiguous"
    otherwise:
        respond to req with "ok"
    end check
end action

listen on port 8080 as web_server
wait for request comes in on web_server as req
call handle with req
"#;
    let analyzed = analyze_result(source);
    assert!(
        analyzed.is_ok(),
        "ambiguous_auth_headers of req must analyze: {analyzed:?}"
    );
    let typed = typecheck_diagnostics(source);
    assert!(
        typed.is_ok(),
        "ambiguous_auth_headers of req must type-check: {typed:?}"
    );
}
