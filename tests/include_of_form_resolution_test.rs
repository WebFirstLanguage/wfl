//! Regression + robustness tests for issue #580 (and the #547-class nested
//! include it shares a root cause with).
//!
//! #548 made an include-exposed action callable through the `call <action> with
//! <arg>` form from a top-level statement (relaxing the fatal "Undefined action"
//! to a non-fatal warning whenever the program uses `include from`). #580 is the
//! same bug surfacing through the *idiomatic* `<action> of <arg>` form, which
//! parses to `Expression::FunctionCall { function: Variable(..), .. }` and never
//! reached #548's relaxation — so it stayed fatal at top level and inside action
//! bodies, while incidentally "working" inside `main loop`/`describe`/`test`
//! (whose bodies the analyzer never descends into).
//!
//! This suite pins the `of` form across every call context, exercises the
//! nested-include case, and guards the boundary so genuinely undefined names are
//! still surfaced (fatal without includes, warning with includes).

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

/// Run a WFL file inside `dir` (so `include from` resolves sibling modules),
/// returning (combined stdout+stderr, exit code).
fn run_file_status(dir: &TempDir, name: &str, extra_args: &[&str]) -> (String, Option<i32>) {
    let path = dir.path().join(name);
    let output = Command::new(wfl_exe())
        .args(extra_args)
        .arg(&path)
        .output()
        .expect("Failed to execute WFL");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (combined, output.status.code())
}

/// Write a `mod.wfl` exposing a one-arg `greet` action next to a `main.wfl`
/// whose body is `main_body`, then run `main.wfl`.
fn with_greet_module(main_body: &str) -> (String, Option<i32>) {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("mod.wfl"),
        "define action called greet with parameters s:\n    return \"HI-\" with s\nend action\n",
    )
    .unwrap();
    fs::write(dir.path().join("main.wfl"), main_body).unwrap();
    let out = run_file_status(&dir, "main.wfl", &[]);
    // keep the tempdir alive until after the run
    drop(dir);
    out
}

/// Assert the run produced no *fatal* undefined-name diagnostic. The relaxed,
/// non-fatal warning's explanatory note reads "This action is not defined in
/// this file ...", so we match only the fatal form `<name>' is not defined`
/// (emitted as `Variable 'greet' is not defined`) to avoid a false positive.
fn assert_no_undefined_fatal(out: &str) {
    assert!(
        !out.contains("' is not defined"),
        "should not report a fatal \"'... is not defined\": {out}"
    );
}

// ---------------------------------------------------------------------------
// #580 — the `of` form must resolve include-exposed actions in every context
// ---------------------------------------------------------------------------

