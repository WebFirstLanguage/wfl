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
fn flush_overloaded_action_without_zero_arg_is_expression_statement() {
    // An overloaded `flush cache` with only a one-parameter overload used to be
    // an ordinary bare-expression evaluation (no-op success). Must not become a
    // stream error (issue #642).
    let src = "define action called flush cache with parameters x:\n\
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
