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

#[test]
fn text_target_one_sided_undefined_classic_lead_is_caught() {
    // Stream lead `value` is defined; classic lead `line value` is not. Target is
    // concrete text, so runtime takes the classic branch — must be a static error
    // (issue #642: previously analysis passed because only the stream lead existed).
    let code = "store value as \"x\"\n\
                write line value to \"/tmp/wfl_onesided_out\"";
    let errors = typecheck(code).expect_err("undefined classic lead must be rejected");
    assert!(
        errors.contains("Variable 'line value' is not defined"),
        "expected the selected classic-lead diagnostic, got: {errors}"
    );
}

#[test]
fn main_loop_stream_binding_rejects_list_payload() {
    // Inside main loop, `out` must be typed as ResponseStream so a list payload
    // is rejected rather than masked by a gradual/file fallback (issue #642).
    let code = "listen on port 8080 as s\n\
                main loop:\n\
                \x20\x20\x20\x20wait for request comes in on s as req\n\
                \x20\x20\x20\x20start streaming response to req with status 200 as out\n\
                \x20\x20\x20\x20store items as [1 and 2]\n\
                \x20\x20\x20\x20store line items as \"legacy\"\n\
                \x20\x20\x20\x20write line items to out\n\
                end loop";
    let errors = typecheck(code).expect_err("list payload must be rejected");
    assert!(
        errors.contains("can only send text, binary, a number, or a boolean"),
        "expected the concrete stream-payload diagnostic, got: {errors}"
    );
}

#[test]
fn property_access_undefined_object_on_text_target_is_caught() {
    // Merged lead with a `.field` postfix: stream reading is `upstream.status`,
    // classic is `line upstream.status`. Neither object is defined; PropertyAccess
    // must not evade definedness on the concrete text-target branch (issue #642).
    let code = "write line upstream.status to \"/tmp/wfl_prop_out\"";
    let errors = typecheck(code).expect_err("undefined property root must be rejected");
    assert!(
        errors.contains("Variable 'line upstream' is not defined"),
        "expected the selected classic property-root diagnostic, got: {errors}"
    );
}

#[test]
fn property_access_checks_the_one_sided_root_for_each_concrete_target() {
    let file_errors = typecheck(
        "store upstream as \"stream-only\"\n\
         write line upstream.status to \"/tmp/wfl_prop_out\"",
    )
    .expect_err("the selected classic property root is undefined");
    assert!(
        file_errors.contains("Variable 'line upstream' is not defined"),
        "text target must diagnose the classic property root, got: {file_errors}"
    );

    let stream_errors = typecheck(
        "store line upstream as \"classic-only\"\n\
         listen on port 8080 as s\n\
         wait for request comes in on s as req\n\
         start streaming response to req with status 200 as out\n\
         write line upstream.status to out",
    )
    .expect_err("the selected stream property root is undefined");
    assert!(
        stream_errors.contains("Variable 'upstream' is not defined"),
        "response-stream target must diagnose the stream property root, got: {stream_errors}"
    );
}

#[test]
fn container_property_is_defined_on_the_concrete_file_branch() {
    // The analyzer recognizes `line value` as a property while it is visiting
    // `Writer`, then restores its own container context before the typechecker
    // revisits the method. Branch-specific definedness must use the
    // typechecker's live container context rather than falsely rejecting the
    // valid classic file-write reading.
    let code = "create container Writer:\n\
                \x20\x20\x20\x20property line value: Text\n\
                \x20\x20\x20\x20action dump:\n\
                \x20\x20\x20\x20\x20\x20\x20\x20write line value to \"C:/tmp/out\"\n\
                \x20\x20\x20\x20end\n\
                end";
    assert!(
        typecheck(code).is_ok(),
        "a container property used by the selected file-write branch must remain defined: {:?}",
        typecheck(code).err()
    );
}

#[test]
fn response_stream_branch_rejects_an_undefined_stream_lead() {
    // Only the classic merged lead (`line value`) exists. Because `out` is a
    // concrete ResponseStream, runtime selects the stream reading (`value`),
    // which must still be rejected as undefined.
    let code = "store line value as \"legacy file payload\"\n\
                listen on port 8080 as s\n\
                wait for request comes in on s as req\n\
                start streaming response to req with status 200 as out\n\
                write line value to out";
    let errors = typecheck(code).expect_err("undefined stream lead must be rejected");
    assert!(
        errors.contains("Variable 'value' is not defined"),
        "expected the selected stream-lead diagnostic, got: {errors}"
    );
}

#[test]
fn action_body_stream_binding_remains_concrete_for_payload_checking() {
    let errors = typecheck(
        "define action called handle with parameters request_value:\n\
             start streaming response to request_value with status 200 as out\n\
             store items as [1 and 2]\n\
             store line items as \"classic\"\n\
             write line items to out\n\
         end action",
    )
    .expect_err("an action-local ResponseStream must reject a List payload");
    assert!(
        errors.contains("can only send text, binary, a number, or a boolean"),
        "expected the action-body stream-payload diagnostic, got: {errors}"
    );
}