/// The exact repro from the issue: `of` form in a top-level statement.
#[test]
fn of_form_top_level_statement() {
    let (out, code) = with_greet_module(
        "include from \"mod.wfl\"\ndisplay \"BEFORE\"\nstore g as greet of \"bob\"\ndisplay \"AFTER=\" with g\n",
    );
    assert!(out.contains("BEFORE"), "BEFORE should print: {out}");
    assert!(out.contains("AFTER=HI-bob"), "expected AFTER=HI-bob: {out}");
    assert_no_undefined_fatal(&out);
    // The type checker must treat the include-exposed `of` result as Any (issue
    // #580), matching the `call` form — no spurious inference warning.
    assert!(
        !out.contains("Could not infer type"),
        "of-form result should be typed as Any: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

/// `of` form inside a user-defined action body.
#[test]
fn of_form_inside_action_body() {
    let (out, code) = with_greet_module(
        "include from \"mod.wfl\"\n\
         define action called wrap with parameters s:\n    return greet of s\nend action\n\
         store r as wrap of \"bob\"\ndisplay \"R=\" with r\n",
    );
    assert!(out.contains("R=HI-bob"), "expected R=HI-bob: {out}");
    assert_no_undefined_fatal(&out);
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

/// `of` form inside a container action (method) body.
#[test]
fn of_form_inside_container_action_body() {
    let (out, code) = with_greet_module(
        "include from \"mod.wfl\"\n\
         create container Greeter:\n    action run needs s: Text:\n        return greet of s\n    end\nend\n\
         create new Greeter as gtr:\nend\n\
         display \"C=\" with gtr.run(\"bob\")\n",
    );
    assert!(out.contains("C=HI-bob"), "expected C=HI-bob: {out}");
    assert_no_undefined_fatal(&out);
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

/// `of` form inside a `main loop` body (was already non-fatal — regression guard).
#[test]
fn of_form_inside_main_loop() {
    let (out, code) = with_greet_module(
        "include from \"mod.wfl\"\nmain loop:\n    store g as greet of \"bob\"\n    display \"L=\" with g\n    break\nend loop\n",
    );
    assert!(out.contains("L=HI-bob"), "expected L=HI-bob: {out}");
    assert_no_undefined_fatal(&out);
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

/// `of` form inside a `describe`/`test` block, run with `--test`.
#[test]
fn of_form_inside_describe_test() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("mod.wfl"),
        "define action called greet with parameters s:\n    return \"HI-\" with s\nend action\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("main.wfl"),
        "include from \"mod.wfl\"\n\
         describe \"greeting\":\n    test \"greet works\":\n        store g as greet of \"bob\"\n        expect g to equal \"HI-bob\"\n    end test\nend describe\n",
    )
    .unwrap();
    let (out, code) = run_file_status(&dir, "main.wfl", &["--test"]);
    assert_no_undefined_fatal(&out);
    assert!(
        out.contains("Passed: 1") && out.contains("Failed: 0"),
        "the include-exposed `of` test should pass: {out}"
    );
    // The `of` form must not emit the spurious "could not infer type" warning
    // that the `call` form avoids (type checker relaxation, issue #580).
    assert!(
        !out.contains("Could not infer type"),
        "of-form result should be typed as Any, not trigger inference errors: {out}"
    );
    assert_eq!(code, Some(0), "test run should exit 0: {out}");
}

/// Multi-argument `of` form: `render of tmpl and ctx` at top level.
#[test]
fn of_form_multi_argument() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("mod.wfl"),
        "define action called render with parameters tmpl and ctx:\n    return tmpl with \"|\" with ctx\nend action\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("main.wfl"),
        "include from \"mod.wfl\"\nstore out as render of \"T\" and \"C\"\ndisplay \"M=\" with out\n",
    )
    .unwrap();
    let (out, code) = run_file_status(&dir, "main.wfl", &[]);
    assert!(out.contains("M=T|C"), "expected M=T|C: {out}");
    assert_no_undefined_fatal(&out);
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

/// Nested/chained `of`: `greet of (greet of "x")` — the argument is itself an
/// include-exposed `of` call.
#[test]
fn of_form_nested_chained() {
    let (out, code) = with_greet_module(
        "include from \"mod.wfl\"\nstore g as greet of (greet of \"x\")\ndisplay \"N=\" with g\n",
    );
    assert!(out.contains("N=HI-HI-x"), "expected N=HI-HI-x: {out}");
    assert_no_undefined_fatal(&out);
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

// ---------------------------------------------------------------------------
// #547-class — nested includes: a file referencing an action from a file IT
// includes must analyze (in isolation) without a fatal undefined-name error.
// ---------------------------------------------------------------------------

#[test]
fn nested_include_of_form_in_container_action() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("base.wfl"),
        "define action called base_op with parameters x:\n    return \"base(\" with x with \")\"\nend action\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("mid.wfl"),
        "include from \"base.wfl\"\n\
         create container Engine:\n    action run needs x: Text:\n        return base_op of x\n    end\nend\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("app.wfl"),
        "include from \"mid.wfl\"\ncreate new Engine as e:\nend\ndisplay e.run(\"hi\")\n",
    )
    .unwrap();
    let (out, code) = run_file_status(&dir, "app.wfl", &[]);
    assert!(out.contains("base(hi)"), "expected base(hi): {out}");
    assert!(
        !out.contains("Semantic error in included file"),
        "nested include should not fail isolated analysis: {out}"
    );
    assert_no_undefined_fatal(&out);
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

// ---------------------------------------------------------------------------
// Guard rails — the relaxation must not mask genuinely undefined names.
// ---------------------------------------------------------------------------

/// Without any `include from`, an `of` call to an undefined callee stays fatal.
#[test]
fn of_form_undefined_without_include_still_fatal() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("main.wfl"),
        "display \"BEFORE\"\nstore g as missing_action of \"x\"\ndisplay g\n",
    )
    .unwrap();
    let (out, code) = run_file_status(&dir, "main.wfl", &[]);
    assert!(
        out.contains("is not defined"),
        "undefined callee without includes must stay fatal: {out}"
    );
    assert_eq!(code, Some(3), "should exit 3 (fatal analyze error): {out}");
}

