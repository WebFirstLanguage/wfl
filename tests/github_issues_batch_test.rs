//! Regression tests for a batch of GitHub issues fixed together:
//!
//! * #583 — a string whose value is `"[]"` must stay `Text`, not be coerced to
//!   an empty `List`.
//! * #582 — an action parameter must shadow a same-named global instead of the
//!   global overwriting the passed argument.
//! * #566 — `X starts with Y` / `X ends with Y` must work as operators at
//!   statement level (previously swallowed as the multi-word identifier
//!   `"X starts"` / `"X ends"`).
//! * #557 — the date-unit words (`year`, `month`, `day`, `hour`, `minute`,
//!   `second`) used as action-local variables inside an *included* file must
//!   not be a fatal "already defined in an outer scope" error; includes must
//!   behave like the main file (non-fatal, runs).
//! * #567 — `Any`/`Unknown` values (list-index results, untyped parameters)
//!   must be accepted by the `add`/`split`/arithmetic type-checker rules rather
//!   than producing false ERROR-level diagnostics (gradual typing).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Absolute path to the `wfl` binary built for this integration-test run.
/// `CARGO_BIN_EXE_wfl` is injected by Cargo, so it always points at the
/// freshly-built binary regardless of profile or working directory — no stale
/// `target/release` build and no cwd assumption.
fn wfl_exe() -> &'static str {
    env!("CARGO_BIN_EXE_wfl")
}

/// Run inline WFL source in a fresh temp dir, returning (stdout+stderr, exit code).
fn run_src(src: &str) -> (String, Option<i32>) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("main.wfl");
    fs::write(&path, src).unwrap();
    let output = Command::new(wfl_exe())
        .arg(&path)
        .output()
        .expect("failed to execute WFL");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    drop(dir);
    (combined, output.status.code())
}

// ---------------------------------------------------------------------------
// #583 — a quoted "[]" string stays Text
// ---------------------------------------------------------------------------

