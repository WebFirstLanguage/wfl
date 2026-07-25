//! Backward-compatibility regression (maintainer re-review, P1): a statement-initial
//! `flush <name>` must NOT hijack a pre-existing zero-argument action named
//! `flush <name>`.
//!
//! Before `flush` became a streaming command, `flush cache` was an expression
//! statement that auto-invoked an action `flush cache`. The dispatcher now routes
//! merged `flush …` tokens to the streaming flush; it must still prefer a defined
//! action of that full name so an existing program keeps working.

use std::fs;
use std::process::Command;
use tempfile::TempDir;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

fn run_src(src: &str) -> (String, Option<i32>) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("main.wfl");
    fs::write(&path, src).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .arg(&path)
        .output()
        .expect("failed to execute WFL");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (combined, output.status.code())
}

#[test]
fn flush_calls_a_matching_zero_arg_action_instead_of_flushing_a_stream() {
    // `flush cache` must invoke the action `flush cache`, printing CALLED — not try
    // to flush a (nonexistent) stream `cache`.
    let src = "define action called flush cache:\n\
               \x20\x20\x20\x20display \"CALLED\"\n\
               end action\n\
               \n\
               flush cache\n";
    let (out, code) = run_src(src);
    assert_eq!(
        code,
        Some(0),
        "program should exit cleanly; output was:\n{out}"
    );
    assert!(
        out.contains("CALLED"),
        "the pre-existing `flush cache` action must be called; output was:\n{out}"
    );
    assert!(
        !out.to_lowercase().contains("stream"),
        "`flush cache` must not be reinterpreted as a stream flush; output was:\n{out}"
    );
}

#[test]
fn truly_bare_flush_still_calls_the_legacy_zero_argument_action() {
    let src = "define action called flush:\n\
               \x20\x20\x20\x20display \"CALLED\"\n\
               end action\n\
               \n\
               flush\n";
    let (out, code) = run_src(src);
    assert_eq!(
        code,
        Some(0),
        "the exact bare `flush` action must remain callable; output was:\n{out}"
    );
    assert!(
        out.contains("CALLED"),
        "the exact bare `flush` statement must auto-call its legacy action; output was:\n{out}"
    );
    assert!(
        !out.to_lowercase().contains("stream"),
        "the exact bare `flush` statement must not become a stream operation; output was:\n{out}"
    );
}

#[test]
fn parenthesized_flush_target_keeps_the_exact_zero_argument_action_fallback() {
    let src = "define action called flush:\n\
               \x20\x20\x20\x20display \"PAREN_CALLED\"\n\
               end action\n\
               store ignored as 1\n\
               flush (ignored)\n";
    let (out, code) = run_src(src);
    assert_eq!(
        code,
        Some(0),
        "`flush (expr)` must keep the exact `flush` action fallback; output:\n{out}"
    );
    assert!(
        out.contains("PAREN_CALLED"),
        "the exact zero-argument action must run; output:\n{out}"
    );
}

#[test]
fn explicit_call_flush_target_keeps_the_exact_zero_argument_action_fallback() {
    let src = "define action called flush:\n\
               \x20\x20\x20\x20display \"CALL_CALLED\"\n\
               end action\n\
               define action called acquire stream:\n\
               \x20\x20\x20\x20return 1\n\
               end action\n\
               flush call acquire stream\n";
    let (out, code) = run_src(src);
    assert_eq!(
        code,
        Some(0),
        "`flush call ...` must keep the exact `flush` action fallback; output:\n{out}"
    );
    assert!(
        out.contains("CALL_CALLED"),
        "the exact zero-argument action must run; output:\n{out}"
    );
}

#[test]
fn truly_bare_flush_still_evaluates_a_non_callable_legacy_variable() {
    let src = "store flush as 1\n\
               flush\n\
               display flush\n";
    let (out, code) = run_src(src);
    assert_eq!(
        code,
        Some(0),
        "the exact bare `flush` variable must remain a valid expression statement; output:\n{out}"
    );
    assert!(
        out.contains('1'),
        "the bare legacy variable must remain readable after evaluation; output:\n{out}"
    );
    assert!(
        !out.to_lowercase().contains("stream"),
        "the exact bare `flush` variable must not become a stream operation; output:\n{out}"
    );
}

#[test]
fn flush_without_a_matching_action_still_errors_as_a_stream_flush() {
    // With no action `flush cache` and no stream `cache`, `flush cache` falls
    // through to the stream interpretation and errors (rather than silently
    // succeeding) — proving the action fallback is a preference, not a bypass.
    let src = "flush cache\n";
    let (out, code) = run_src(src);
    assert_ne!(
        code,
        Some(0),
        "a bare `flush cache` with no target must error; output:\n{out}"
    );
}

