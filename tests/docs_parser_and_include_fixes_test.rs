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
fn shadowing_a_builtin_with_a_non_function_still_errors() {
    // The builtin relaxation only applies to include-injected symbols. A user
    // who shadows a builtin name with a non-function value and then calls it
    // must still get an "is not a function" error rather than a silent pass.
    let out = run_wfl("store touppercase as \"x\"\ndisplay touppercase of \"y\"\n");
    assert!(
        out.contains("is not a function"),
        "shadowed builtin used as a function should still error: {out}"
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

// ---------------------------------------------------------------------------
// #551 — variable bound to a builtin result inside an included file
// ---------------------------------------------------------------------------

#[test]
fn included_action_can_store_builtin_result_in_variable() {
    // `store full as wflhash256 of s` inside an included action must
    // type-check: builtins are injected into the include's analyzer scope as
    // plain parent variables with an Unknown type, which used to abort with
    // "Could not infer type for variable 'full'" (issue #551).
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("mod.wfl"),
        "define action called h with parameters s:\n    store full as wflhash256 of s\n    return full\nend action\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("main.wfl"),
        "include from \"mod.wfl\"\nstore r as call h with \"x\"\ndisplay \"R=\" with r\n",
    )
    .unwrap();

    let out = run_file(&dir, "main.wfl");
    assert!(
        !out.contains("Could not infer type"),
        "builtin result variable must be inferable in an included file: {out}"
    );
    // wflhash256 of "x" is a 64-hex-char digest.
    assert!(
        out.contains("R=") && !out.contains("R=\n"),
        "included action should run and return the hash: {out}"
    );
}

#[test]
fn included_file_can_store_builtin_result_at_top_level() {
    // Same failure mode for a top-level `store` in the included file.
    let dir = TempDir::new().expect("tempdir");
    // The digest is displayed from inside the included file itself: main-file
    // visibility of include-defined variables is a separate concern from the
    // type-inference bug covered here.
    fs::write(
        dir.path().join("mod.wfl"),
        "store digest as wflhash256 of \"a\"\ndisplay \"D=\" with digest\n",
    )
    .unwrap();
    fs::write(dir.path().join("main.wfl"), "include from \"mod.wfl\"\n").unwrap();

    let out = run_file(&dir, "main.wfl");
    assert!(
        !out.contains("Could not infer type"),
        "top-level builtin result variable must be inferable: {out}"
    );
    assert!(out.contains("D="), "included store should run: {out}");
}

#[test]
fn included_action_can_store_parse_json_result() {
    // parse_json had no entry in the builtin return-type table, so even the
    // main-file inference produced "Could not infer type"; in an included
    // file it was fatal (issue #551).
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("mod.wfl"),
        "define action called first_elem with parameters s:\n    store p as parse_json of s\n    return p\nend action\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("main.wfl"),
        "include from \"mod.wfl\"\nstore r as call first_elem with \"[7]\"\ndisplay \"P=\" with r\n",
    )
    .unwrap();

    let out = run_file(&dir, "main.wfl");
    assert!(
        !out.contains("Could not infer type"),
        "parse_json result variable must be inferable in an included file: {out}"
    );
    assert!(out.contains("P=[7]"), "expected 'P=[7]', got: {out}");
}

#[test]
fn parse_json_result_variable_is_inferable_in_main_file() {
    // Guard for the return-type table: parse_json in the main file must not
    // produce a spurious "Could not infer type" diagnostic either.
    let out = run_wfl("store p as parse_json of \"[1]\"\ndisplay p\n");
    assert!(
        !out.contains("Could not infer type"),
        "parse_json result must be inferable in the main file: {out}"
    );
    assert!(out.contains("[1]"), "expected '[1]', got: {out}");
}

// ---------------------------------------------------------------------------
// #553 — remaining RHS forms inside included files: list index, object index,
// comparison results, and `length of`. Each used to abort with a fatal
// "Could not infer type for variable 'v'" when it appeared in an included
// file, even though the same code ran fine in the main file.
// ---------------------------------------------------------------------------

/// Escape a Rust string for interpolation into a WFL double-quoted literal.
fn wfl_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Write `mod.wfl` with an included action whose body is `body`, call it from
/// `main.wfl` with two text arguments, and return the combined output.
fn run_included_action(body: &str, arg1: &str, arg2: &str) -> String {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("mod.wfl"),
        format!("define action called f with parameters a and b:\n{body}\nend action\n"),
    )
    .unwrap();
    let (arg1, arg2) = (wfl_escape(arg1), wfl_escape(arg2));
    fs::write(
        dir.path().join("main.wfl"),
        format!(
            "include from \"mod.wfl\"\nstore r as call f with \"{arg1}\" and \"{arg2}\"\ndisplay \"R=\" with r\n"
        ),
    )
    .unwrap();
    run_file(&dir, "main.wfl")
}

