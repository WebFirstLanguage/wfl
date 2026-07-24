//! Backward-compatibility coverage for the ambiguous `write line|chunk ... to
//! <target>` type check (maintainer re-review, P1).
//!
//! The statement has two readings parsed from the same tokens: a STREAM write of
//! `value` (when the target is a response-stream handle) and a classic FILE write
//! of `fallback_content` (when the target is anything else). The runtime picks by
//! the target's runtime type. The type checker must check the reading the runtime
//! actually takes — checking the stream `value` unconditionally rejected a valid
//! pre-existing file write on a branch that never runs (and the reverse let a
//! broken file write pass because only the stream reading was checked).

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
fn text_target_valid_file_write_is_not_rejected_on_the_stream_branch() {
    // Target is a concrete text path, so the runtime takes the FILE reading:
    // `line value minus n` = 10 - 1 (Number - Number), which is valid. The stream
    // reading `value minus n` would be Text - Number, but the runtime never
    // evaluates it here — so this MUST type-check. (Before the fix it was rejected
    // on the never-run stream branch.)
    let code = "store value as \"wrong stream type\"\n\
                store line value as 10\n\
                store n as 1\n\
                write line value minus n to \"/tmp/wfl_branch_out\"";
    assert!(
        typecheck(code).is_ok(),
        "a valid classic file write must not be rejected on the unused stream reading: {:?}",
        typecheck(code).err()
    );
}

#[test]
fn text_target_broken_file_write_is_caught() {
    // Reverse the types: `line value` is Text, so the FILE reading
    // `line value minus n` = Text - Number is ill-typed. The runtime takes the file
    // reading (target is a concrete text path), so this MUST be a static error.
    // (Before the fix only the stream reading `value minus n` = Number - Number was
    // checked, so this wrongly passed and failed only at runtime.)
    let code = "store value as 10\n\
                store line value as \"text\"\n\
                store n as 1\n\
                write line value minus n to \"/tmp/wfl_branch_out\"";
    assert!(
        typecheck(code).is_err(),
        "a file write whose content is Text minus Number must be a static error, \
         not deferred to runtime"
    );
}

#[test]
fn concrete_non_streamable_payload_to_a_stream_is_rejected() {
    // An unambiguous stream write (the target is a real response-stream handle) of
    // a concrete List payload must be a static error — the runtime only accepts
    // text/binary/number/boolean, so a Map/List/Nothing reaching `write` fails at
    // runtime otherwise.
    let code = "listen on port 8080 as s\n\
                wait for request comes in on s as req\n\
                start streaming response to req with status 200 and content type \"text/plain\" as out\n\
                store items as [1 and 2 and 3]\n\
                write line items to out";
    assert!(
        typecheck(code).is_err(),
        "writing a concrete List to a response stream must be a static type error"
    );
}

#[test]
fn text_and_binary_payloads_to_a_stream_still_typecheck() {
    // The valid stream payloads must keep passing.
    let code = "listen on port 8080 as s\n\
                wait for request comes in on s as req\n\
                start streaming response to req with status 200 and content type \"text/plain\" as out\n\
                write line \"hello\" to out\n\
                write chunk 42 to out";
    assert!(
        typecheck(code).is_ok(),
        "text/number stream payloads must type-check: {:?}",
        typecheck(code).err()
    );
}