#[test]
fn flush_non_callable_full_name_binding_is_expression_statement() {
    // Pre-streaming: `store flush cache as 1` then `flush cache` evaluated the
    // variable and completed. Must not try to flush an undefined stream `cache`
    // (issue #642).
    let src = "store flush cache as 1\n\
               flush cache\n\
               display flush cache\n";
    let (out, code) = run_src(src);
    assert_eq!(
        code,
        Some(0),
        "non-callable full-name binding must keep working as an expression statement; output:\n{out}"
    );
    assert!(
        out.contains('1'),
        "expected the bound value to still be readable; output:\n{out}"
    );
    assert!(
        !out.to_lowercase().contains("stream"),
        "`flush cache` must not be reinterpreted as a stream flush; output:\n{out}"
    );
}

#[test]
fn flush_parameterized_single_action_still_arity_errors() {
    // A single parameterized `flush cache` action: old expression-statement
    // auto-call with zero args produced an arity error — not a silent success
    // and not a stream flush (issue #642 re-review).
    let src = "define action called flush cache with parameters x:\n\
               \x20\x20\x20\x20display x\n\
               end action\n\
               \n\
               flush cache\n";
    let (out, code) = run_src(src);
    assert_ne!(
        code,
        Some(0),
        "parameterized flush cache with no args must arity-error; output:\n{out}"
    );
    assert!(
        out.to_lowercase().contains("argument")
            || out.to_lowercase().contains("expected")
            || out.contains("1"),
        "expected an arity error, got:\n{out}"
    );
}

#[test]
fn flush_overloaded_without_zero_arg_is_expression_not_stream() {
    // True overload set with no zero-arg member: bare name evaluates as an
    // overloaded value (expression statement), not a stream flush.
    let src = "define action called flush cache with parameters x:\n\
               \x20\x20\x20\x20display x\n\
               end action\n\
               define action called flush cache with parameters x and y:\n\
               \x20\x20\x20\x20display x\n\
               end action\n\
               \n\
               flush cache\n\
               display \"OK\"\n";
    let (out, code) = run_src(src);
    assert_eq!(
        code,
        Some(0),
        "overloaded flush with no zero-arg overload must not error as a stream flush; output:\n{out}"
    );
    assert!(
        out.contains("OK"),
        "program should reach the trailing display; output:\n{out}"
    );
}

#[test]
fn flush_with_postfix_uses_legacy_expression_when_bound() {
    // `flush cache[0]` must evaluate IndexAccess on Variable("flush cache"), not
    // try to flush stream `cache[0]`.
    let src = "store flush cache as [\"a\" and \"b\"]\n\
               flush cache[0]\n\
               display \"OK\"\n";
    let (out, code) = run_src(src);
    assert_eq!(
        code,
        Some(0),
        "flush cache[0] with a bound list must be an expression statement; output:\n{out}"
    );
    assert!(out.contains("OK"), "expected OK; output:\n{out}");
}

fn typecheck_src(src: &str) -> Result<(), String> {
    let tokens = lex_wfl_with_positions(src);
    let program = Parser::new(&tokens).parse().expect("parse");
    TypeChecker::new()
        .check_types(&program)
        .map_err(|errors| format!("{errors:?}"))
}

#[test]
fn flush_with_nested_postfix_uses_the_recursive_legacy_root() {
    let src = "store flush cache as [[\"a\"]]\n\
               flush cache[0][0]\n\
               display \"OK\"\n";
    let (out, code) = run_src(src);
    assert_eq!(
        code,
        Some(0),
        "nested legacy postfix must resolve the full-name root; output:\n{out}"
    );
    assert!(out.contains("OK"), "expected OK; output:\n{out}");
}

#[test]
fn flush_with_binary_expression_uses_the_legacy_binding() {
    let src = "store flush cache as 1\n\
               flush cache plus 1\n\
               display \"OK\"\n";
    let (out, code) = run_src(src);
    assert_eq!(
        code,
        Some(0),
        "binary legacy expression must resolve the full-name root; output:\n{out}"
    );
    assert!(out.contains("OK"), "expected OK; output:\n{out}");
}

