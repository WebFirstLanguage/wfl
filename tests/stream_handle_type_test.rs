//! Type-contract coverage for stream handles (maintainer review).
//!
//! Stream handles bound by `start streaming response ... as <out>` and
//! `... stream response as <upstream>` are map-shaped objects. `close <handle>`
//! must accept them — before this fix the type checker only accepted a `File`
//! object, so a valid `close out` / `close upstream` produced a spurious
//! "Expected a File object" diagnostic. A concrete scalar must still be rejected.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, Statement};
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
fn test_close_non_file_custom_handle_is_rejected() {
    // Only a File custom type is closeable via `close`. Other custom handles
    // (a database connection here) are NOT — the interpreter cannot close them,
    // so accepting `close db` would be a false negative.
    let code = "open database at \"sqlite::memory:\" as db\nclose db";
    let errors = typecheck(code).expect_err("closing a database handle must be a type error");
    assert!(
        errors.contains("file or stream handle") || errors.contains("File"),
        "expected a close-operand type error for a database handle, got: {errors}"
    );
}

#[test]
fn test_outbound_stream_handle_fields_are_indexable() {
    // The outbound handle now has a distinct `Custom("HttpStream")` type; reading
    // its fields by index must still type-check (the field type is runtime-known).
    // Mirrors the docs example: a direct field and a nested header lookup.
    let code = "open url at \"http://example.com\" and stream response as resp\n\
                store code as resp[\"status\"]\n\
                store ct as resp[\"headers\"][\"content-type\"]\n\
                close resp";
    assert!(
        typecheck(code).is_ok(),
        "indexing a stream handle's fields must still type-check: {:?}",
        typecheck(code).err()
    );
}

#[test]
fn test_outbound_stream_handle_dot_access_typechecks() {
    // The canonical docs use dot access (`upstream.status`,
    // `upstream.headers["content-type"]`); the distinct handle type must support
    // it, not just bracket notation.
    let code = "open url at \"http://example.com\" and stream response as upstream\n\
                display upstream.status\n\
                store ct as upstream.headers[\"content-type\"]\n\
                close upstream";
    assert!(
        typecheck(code).is_ok(),
        "dot access on a stream handle must type-check: {:?}",
        typecheck(code).err()
    );

    // Type-checking alone is a false green here: even when
    // `upstream.headers["content-type"]` mis-parses into two statements
    // (`store ct as upstream.headers` + a stray `["content-type"]` list literal)
    // both halves type-check. Assert the *structure*: the header lookup binds as
    // one IndexAccess over the PropertyAccess, so the key is not silently dropped.
    let tokens = lex_wfl_with_positions(code);
    let program = Parser::new(&tokens).parse().expect("parse");
    let ct_stmt = program
        .statements
        .iter()
        .find_map(|s| match s {
            Statement::VariableDeclaration { name, value, .. } if name == "ct" => Some(value),
            _ => None,
        })
        .expect("a `store ct as ...` statement");
    assert!(
        matches!(ct_stmt, Expression::IndexAccess { .. }),
        "`upstream.headers[\"content-type\"]` must bind `ct` to an IndexAccess over the \
         PropertyAccess (the lookup must not split off), got {ct_stmt:#?}"
    );
}

#[test]
fn test_stream_handle_numeric_index_is_rejected() {
    // Runtime object indexing requires a text field name; a numeric key must be a
    // static type error (it was, back when the handle was Map<Text, _>).
    let code = "open url at \"http://example.com\" and stream response as resp\n\
                store x as resp[5]\n\
                close resp";
    let errors = typecheck(code).expect_err("a numeric stream-handle key must be a type error");
    assert!(
        errors.contains("field name must be text") || errors.contains("must be text"),
        "expected a text-key type error, got: {errors}"
    );
}

#[test]
fn test_wait_for_next_from_non_stream_is_rejected() {
    // `wait for next chunk|line from <source>` requires an outbound stream handle;
    // reading one from a concrete number is a static type error, not deferred.
    for verb in ["chunk", "line"] {
        let code = format!("store n as 5\nwait for next {verb} from n as c");
        let errors =
            typecheck(&code).expect_err("reading a stream from a number must be a type error");
        assert!(
            errors.contains("stream"),
            "expected a stream-source type error for `wait for next {verb}`, got: {errors}"
        );
    }
}

#[test]
fn test_wait_for_next_from_http_stream_is_ok() {
    // The valid form (reading from an outbound stream handle) must type-check.
    let code = "open url at \"http://example.com\" and stream response as up\n\
                wait for next chunk from up as c\n\
                close up";
    assert!(
        typecheck(code).is_ok(),
        "reading from an outbound stream handle must type-check: {:?}",
        typecheck(code).err()
    );
}

#[test]
fn test_flush_non_stream_is_rejected() {
    // `flush <target>` requires a server response-stream handle.
    let code = "store n as 5\nflush n";
    let errors = typecheck(code).expect_err("flushing a number must be a type error");
    assert!(
        errors.contains("stream"),
        "expected a stream-target type error for `flush`, got: {errors}"
    );
}

#[test]
fn test_write_line_to_non_stream_is_rejected() {
    // An UNAMBIGUOUS stream write (literal value => no classic file-write fallback)
    // to a concrete number cannot be a file write either, so it is a type error.
    let code = "store n as 5\nwrite line \"x\" to n";
    let errors = typecheck(code).expect_err("writing to a number must be a type error");
    assert!(
        errors.contains("stream"),
        "expected a stream-target type error for `write line`, got: {errors}"
    );
}

#[test]
fn test_write_and_flush_response_stream_is_ok() {
    // The valid server-streaming path must type-check cleanly.
    let code = "listen on port 8080 as s\n\
                wait for request comes in on s as req\n\
                start streaming response to req with status 200 and content type \"text/plain\" as out\n\
                write line \"hi\" to out\n\
                flush out\n\
                close out";
    assert!(
        typecheck(code).is_ok(),
        "writing/flushing a response-stream handle must type-check: {:?}",
        typecheck(code).err()
    );
}

#[test]
fn test_write_line_variable_to_file_path_is_ok() {
    // The AMBIGUOUS merged form (`write line <ident> ... to <target>`) carries a
    // classic file-write fallback, so a text file-path target must NOT be rejected.
    let code = "store payload as \"data\"\n\
                write line payload to \"/tmp/out.txt\"";
    assert!(
        typecheck(code).is_ok(),
        "an ambiguous `write line <var> to <file>` must accept a text file target: {:?}",
        typecheck(code).err()
    );
}

#[test]
fn test_close_ordinary_map_is_rejected() {
    // A plain user map is NOT closeable — only file/stream handles are. This is
    // the tightening that a distinct handle type buys over accepting any `Map`.
    let code = "create map m:\n    \"k\" is \"v\"\nend map\nclose m";
    let errors = typecheck(code).expect_err("closing an ordinary map must be a type error");
    assert!(
        errors.contains("file or stream handle") || errors.contains("File"),
        "expected a close-operand type error for a plain map, got: {errors}"
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
