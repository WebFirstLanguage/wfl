//! Phase 1 (issue #610) — "convert every known correctness defect into an
//! end-to-end regression test".
//!
//! This file is the **single auditable index** of every known correctness
//! defect surfaced during the Phase 1 issue inventory, mapped to an end-to-end
//! test that drives the release `wfl` binary against the defect's own minimal
//! reproduction. It has two halves:
//!
//! * **Fixed defects** — a `#[test]` that asserts the *correct* behaviour, so a
//!   regression re-opening the defect turns the suite red. Defects already
//!   covered by a dedicated file are indexed below rather than duplicated.
//! * **Open defects** — a `#[ignore]`d `#[test]` that asserts the *desired*
//!   behaviour (the fix's acceptance criterion). It is skipped in CI today so
//!   the tree stays green, and is flipped to a passing guard by simply removing
//!   `#[ignore]` the moment Phase 2 lands the fix. Running
//!   `cargo test -- --ignored` reproduces every open defect on demand.
//!
//! ## Coverage map for the Phase 1 inventory (17 issues)
//!
//! | Issue | Class | Status | Regression test |
//! |---|---|---|---|
//! | #582 | Critical (fixed) | ✅ | `github_issues_batch_test.rs::parameter_shadows_same_named_global` |
//! | #557 | High (fixed) | ✅ | `github_issues_batch_test.rs` (date-unit include vars) |
//! | #566 | High (fixed) | ✅ | `github_issues_batch_test.rs` + `route_test.rs` |
//! | #571 | High (fixed) | ✅ | this file: `issue_571_*` |
//! | #580 | High (fixed) | ✅ | `include_of_form_resolution_test.rs` |
//! | #567 | Medium (fixed) | ✅ | `github_issues_batch_test.rs` (Any/Unknown add/split) |
//! | #569 | Medium (fixed) | ✅ | this file: `issue_569_*` |
//! | #583 | Medium (fixed) | ✅ | `github_issues_batch_test.rs::bracket_string_stays_text` |
//! | #588 | Medium (fixed) | ✅ | `github_issues_batch_test.rs` (`store x as <call>` Unknown) |
//! | #590 | Medium (fixed) | ✅ | `recursive_action_return_type_test.rs` |
//! | #592 | **High (open)** | ⏳ | this file: `issue_592_*` (`#[ignore]`) |
//! | #578 | **High (open)** | ⏳ | this file: `issue_578_*` (`#[ignore]`) |
//! | #573 | Medium (open) | ⏳ | `web_server_binary_test.rs` + `binary_file_and_mime_test.wfl` |
//! | #555 | Medium (open) | ⏳ | `TestPrograms/` `CI-SKIP` corpus (docs-in-CI gate) |
//! | #600 | Post-prod | — | multi-cert SNI enhancement, not a release-gate defect |
//! | #612 | Low | — | PR #609 safe defaults already shipped |
//!
//! Every "fixed" verdict was measured against a fresh `cargo build --release`
//! binary, not inferred from commit history.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Absolute path to the `wfl` binary Cargo built for this test run.
fn wfl_exe() -> &'static str {
    env!("CARGO_BIN_EXE_wfl")
}

/// Write `files` (relative `name`, `content`) into a fresh temp dir, run the
/// `entry` program with the temp dir as the working directory (so relative
/// `include from` / `list files in` paths resolve inside it), and return the
/// merged stdout+stderr and the process exit code.
fn run_files(files: &[(&str, &str)], entry: &str) -> (String, Option<i32>) {
    let dir = TempDir::new().expect("tempdir");
    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(&path, content).expect("write file");
    }
    let entry_path = dir.path().join(entry);
    let output = Command::new(wfl_exe())
        .arg(&entry_path)
        .current_dir(dir.path())
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

/// Convenience for single-file programs.
fn run_src(src: &str) -> (String, Option<i32>) {
    run_files(&[("main.wfl", src)], "main.wfl")
}