#[test]
fn flush_with_of_call_uses_the_full_legacy_action_name() {
    let src = "define action called flush cache with parameters value:\n\
               \x20\x20\x20\x20display value\n\
               end action\n\
               flush cache of 7\n";
    let (out, code) = run_src(src);
    assert_eq!(
        code,
        Some(0),
        "an `of` continuation must call the full legacy action name; output:\n{out}"
    );
    assert!(
        out.contains('7'),
        "the full-name action should receive its argument; output:\n{out}"
    );
}

#[test]
fn flush_with_post_of_index_preserves_the_legacy_action_result() {
    let src = "define action called flush cache with parameters values:\n\
               \x20\x20\x20\x20return values\n\
               end action\n\
               store items as [7]\n\
               flush cache of (items)[0]\n\
               display \"OK\"\n";
    let (out, code) = run_src(src);
    assert_eq!(
        code,
        Some(0),
        "the indexed result of the full legacy action must remain an expression statement; output:\n{out}"
    );
    assert!(
        out.contains("OK"),
        "the legacy post-`of` expression must complete; output:\n{out}"
    );
    assert!(
        !out.to_lowercase().contains("stream"),
        "the legacy action result must not be reinterpreted as a stream target; output:\n{out}"
    );
}

#[test]
fn flush_split_rewrite_keeps_the_original_legacy_binding() {
    let src = "store flush cache as 1\n\
               flush cache split \"a,b\" by \",\"\n\
               display \"OK\"\n";
    let (out, code) = run_src(src);
    assert_eq!(
        code,
        Some(0),
        "split rewrite must still select the bound legacy expression; output:\n{out}"
    );
    assert!(out.contains("OK"), "expected OK; output:\n{out}");
}

#[test]
fn flush_explicit_find_in_rewrite_keeps_the_original_legacy_binding() {
    let src = "create pattern letter_a:\n\
               \x20\x20\x20\x20\"a\"\n\
               end pattern\n\
               store flush cache as 1\n\
               flush cache find letter_a in \"abc\"\n\
               display \"OK\"\n";
    let (out, code) = run_src(src);
    assert_eq!(
        code,
        Some(0),
        "explicit find-in rewrite must select the bound legacy expression; output:\n{out}"
    );
    assert!(out.contains("OK"), "expected OK; output:\n{out}");
}

#[test]
fn flush_explicit_replace_in_rewrite_keeps_the_original_legacy_binding() {
    let src = "create pattern letter_a:\n\
               \x20\x20\x20\x20\"a\"\n\
               end pattern\n\
               store flush cache as 1\n\
               flush cache replace letter_a with \"z\" in \"abc\"\n\
               display \"OK\"\n";
    let (out, code) = run_src(src);
    assert_eq!(
        code,
        Some(0),
        "explicit replace-in rewrite must select the bound legacy expression; output:\n{out}"
    );
    assert!(out.contains("OK"), "expected OK; output:\n{out}");
}

#[test]
fn flush_direct_container_property_uses_the_legacy_expression_branch() {
    let src = "create container Cache:\n\
               \x20\x20\x20\x20property flush cache: Number\n\
               \x20\x20\x20\x20action inspect:\n\
               \x20\x20\x20\x20\x20\x20\x20\x20flush cache plus 1\n\
               \x20\x20\x20\x20end\n\
               end";
    assert!(
        typecheck_src(src).is_ok(),
        "a direct container property must select the legacy branch: {:?}",
        typecheck_src(src).err()
    );
}

#[test]
fn flush_inherited_container_property_uses_the_legacy_expression_branch() {
    let src = "create container Base:\n\
               \x20\x20\x20\x20property flush cache: Number\n\
               end\n\
               create container Child extends Base:\n\
               \x20\x20\x20\x20action inspect:\n\
               \x20\x20\x20\x20\x20\x20\x20\x20flush cache plus 1\n\
               \x20\x20\x20\x20end\n\
               end";
    assert!(
        typecheck_src(src).is_ok(),
        "an inherited container property must select the legacy branch: {:?}",
        typecheck_src(src).err()
    );
}

#[test]
fn flush_invalid_container_property_expression_is_statically_rejected() {
    let src = "create container Cache:\n\
               \x20\x20\x20\x20property flush cache: Text\n\
               \x20\x20\x20\x20action inspect:\n\
               \x20\x20\x20\x20\x20\x20\x20\x20flush cache minus 1\n\
               \x20\x20\x20\x20end\n\
               end";
    let errors = typecheck_src(src).expect_err("Text minus Number must be rejected");
    assert!(
        !errors.contains("`flush` requires a response-stream handle"),
        "the bound property must be checked as the legacy expression, got: {errors}"
    );
}
