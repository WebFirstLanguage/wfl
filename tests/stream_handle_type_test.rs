//! Type-contract coverage for stream handles (maintainer review).
//!
//! Stream handles bound by `start streaming response ... as <out>` and
//! `... stream response as <upstream>` are map-shaped objects. `close <handle>`
//! must accept them — before this fix the type checker only accepted a `File`
//! object, so a valid `close out` / `close upstream` produced a spurious
//! "Expected a File object" diagnostic. A concrete scalar must still be rejected.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

fn typecheck(code: &str) -> Result<(), String> {
    let tokens = lex_wfl_with_positions(code);
    let program = Parser::new(&tokens).parse().expect("parse");
    TypeChecker::new()
        .check_types(&program)
        .map_err(|e| format!("{e:?}"))
}

#[test]
fn test_close_server_response_stream_handle_typechecks() {
    // `out` is a streaming-response handle; `close out` must type-check cleanly.
    let code = "listen on port 8080 as s\n\
                wait for request comes in on s as req\n\
                start streaming response to req with status 200 and content type \"text/plain\" as out\n\
                write line \"hi\" to out\n\
                close out";
    assert!(
        typecheck(code).is_ok(),
        "closing a streaming-response handle must not be flagged as a non-File: {:?}",
        typecheck(code).err()
    );
}

#[test]
fn test_close_outbound_stream_handle_typechecks() {
    // `upstream` is an outbound streaming handle; `close upstream` must be clean.
    let code = "open url at \"http://example.com\" and stream response as upstream\n\
                close upstream";
    assert!(
        typecheck(code).is_ok(),
        "closing an outbound stream handle must not be flagged as a non-File: {:?}",
        typecheck(code).err()
    );
}

#[test]
fn test_close_scalar_is_still_rejected() {
    // A concrete non-handle value is still a type error — the fix widens `close`
    // to file/stream handles, it does not make `close` accept anything.
    let code = "store n as 5\nclose n";
    let errors = typecheck(code).expect_err("closing a number must be a type error");
    assert!(
        errors.contains("file or stream handle") || errors.contains("File"),
        "expected a close-operand type error, got: {errors}"
    );
}
