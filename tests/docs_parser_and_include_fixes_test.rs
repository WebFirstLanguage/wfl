//! Regression tests for a batch of reported issues:
//!
//! * #549 — documented/natural forms that the parser rejected:
//!   `substring of X from START length LEN`, `split of X by DELIM`, and
//!   `check if n is not equal to N`.
//! * #547 — actions in an `include from` file could not call stdlib functions
//!   (analyzer reported "'touppercase' is not a function").
//! * #548 — an include-exposed action called from a top-level statement raised
//!   a fatal "Undefined action", aborting the program before any code ran.
//! * #468 — `add ... to X` and `respond ... with X` were not counted as uses of
//!   `X`, producing false ANALYZE-UNUSED warnings.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn wfl_exe() -> &'static str {
    if cfg!(target_os = "windows") {
        "target/release/wfl.exe"
    } else {
        "target/release/wfl"
    }
}

/// Run a single WFL program (written to a temp file) and return combined
/// stdout + stderr.
fn run_wfl(code: &str) -> String {
    let dir = TempDir::new().expect("tempdir");
    let main = dir.path().join("main.wfl");
    fs::write(&main, code).expect("write main.wfl");
    run_file(&dir, "main.wfl")
}

/// Run a WFL file that lives inside `dir` (so `include from` can resolve
/// sibling modules), returning combined stdout + stderr.
fn run_file(dir: &TempDir, name: &str) -> String {
    let path = dir.path().join(name);
    let output = Command::new(wfl_exe())
        .arg(&path)
        .output()
        .expect("Failed to execute WFL");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!("{stdout}{stderr}")
}

/// Run `wfl --analyze <file>` and return combined output.
fn analyze_wfl(code: &str) -> String {
    let dir = TempDir::new().expect("tempdir");
    let main = dir.path().join("main.wfl");
    fs::write(&main, code).expect("write");
    let output = Command::new(wfl_exe())
        .arg("--analyze")
        .arg(&main)
        .output()
        .expect("Failed to execute WFL");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!("{stdout}{stderr}")
}

// ---------------------------------------------------------------------------
// #549.1 — substring ... from ... length ...
// ---------------------------------------------------------------------------

#[test]
fn substring_from_length_form_parses_and_runs() {
    let out = run_wfl("display substring of \"abcdefghij\" from 0 length 5\n");
    assert!(out.contains("abcde"), "expected 'abcde', got: {out}");
    assert!(
        !out.to_lowercase().contains("error"),
        "unexpected error: {out}"
    );
}

#[test]
fn substring_and_form_still_works() {
    // The original spelling must keep working.
    let out = run_wfl("display substring of \"abcdefghij\" and 0 and 5\n");
    assert!(out.contains("abcde"), "expected 'abcde', got: {out}");
}

#[test]
fn substring_length_separator_is_case_insensitive() {
    // `length` as an argument separator should match case-insensitively, like
    // other bareword pseudo-keywords in the parser.
    let out = run_wfl("display substring of \"abcdefghij\" from 0 Length 5\n");
    assert!(out.contains("abcde"), "expected 'abcde', got: {out}");
    assert!(
        !out.to_lowercase().contains("error"),
        "unexpected error: {out}"
    );
}

// ---------------------------------------------------------------------------
// #549.2 — split of X by DELIM
// ---------------------------------------------------------------------------

#[test]
fn split_of_by_form_parses_and_runs() {
    let out = run_wfl("store parts as split of \"a-b-c\" by \"-\"\ndisplay parts\n");
    assert!(out.contains("a") && out.contains("b") && out.contains("c"));
    assert!(
        !out.to_lowercase().contains("error"),
        "unexpected error: {out}"
    );
}

#[test]
fn split_without_of_still_works() {
    let out = run_wfl("store parts as split \"a-b-c\" by \"-\"\ndisplay parts\n");
    assert!(out.contains("a") && out.contains("b") && out.contains("c"));
    assert!(
        !out.to_lowercase().contains("error"),
        "unexpected error: {out}"
    );
}

// ---------------------------------------------------------------------------
// #549.3 — is not equal to
// ---------------------------------------------------------------------------

#[test]
fn is_not_equal_to_true_branch() {
    let out = run_wfl("check if 5 is not equal to 3:\n  display \"neq\"\nend check\n");
    assert!(out.contains("neq"), "expected 'neq', got: {out}");
    assert!(
        !out.to_lowercase().contains("error"),
        "unexpected error: {out}"
    );
}