#[test]
fn bracket_string_stays_text() {
    let (out, code) = run_src(
        "store a as \"[]\"\ndisplay typeof of a\n\
         store b as \"[1, 2]\"\ndisplay typeof of b\n",
    );
    assert!(
        out.contains("Text"),
        "typeof of \"[]\" should be Text: {out}"
    );
    assert!(
        !out.contains("List"),
        "\"[]\" must not be coerced to a List: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn bracket_string_built_at_runtime_stays_text() {
    // `"" with "[" with "" with "]"` builds the two characters "[]" at runtime;
    // it must remain Text just like the literal form.
    let (out, code) =
        run_src("store s as \"\" with \"[\" with \"\" with \"]\"\ndisplay typeof of s\n");
    assert!(
        out.contains("Text"),
        "runtime-built \"[]\" should be Text: {out}"
    );
    assert!(
        !out.contains("List"),
        "runtime \"[]\" must not become a List: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

// ---------------------------------------------------------------------------
// #582 — a parameter shadows a same-named global
// ---------------------------------------------------------------------------

#[test]
fn parameter_shadows_same_named_global() {
    let (out, code) = run_src(
        "define action called takes_p with parameters p:\n    return p\nend action\n\
         store p as \"GLOBAL-VALUE\"\n\
         display \"got: \" with takes_p of \"arg\"\n",
    );
    assert!(
        out.contains("got: arg"),
        "parameter must shadow the global (expected 'got: arg'): {out}"
    );
    assert!(
        !out.contains("GLOBAL-VALUE"),
        "the global must not override the passed argument: {out}"
    );
    // Referencing an untyped parameter must not emit a false type diagnostic
    // (gradual typing — related #567 cleanup).
    assert!(
        !out.contains("Cannot determine type of variable"),
        "untyped parameter reference should not raise a type error: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn multiple_params_shadow_multiple_globals() {
    // Common single-letter helper params (t, v) shadowing globals of the same
    // name — the exact templating-engine failure mode from the issue.
    let (out, code) = run_src(
        "define action called combine with parameters t and v:\n    return t with \"|\" with v\nend action\n\
         store t as \"GT\"\nstore v as \"GV\"\n\
         display \"R=\" with combine of \"a\" and \"b\"\n",
    );
    assert!(
        out.contains("R=a|b"),
        "params t,v must shadow globals: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

// ---------------------------------------------------------------------------
// #566 — `starts with` / `ends with` operators at statement level
// ---------------------------------------------------------------------------

#[test]
fn ends_with_operator_in_check() {
    let (out, code) = run_src(
        "store path as \"/style.css\"\n\
         check if path ends with \".css\":\n    display \"is css\"\notherwise:\n    display \"not css\"\nend check\n",
    );
    assert!(out.contains("is css"), "`ends with` should match: {out}");
    assert!(
        !out.contains("is not defined"),
        "`path ends` must not be read as an identifier: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn starts_with_operator_in_check() {
    let (out, code) = run_src(
        "store path as \"/api/users\"\n\
         check if path starts with \"/api\":\n    display \"api route\"\notherwise:\n    display \"other\"\nend check\n",
    );
    assert!(
        out.contains("api route"),
        "`starts with` should match: {out}"
    );
    assert!(
        !out.contains("is not defined"),
        "`path starts` must parse as operator: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn starts_ends_with_negative_cases() {
    let (out, code) = run_src(
        "store p as \"hello.txt\"\n\
         check if p starts with \"world\":\n    display \"BAD-starts\"\notherwise:\n    display \"ok-starts\"\nend check\n\
         check if p ends with \".md\":\n    display \"BAD-ends\"\notherwise:\n    display \"ok-ends\"\nend check\n",
    );
    assert!(
        out.contains("ok-starts") && out.contains("ok-ends"),
        "negatives should not match: {out}"
    );
    assert!(
        !out.contains("BAD"),
        "non-matching prefix/suffix must be false: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn ends_with_stores_boolean_result() {
    // The desugared form must be a real boolean value usable outside `check if`.
    let (out, code) = run_src(
        "store name as \"report.pdf\"\nstore is_pdf as name ends with \".pdf\"\ndisplay is_pdf\n",
    );
    assert!(
        out.contains("yes") || out.contains("true"),
        "result should be truthy boolean: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

// ---------------------------------------------------------------------------
// #557 — date-unit local variables inside an included file are not fatal
// ---------------------------------------------------------------------------

/// Run `app.wfl` inside a temp dir that also holds `mod.wfl`.
fn run_include(mod_src: &str, app_src: &str) -> (String, Option<i32>) {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("mod.wfl"), mod_src).unwrap();
    fs::write(dir.path().join("app.wfl"), app_src).unwrap();
    let output = Command::new(wfl_exe())
        .arg(dir.path().join("app.wfl"))
        .output()
        .expect("failed to execute WFL");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    drop(dir);
    (combined, output.status.code())
}

#[test]
fn date_unit_local_in_included_file_not_fatal() {
    let (out, code) = run_include(
        "define action called mk with parameters n:\n    store year as n plus 1\n    return year\nend action\n",
        "include from \"mod.wfl\"\n\
         store a as call mk with 1\nstore b as call mk with 2\n\
         display \"R=\" with a with \",\" with b\n",
    );
    assert!(
        out.contains("R=2,3"),
        "included date-unit local must run: {out}"
    );
    assert!(
        !out.contains("has already been defined in an outer scope"),
        "date-unit local in an include must not be fatal (#557): {out}"
    );
    assert!(
        !out.contains("Semantic error in included file"),
        "included file must analyze without a fatal error: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn all_date_unit_words_usable_as_include_locals() {
    // Each of the six singular date-unit words, one per action-local store.
    let mod_src = "\
define action called f1 with parameters n:\n    store year as n\n    return year\nend action\n\
define action called f2 with parameters n:\n    store month as n\n    return month\nend action\n\
define action called f3 with parameters n:\n    store day as n\n    return day\nend action\n\
define action called f4 with parameters n:\n    store hour as n\n    return hour\nend action\n\
define action called f5 with parameters n:\n    store minute as n\n    return minute\nend action\n\
define action called f6 with parameters n:\n    store second as n\n    return second\nend action\n";
    let (out, code) = run_include(
        mod_src,
        "include from \"mod.wfl\"\ndisplay \"OK=\" with f1 of 1 with f6 of 6\n",
    );
    assert!(
        !out.contains("has already been defined in an outer scope"),
        "no date-unit word may be a fatal outer-scope conflict in an include: {out}"
    );
    assert!(out.contains("OK="), "the include must run: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

// ---------------------------------------------------------------------------
// #567 — Any/Unknown values accepted by strict typechecker rules
// ---------------------------------------------------------------------------

/// The type checker prints its ERROR-level diagnostics under this banner; a
/// clean run must not contain it.
const TYPE_WARN_BANNER: &str = "Type checking warnings";

#[test]
fn any_from_list_index_accepted_by_add() {
    let (out, code) = run_src(
        "store rows as [[10 and 20]]\nstore total as 0\n\
         for each r in rows:\n    store t as r[1]\n    add t to total\nend for\ndisplay total\n",
    );
    assert!(out.contains("20"), "program should compute 20: {out}");
    assert!(
        !out.contains("Cannot add non-numeric value to number"),
        "Any-from-index must be accepted by `add ... to` (#567): {out}"
    );
    assert!(
        !out.contains(TYPE_WARN_BANNER),
        "no false type warnings expected: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn any_from_list_index_accepted_by_arithmetic() {
    let (out, code) = run_src(
        "store rows as [[10 and 20]]\n\
         for each r in rows:\n    store t as r[1]\n    store d as 100 minus t\n    display d\nend for\n",
    );
    assert!(out.contains("80"), "program should compute 80: {out}");
    assert!(
        !out.contains("Cannot perform"),
        "Any operand must be accepted by arithmetic (#567): {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn concrete_value_accepted_into_list_of_any() {
    // A list literal is typed `List(Any)`; adding any concrete value must be
    // accepted (a list of statically-unknown element type takes anything) —
    // it must not raise `Cannot add Text to list of Any` (#567).
    let (out, code) = run_src(
        "store xs as [10 and 20]\nadd \"hello\" to xs\nadd 30 to xs\ndisplay length of xs\n",
    );
    assert!(out.contains("4"), "list should have 4 elements: {out}");
    assert!(
        !out.contains("Cannot add"),
        "a concrete value must be accepted into a List(Any) (#567): {out}"
    );
    assert!(
        !out.contains(TYPE_WARN_BANNER),
        "no false type warnings expected: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn unknown_param_accepted_by_split() {
    let (out, code) = run_src(
        "define action called split_words with parameters p:\n    store parts as split p by \" \"\n    return parts\nend action\n\
         store ws as call split_words with \"a b c\"\ndisplay \"done\"\n",
    );
    assert!(out.contains("done"), "program should run: {out}");
    assert!(
        !out.contains("Expected Text for string splitting"),
        "Unknown param must be accepted by `split ... by` (#567): {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}