// ===========================================================================
// FIXED DEFECTS — assert correct behaviour (must stay green)
// ===========================================================================

// --- #569 ------------------------------------------------------------------
// The type checker must infer a user-defined action's return type from its
// `return` expression, so a `call` result used where `Text` is required does
// NOT emit a spurious `error[ERROR]: … Expected Text but found Nothing`.
// https://github.com/WebFirstLanguage/wfl/issues/569

#[test]
fn issue_569_action_call_result_is_text_not_nothing() {
    // `touppercase of <text>` is a strictly Text-typed position; before the fix
    // the action-call result typed as `Nothing` and the type checker screamed.
    let (out, code) = run_src(
        "define action called h with parameters name:\n\
        \x20   store greeting as \"hello \"\n\
        \x20   return greeting with name\n\
         end action\n\
         store c as call h with \"world\"\n\
         store upper as touppercase of c\n\
         display upper\n",
    );
    assert!(out.contains("HELLO WORLD"), "program must run: {out}");
    assert!(
        !out.contains("found Nothing"),
        "action-call result must not be typed Nothing (#569): {out}"
    );
    assert!(
        !out.contains("Expected Text but found"),
        "no spurious Text-mismatch on an action-call result (#569): {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

// --- #571 ------------------------------------------------------------------
// Natural-language arithmetic must bind tighter than comparison, `/` lexes as
// division, `modulo` works, and `is between` is a real range check. These are
// silent-wrong-result footguns, so they get a value assertion.
// https://github.com/WebFirstLanguage/wfl/issues/571

#[test]
fn issue_571_precedence_division_modulo_between() {
    let (out, code) = run_src(
        "store a as 2 plus 3 times 4\n\
         display \"A=\" with a\n\
         store b as 10 divided by 4\n\
         display \"B=\" with b\n\
         store m as 17 modulo 5\n\
         display \"M=\" with m\n\
         check if 5 is between 1 and 10:\n\
        \x20   display \"BETWEEN=yes\"\n\
         end check\n",
    );
    // arithmetic binds tighter than nothing here, but `times` binds tighter than
    // `plus`: 2 + (3 * 4) = 14, not (2 + 3) * 4 = 20.
    assert!(out.contains("A=14"), "operator precedence (#571): {out}");
    assert!(out.contains("B=2.5"), "`/` must be division (#571): {out}");
    assert!(out.contains("M=2"), "`modulo` must work (#571): {out}");
    assert!(
        out.contains("BETWEEN=yes"),
        "`is between` must be a range check (#571): {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

// ===========================================================================
// OPEN DEFECTS — assert desired behaviour, `#[ignore]`d until Phase 2 fixes.
// Remove `#[ignore]` (and update the linked issue) when the fix lands.
// Run all of them with:  cargo test --test phase1_correctness_regression_test -- --ignored
// ===========================================================================

// --- #592 ------------------------------------------------------------------
// A zero-argument include-exposed action referenced by its BARE name (no `of`,
// no `call`) is fatal at top level (`Variable '…' is not defined`, exit 3),
// while `call greet` and the `of` form work. Desired: it resolves like the
// other call forms and prints the return value.
// https://github.com/WebFirstLanguage/wfl/issues/592
//
// CURRENT (26.7.36): fatal `error[ANALYZE-SEMANTIC]: Variable 'greet' is not
// defined`, exit 3.
#[test]
#[ignore = "open defect #592: bare zero-arg include-exposed action is fatal at top level"]
fn issue_592_bare_zero_arg_included_action() {
    let (out, code) = run_files(
        &[
            (
                "mod.wfl",
                "define action called greet:\n    return \"hello from greet\"\nend action\n",
            ),
            ("main.wfl", "include from \"mod.wfl\"\nstore x as greet\ndisplay x\n"),
        ],
        "main.wfl",
    );
    assert!(
        out.contains("hello from greet"),
        "bare zero-arg included action must resolve (#592): {out}"
    );
    assert!(
        !out.contains("is not defined"),
        "must not be a fatal undefined-variable error (#592): {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0 (#592): {out}");
}

// --- #578 (confirmed functional bugs from the umbrella issue) --------------
// Each checkbox in #578 is an independent unit of work; the four below are the
// "wrong result / crash" tier that was re-verified against 26.7.36.
// https://github.com/WebFirstLanguage/wfl/issues/578

// `list files in <dir> with pattern <glob>` drops every match and returns 0.
// CURRENT (26.7.36): COUNT=0 even when matching files exist.
#[test]
#[ignore = "open defect #578: `list files … with pattern` glob path returns 0"]
fn issue_578_list_files_with_pattern_matches() {
    let (out, code) = run_files(
        &[
            ("input/a.txt", "a\n"),
            ("input/b.txt", "b\n"),
            ("input/c.log", "c\n"),
            (
                "main.wfl",
                "store files as list files in \"input\" with pattern \"*.txt\"\n\
                 display \"COUNT=\" with length of files\n",
            ),
        ],
        "main.wfl",
    );
    assert!(
        out.contains("COUNT=2"),
        "glob filter must return the two .txt files (#578): {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0 (#578): {out}");
}

// A `one or more letter` quantifier is ignored: `pattern_find_all` advances one
// character at a time, so a four-word sentence yields 16 single-letter matches
// instead of 4 word matches.
// CURRENT (26.7.36): NMATCHES=16.
#[test]
#[ignore = "open defect #578: pattern-VM ignores the `one or more` quantifier"]
fn issue_578_pattern_one_or_more_quantifier() {
    let (out, _code) = run_src(
        "create pattern word:\n    one or more letter\nend pattern\n\
         store results as pattern_find_all of \"the quick brown fox\" and word\n\
         display \"NMATCHES=\" with length of results\n",
    );
    assert!(
        out.contains("NMATCHES=4"),
        "`one or more letter` must match 4 whole words, not per-char (#578): {out}"
    );
}

// `repeat N times:` is not accepted (it collides with the `times` multiply
// operator), even though it is the most natural counted loop.
// CURRENT (26.7.36): parse error at the numeric literal.
#[test]
#[ignore = "open defect #578: `repeat N times` counted loop is unsupported"]
fn issue_578_repeat_n_times() {
    let (out, code) = run_src("repeat 3 times:\n    display \"hi\"\nend repeat\n");
    let hi_count = out.matches("hi").count();
    assert_eq!(hi_count, 3, "`repeat 3 times` must run its body 3 times (#578): {out}");
    assert_eq!(code, Some(0), "program should exit 0 (#578): {out}");
}

// `Number plus Text` must be a compile-time type error (the docs promise it),
// not a silent string concatenation.
// CURRENT (26.7.36): prints `25Alice` and exits 0.
#[test]
#[ignore = "open defect #578: `Number plus Text` silently concatenates instead of erroring"]
fn issue_578_number_plus_text_is_a_type_error() {
    let (out, code) = run_src(
        "store age as 25\nstore name as \"Alice\"\ndisplay age plus name\n",
    );
    assert!(
        !out.contains("25Alice"),
        "`Number plus Text` must not silently concatenate (#578): {out}"
    );
    assert!(
        out.to_lowercase().contains("type") && code != Some(0),
        "`Number plus Text` should raise a type error and fail (#578): {out}"
    );
}

// There is no text→number conversion, yet the type checker's own hint tells the
// user to "convert to number". `convert <text> to number` does not parse.
// CURRENT (26.7.36): parse error at `to`.
#[test]
#[ignore = "open defect #578: no text→number conversion builtin/syntax"]
fn issue_578_text_to_number_conversion() {
    let (out, code) = run_src(
        "store t as \"42\"\nstore n as convert t to number\ndisplay n plus 1\n",
    );
    assert!(out.contains("43"), "`convert t to number` then +1 must be 43 (#578): {out}");
    assert_eq!(code, Some(0), "program should exit 0 (#578): {out}");
}