#[test]
fn is_not_equal_to_false_branch() {
    let out = run_wfl(
        "check if 3 is not equal to 3:\n  display \"neq\"\notherwise:\n  display \"eq\"\nend check\n",
    );
    assert!(out.contains("eq") && !out.contains("neq"), "got: {out}");
}

#[test]
fn is_equal_to_still_works() {
    let out = run_wfl("check if 3 is equal to 3:\n  display \"yes\"\nend check\n");
    assert!(out.contains("yes"), "expected 'yes', got: {out}");
}

// ---------------------------------------------------------------------------
// #547 — included action can call stdlib functions
// ---------------------------------------------------------------------------

#[test]
fn included_action_can_call_stdlib() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("mod.wfl"),
        "define action called shout with parameters s:\n    return touppercase of s\nend action\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("main.wfl"),
        "include from \"mod.wfl\"\nmain loop:\n    store g as call shout with \"bob\"\n    display \"R=\" with g\n    break\nend loop\n",
    )
    .unwrap();

    let out = run_file(&dir, "main.wfl");
    assert!(out.contains("R=BOB"), "expected 'R=BOB', got: {out}");
    assert!(
        !out.contains("is not a function"),
        "should not report stdlib fn as non-function: {out}"
    );
}

// ---------------------------------------------------------------------------
// #548 — include-exposed action callable from a top-level statement
// ---------------------------------------------------------------------------

#[test]
fn included_action_callable_from_top_level() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("mod.wfl"),
        "define action called greet with parameters s:\n    return \"HI-\" with s\nend action\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("main.wfl"),
        "include from \"mod.wfl\"\ndisplay \"BEFORE\"\nstore g as call greet with \"bob\"\ndisplay \"AFTER=\" with g\n",
    )
    .unwrap();

    // The undefined-action check is downgraded to a non-fatal warning when a
    // program uses `include from`, so the program must run to completion even
    // though the analyzer cannot statically see the included action.
    let out = run_file(&dir, "main.wfl");
    assert!(
        out.contains("BEFORE"),
        "BEFORE must print (not aborted): {out}"
    );
    assert!(
        out.contains("AFTER=HI-bob"),
        "expected 'AFTER=HI-bob', got: {out}"
    );
}

#[test]
fn undefined_action_without_include_still_errors() {
    // Guard: the include relaxation must NOT hide genuinely undefined actions
    // in programs that do not use `include from`.
    let out = run_wfl("store g as call totally_missing with \"x\"\ndisplay g\n");
    assert!(
        out.contains("Undefined action"),
        "undefined action should still be reported without includes: {out}"
    );
}

#[test]
fn undefined_action_with_include_is_surfaced_as_warning() {
    // With includes present, a genuinely undefined action (e.g. a typo) is still
    // surfaced — as a non-fatal warning rather than being silently suppressed.
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("mod.wfl"),
        "define action called greet with parameters s:\n    return \"HI-\" with s\nend action\n",
    )
    .unwrap();
    // `grret` is a typo for `greet`.
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        "include from \"mod.wfl\"\nstore g as call grret with \"bob\"\ndisplay g\n",
    )
    .unwrap();

    let output = Command::new(wfl_exe())
        .arg("--analyze")
        .arg(&main)
        .output()
        .expect("run --analyze");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Undefined action") && combined.contains("grret"),
        "typo'd action should be surfaced as a warning even with includes: {combined}"
    );
    // The analyzer must not abort with a fatal error for this case.
    assert!(
        combined.to_lowercase().contains("warning"),
        "should be reported at warning severity: {combined}"
    );
}

// ---------------------------------------------------------------------------
// #468 — add-to / respond uses are counted
// ---------------------------------------------------------------------------

#[test]
fn add_variable_to_counter_counts_as_use() {
    let out = analyze_wfl(
        "store req_count as 0\nstore step as 1\nadd step to req_count\ndisplay req_count\n",
    );
    assert!(
        !out.contains("Unused variable"),
        "no variable should be flagged unused: {out}"
    );
}

#[test]
fn uses_inside_main_loop_are_counted() {
    // Variables used only inside a `main loop` body must be seen as used.
    let out = analyze_wfl(
        "store greeting as \"hello\"\nmain loop:\n    display greeting\n    break\nend loop\n",
    );
    assert!(
        !out.contains("Unused variable"),
        "variable used inside main loop should not be flagged: {out}"
    );
}
