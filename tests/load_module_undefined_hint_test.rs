//! Issue #584 — referencing an action defined in a `load module` file.
//!
//! `load module from "..."` runs a file in an *isolated* child scope; by design
//! it does NOT expose the module's actions/containers/variables to the caller
//! (that is what `include from` is for — see `Docs/04-advanced-features/modules.md`).
//! Empirically, a caller that references a `load module`-defined action fails at
//! *runtime* too, not only in the static analyzer — so relaxing the analyzer to a
//! warning (as the include path does) would be wrong: it would let analysis pass
//! and then crash at runtime.
//!
//! The correct, backward-compatible fix keeps the fatal analyzer error (the
//! program genuinely cannot run) but makes it *actionable*: when a file uses
//! `load module`, an undefined action/variable diagnostic carries a note that
//! points the user at `include from`, the mechanism that actually shares
//! definitions. This suite pins that behavior and guards the boundaries.

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

/// Run a WFL file inside `dir` (so module paths resolve to sibling files),
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

/// Write a `lib_mod.wfl` exposing a one-arg `mod_double` action next to a
/// `main_mod.wfl` whose body is `main_body`, then run `main_mod.wfl`.
fn with_double_module(main_body: &str, extra_args: &[&str]) -> (String, Option<i32>) {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("lib_mod.wfl"),
        "define action called mod_double with parameters n:\n    return n plus n\nend action\n",
    )
    .unwrap();
    fs::write(dir.path().join("main_mod.wfl"), main_body).unwrap();
    let out = run_file_status(&dir, "main_mod.wfl", extra_args);
    drop(dir);
    out
}

// ---------------------------------------------------------------------------
// The exact repro from issue #584 — `load module` + `of`-form call.
// ---------------------------------------------------------------------------

/// The undefined-action error must stay fatal (the call cannot resolve at
/// runtime either), and it must carry an actionable note pointing at
/// `include from`.
#[test]
fn load_module_of_form_is_fatal_with_include_hint() {
    let (out, code) = with_double_module(
        "load module from \"lib_mod.wfl\"\ndisplay mod_double of 5\n",
        &[],
    );
    assert!(
        out.contains("is not defined"),
        "reference to a load-module action must stay fatal: {out}"
    );
    assert!(
        out.contains("include from"),
        "the diagnostic should guide the user to `include from`: {out}"
    );
    assert_eq!(
        code,
        Some(3),
        "should exit 3 (fatal analyze error), not run: {out}"
    );
}

/// Same for the `call ... with` form (which surfaces as "Undefined action").
#[test]
fn load_module_call_form_is_fatal_with_include_hint() {
    let (out, code) = with_double_module(
        "load module from \"lib_mod.wfl\"\nstore d as call mod_double with 5\ndisplay d\n",
        &[],
    );
    assert!(
        out.contains("Undefined action") || out.contains("is not defined"),
        "call-form reference to a load-module action must stay fatal: {out}"
    );
    assert!(
        out.contains("include from"),
        "the diagnostic should guide the user to `include from`: {out}"
    );
    assert_eq!(code, Some(3), "should exit 3: {out}");
}

/// The hint also appears under `--analyze`.
#[test]
fn load_module_hint_under_analyze() {
    let (out, _code) = with_double_module(
        "load module from \"lib_mod.wfl\"\ndisplay mod_double of 5\n",
        &["--analyze"],
    );
    assert!(
        out.contains("include from"),
        "`--analyze` should also surface the `include from` guidance: {out}"
    );
}

// ---------------------------------------------------------------------------
// The documented resolution: `include from` shares the action and runs.
// ---------------------------------------------------------------------------

/// Swapping `load module` for `include from` makes the same program work — this
/// is the path the hint steers users toward.
#[test]
fn include_from_shares_the_action_and_runs() {
    let (out, code) = with_double_module(
        "include from \"lib_mod.wfl\"\ndisplay mod_double of 5\n",
        &[],
    );
    assert!(
        out.contains("10"),
        "include-from should share `mod_double` and print 10: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

// ---------------------------------------------------------------------------
// Guard rails — the hint is specific to `load module` programs.
// ---------------------------------------------------------------------------

/// Without any `load module` (or `include`), an undefined `of` callee stays
/// fatal but must NOT carry the `include from` hint (it would be misleading).
#[test]
fn undefined_without_load_module_has_no_include_hint() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("main.wfl"),
        "display \"BEFORE\"\nstore g as missing_action of \"x\"\ndisplay g\n",
    )
    .unwrap();
    let (out, code) = run_file_status(&dir, "main.wfl", &[]);
    assert!(
        out.contains("is not defined"),
        "undefined callee must stay fatal: {out}"
    );
    assert!(
        !out.contains("include from"),
        "no load module present, so no `include from` hint should appear: {out}"
    );
    assert_eq!(code, Some(3), "should exit 3: {out}");
}