#[test]
fn action_local_is_defined_on_the_concrete_file_branch() {
    let code = "define action called dump:\n\
                \x20\x20\x20\x20store line value as \"action-local\"\n\
                \x20\x20\x20\x20write line value to \"C:/tmp/out\"\n\
                end action";
    assert!(
        typecheck(code).is_ok(),
        "an action-local merged lead must remain visible during branch checking: {:?}",
        typecheck(code).err()
    );
}

#[test]
fn main_loop_local_is_defined_on_the_concrete_file_branch() {
    let code = "main loop:\n\
                \x20\x20\x20\x20store line value as \"loop-local\"\n\
                \x20\x20\x20\x20write line value to \"C:/tmp/out\"\n\
                end loop";
    assert!(
        typecheck(code).is_ok(),
        "a main-loop-local merged lead must remain visible during branch checking: {:?}",
        typecheck(code).err()
    );
}

#[test]
fn wrapped_missing_name_is_rejected_on_the_concrete_file_branch() {
    let errors = typecheck(
        "store line value as \"prefix\"\n\
         write line value with file exists at missing_path to \"C:/tmp/wfl_wrapped_out\"",
    )
    .expect_err("the selected classic branch must validate names inside FileExists");
    assert!(
        errors.contains("Variable 'missing_path' is not defined"),
        "expected the wrapped path diagnostic, got: {errors}"
    );
}

#[test]
fn wrapped_missing_name_is_rejected_on_the_concrete_stream_branch() {
    let errors = typecheck(
        "store value as \"prefix\"\n\
         listen on port 8080 as s\n\
         wait for request comes in on s as req\n\
         start streaming response to req with status 200 as out\n\
         write line value with file exists at missing_path to out",
    )
    .expect_err("the selected stream branch must validate names inside FileExists");
    assert!(
        errors.contains("Variable 'missing_path' is not defined"),
        "expected the wrapped path diagnostic, got: {errors}"
    );
}

#[test]
fn wrapped_missing_name_is_rejected_on_every_gradual_branch() {
    let errors = typecheck(
        "define action called send with parameters destination:\n\
             store value as \"stream\"\n\
             store line value as \"classic\"\n\
             write line value with file exists at missing_path to destination\n\
         end action",
    )
    .expect_err("a gradual target must validate wrapped names on every viable branch");
    assert!(
        errors.contains("Variable 'missing_path' is not defined"),
        "expected the wrapped path diagnostic, got: {errors}"
    );
}

#[test]
fn gradual_target_requires_both_candidate_leads_to_be_defined() {
    let undefined_stream_lead = "define action called send with parameters target:\n\
                                 \x20\x20\x20\x20store line value as \"classic\"\n\
                                 \x20\x20\x20\x20write line value to target\n\
                                 end action";
    let stream_errors =
        typecheck(undefined_stream_lead).expect_err("the viable stream lead must be defined");
    assert!(
        stream_errors.contains("Variable 'value' is not defined"),
        "expected the gradual stream-lead diagnostic, got: {stream_errors}"
    );

    let undefined_classic_lead = "define action called send with parameters target:\n\
                                  \x20\x20\x20\x20store value as \"stream\"\n\
                                  \x20\x20\x20\x20write line value to target\n\
                                  end action";
    let classic_errors =
        typecheck(undefined_classic_lead).expect_err("the viable classic lead must be defined");
    assert!(
        classic_errors.contains("Variable 'line value' is not defined"),
        "expected the gradual classic-lead diagnostic, got: {classic_errors}"
    );
}

#[test]
fn gradual_target_validates_every_viable_payload_branch() {
    let invalid_stream_payload = "define action called send with parameters target:\n\
                                  \x20\x20\x20\x20store value as [1 and 2]\n\
                                  \x20\x20\x20\x20store line value as \"classic\"\n\
                                  \x20\x20\x20\x20write line value to target\n\
                                  end action";
    let payload_errors =
        typecheck(invalid_stream_payload).expect_err("the viable stream payload must be valid");
    assert!(
        payload_errors.contains("can only send text, binary, a number, or a boolean"),
        "expected the gradual stream-payload diagnostic, got: {payload_errors}"
    );

    let both_valid = "define action called send with parameters target:\n\
                      \x20\x20\x20\x20store value as \"stream\"\n\
                      \x20\x20\x20\x20store line value as \"classic\"\n\
                      \x20\x20\x20\x20write line value to target\n\
                      end action";
    assert!(
        typecheck(both_valid).is_ok(),
        "both viable gradual branches are valid: {:?}",
        typecheck(both_valid).err()
    );
}