/// With an include present, a typo'd `of` callee is surfaced as a non-fatal
/// warning under `--analyze` (not silently dropped, not fatal).
#[test]
fn of_form_typo_with_include_is_warning() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("mod.wfl"),
        "define action called greet with parameters s:\n    return \"HI-\" with s\nend action\n",
    )
    .unwrap();
    // `grret` is a typo for `greet`.
    fs::write(
        dir.path().join("main.wfl"),
        "include from \"mod.wfl\"\nstore g as grret of \"bob\"\ndisplay g\n",
    )
    .unwrap();
    let (out, _code) = run_file_status(&dir, "main.wfl", &["--analyze"]);
    assert!(
        out.contains("Undefined action") && out.contains("grret"),
        "typo'd `of` callee should be surfaced with includes present: {out}"
    );
    assert!(
        out.to_lowercase().contains("warning"),
        "should be reported at warning severity, not fatal: {out}"
    );
}

/// The `call ... with` form (issue #548's fix) still works when `of`-form
/// siblings are present — no regression.
#[test]
fn call_with_form_still_works() {
    let (out, code) = with_greet_module(
        "include from \"mod.wfl\"\nstore g as call greet with \"bob\"\ndisplay \"K=\" with g\n",
    );
    assert!(out.contains("K=HI-bob"), "expected K=HI-bob: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

/// An undefined *argument* to an `of` call (no includes) is still reported — the
/// relaxation must not stop the analyzer from descending into arguments.
#[test]
fn of_form_undefined_argument_without_include_is_reported() {
    // `touppercase` is a real builtin, so the callee resolves; the argument
    // `nope` is undefined and must still be flagged.
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("main.wfl"),
        "store g as touppercase of nope\ndisplay g\n",
    )
    .unwrap();
    let (out, code) = run_file_status(&dir, "main.wfl", &[]);
    assert!(
        out.contains("is not defined") && out.contains("nope"),
        "undefined argument should still be reported: {out}"
    );
    assert_eq!(code, Some(3), "should exit 3: {out}");
}

// ---------------------------------------------------------------------------
// Shaken loose (documented, NOT fixed by #580): the analyzer never descends
// into `main loop` / `describe` / `test` bodies (catch-all `_ => {}`), so a
// genuine typo there is neither an error nor a warning at analyze time. This
// test pins that CURRENT behavior; if a future change adds analyzer coverage of
// those blocks (recommended follow-up), update this expectation deliberately.
// ---------------------------------------------------------------------------

#[test]
fn main_loop_body_is_currently_not_statically_analyzed() {
    // No `include from`: a bare undefined reference inside a main loop body is
    // currently not reported by `--analyze` (the body is skipped entirely).
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("main.wfl"),
        "main loop:\n    display totally_bogus_ref\n    break\nend loop\n",
    )
    .unwrap();
    let (out, _code) = run_file_status(&dir, "main.wfl", &["--analyze"]);
    // Snapshot of today's behavior: the analyzer does not flag the reference.
    assert!(
        !out.contains("totally_bogus_ref"),
        "main-loop bodies are currently not analyzed; update this test if that changes: {out}"
    );
}