#[test]
fn included_action_can_store_list_index_result() {
    let out = run_included_action(
        "    store parts as string_split of a and \"-\"\n    store v as parts[0]\n    return v",
        "x-y",
        "z",
    );
    assert!(
        !out.contains("Could not infer type"),
        "list-index result must be inferable in an included file: {out}"
    );
    assert!(out.contains("R=x"), "expected 'R=x', got: {out}");
}

#[test]
fn included_action_can_store_object_index_result() {
    let out = run_included_action(
        "    store rec as parse_json of \"{\\\"k\\\":1}\"\n    store v as rec[\"k\"]\n    return v",
        "x",
        "z",
    );
    assert!(
        !out.contains("Could not infer type"),
        "object-index result must be inferable in an included file: {out}"
    );
    assert!(
        !out.contains("Cannot index into"),
        "indexing a parse_json (Any-typed) value must not be a type error: {out}"
    );
    assert!(out.contains("R=1"), "expected 'R=1', got: {out}");
}

#[test]
fn included_action_can_store_comparison_result() {
    let out = run_included_action("    store v as a is equal to b\n    return v", "x", "z");
    assert!(
        !out.contains("Could not infer type"),
        "comparison result must be inferable in an included file: {out}"
    );
    assert!(out.contains("R=no"), "expected 'R=no', got: {out}");
}

#[test]
fn included_action_can_store_ordered_comparison_result() {
    let out = run_included_action(
        "    store v as a is greater than or equal to b\n    return v",
        "b",
        "a",
    );
    assert!(
        !out.contains("Could not infer type"),
        "ordered-comparison result must be inferable in an included file: {out}"
    );
    assert!(out.contains("R=yes"), "expected 'R=yes', got: {out}");
}

#[test]
fn included_action_can_store_length_of_result() {
    let out = run_included_action("    store v as length of a\n    return v", "hello", "z");
    assert!(
        !out.contains("Could not infer type"),
        "`length of` result must be inferable in an included file: {out}"
    );
    assert!(out.contains("R=5"), "expected 'R=5', got: {out}");
}

#[test]
fn list_index_result_variable_is_inferable_in_main_file() {
    // Guard: the same forms must not produce spurious "Could not infer type"
    // warnings in the main file either (they were warnings there, not fatal).
    let out = run_wfl(
        "define action called f with parameters a and b:\n    store parts as string_split of a and \"-\"\n    store v as parts[0]\n    return v\nend action\nstore r as call f with \"x-y\" and \"z\"\ndisplay r\n",
    );
    assert!(
        !out.contains("Could not infer type"),
        "list-index result must be inferable in the main file: {out}"
    );
    assert!(out.contains("x"), "expected 'x', got: {out}");
}

#[test]
fn comparison_result_variable_is_inferable_in_main_file() {
    let out = run_wfl(
        "define action called f with parameters a and b:\n    store v as a is equal to b\n    return v\nend action\nstore r as call f with \"x\" and \"z\"\ndisplay r\n",
    );
    assert!(
        !out.contains("Could not infer type"),
        "comparison result must be inferable in the main file: {out}"
    );
    assert!(out.contains("no"), "expected 'no', got: {out}");
}

#[test]
fn action_local_store_reusing_outer_name_is_a_semantic_error() {
    // WFL forbids shadowing: a body-local `store` that reuses an outer
    // variable's name is rejected by the analyzer with a pointer to
    // `change`. This guards the type checker's scope handling assumption
    // that a name resolving to an outer symbol always refers to that same
    // variable (there is no valid program where it is a distinct local).
    let out = run_wfl(
        "store parts as 5\ndefine action called f with parameters a and b:\n    store parts as string_split of a and \"-\"\n    return parts\nend action\nstore r as call f with \"x-y\" and \"z\"\ndisplay r\n",
    );
    assert!(
        out.contains("already been defined") && out.contains("change parts to"),
        "shadowing store must be rejected with a 'change' suggestion: {out}"
    );
}

#[test]
fn include_type_errors_are_nonfatal_like_main_file() {
    // Deeper guarantee behind #551/#553: the include pipeline must never be
    // stricter than the main-file pipeline. A form the type checker genuinely
    // cannot infer (untyped parameters in arithmetic) is a non-fatal warning
    // in the main file, so an included file must also run, not abort.
    let out = run_included_action("    store v as a plus b\n    return v", "1", "2");
    assert!(
        out.contains("R="),
        "included file with an uninferable store must still run: {out}"
    );
}
