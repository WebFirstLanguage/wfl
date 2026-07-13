//! Phase 1 (issue #610) — "convert every known correctness defect into an
//! end-to-end regression test".
//!
//! This file is the auditable index of the known correctness defects surfaced
//! during the Phase 1 issue inventory. It has two halves:
//!
//! * **Fixed defects** — a `#[test]` that asserts the *correct* behaviour, so a
//!   regression re-opening the defect turns the suite red. Defects already
//!   covered by a dedicated file are indexed below rather than duplicated; the
//!   new guards here drive the `wfl` binary end-to-end.
//! * **Open defects** — a `#[ignore]`d `#[test]` that asserts the *desired*
//!   behaviour (the fix's acceptance criterion). It is skipped in CI today so
//!   the tree stays green, and is flipped to a passing guard by removing
//!   `#[ignore]` the moment the fix lands. Running `cargo test -- --ignored`
//!   reproduces every open defect on demand (each currently fails).
//!
//! ## Coverage map for the Phase 1 inventory (16 tracked issues + the #610 tracker)
//!
//! | Issue | Class | Status | Regression test |
//! |---|---|---|---|
//! | #582 | Critical (fixed) | ✅ | `github_issues_batch_test.rs::parameter_shadows_same_named_global` |
//! | #557 | High (fixed) | ✅ | `github_issues_batch_test.rs` (date-unit include vars) |
//! | #566 | High (fixed) | ✅ | `github_issues_batch_test.rs` + `route_test.rs` |
//! | #571 | High (fixed) | ✅ | this file: `issue_571_*` (drives the binary) |
//! | #580 | High (fixed) | ✅ | `include_of_form_resolution_test.rs` |
//! | #567 | Medium (fixed) | ✅ | `github_issues_batch_test.rs` (Any/Unknown add/split) |
//! | #569 | Medium (fixed) | ✅ | this file: `issue_569_*` (drives the binary) |
//! | #583 | Medium (fixed) | ✅ | `github_issues_batch_test.rs::bracket_string_stays_text` |
//! | #588 | Medium (fixed) | ✅ | `github_issues_batch_test.rs` (`store x as <call>` Unknown) |
//! | #590 | Medium (fixed) | ✅ | `recursive_action_return_type_test.rs` (focused parser/type-checker guard) |
//! | #592 | **High (open)** | ⏳ | this file: `issue_592_*` (`#[ignore]`, top-level + action-body) |
//! | #578 | **High (open, umbrella)** | ⏳ | this file: `issue_578_*` (`#[ignore]`) — see note below |
//! | #573 | Medium (open) | ⏳ | `web_server_binary_test.rs` guards the byte round-trip; the issue stays open on GitHub (user-facing binary-read keyword + MIME helper incomplete) |
//! | #555 | Medium (open) | ⏳ | `TestPrograms/` `CI-SKIP` corpus (docs-in-CI gate) |
//! | #600 | Post-prod | — | multi-cert SNI enhancement, not a release-gate defect |
//! | #612 | Low | — | PR #609 safe defaults already shipped |
//!
//! ### Note on #578 (umbrella issue)
//!
//! #578 is not a single defect: it collects ~26 checkbox items of varying
//! severity (confirmed functional bugs, footguns, inference gaps, missing
//! forms, ergonomics). The `issue_578_*` tests below cover the **reproducible
//! confirmed functional/silent-wrong-result bugs** re-verified against the
//! release build — they are a representative, not exhaustive, sample. Two
//! caveats found while verifying:
//!
//! * #578's *nested-`for each` over a growing list crashes* item **did not
//!   reproduce** on the current build (the nested loops complete and exit 0), so
//!   it is not encoded as a crash guard — that is why the baseline records "no
//!   *reproducible* crashes/hangs", not "no bugs".
//! * #578's *`X ends with Y` misparse* item is also **no longer reproducible**
//!   (fixed alongside #566).
//!
//! Full per-item classification of the remaining #578 checkboxes is Phase 2
//! scoping work, tracked on the issue itself.
//!
//! Every "fixed" verdict was measured against a fresh `cargo build --release`
//! binary, not inferred from commit history.

use std::fs;
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Absolute path to the `wfl` binary Cargo built for this test run. Under
/// `cargo test --release` (how the integration suite is run) this is the release
/// binary; under a plain `cargo test` it is the debug binary. `CARGO_BIN_EXE_wfl`
/// always points at the freshly-built one, so there is no stale-binary risk.
fn wfl_exe() -> &'static str {
    env!("CARGO_BIN_EXE_wfl")
}

/// Hard wall-clock cap for a single program run. A regression that loops or
/// hangs is killed here instead of consuming the whole job timeout.
const RUN_TIMEOUT: Duration = Duration::from_secs(30);

/// Write `files` (relative `name`, `content`) into a fresh temp dir, run the
/// `entry` program with the temp dir as the working directory (so relative
/// `include from` / `list files in` paths resolve inside it), and return the
/// merged stdout+stderr and the process exit code (`None` if the run was killed
/// on timeout). Pipes are drained on background threads so a chatty program
/// cannot dead-lock on a full pipe buffer, and the child is killed if it exceeds
/// [`RUN_TIMEOUT`].
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
    let mut child = Command::new(wfl_exe())
        .arg(&entry_path)
        .current_dir(dir.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn WFL");

    let mut out_pipe = child.stdout.take().expect("stdout");
    let mut err_pipe = child.stderr.take().expect("stderr");
    let out_thread = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out_pipe.read_to_string(&mut s);
        s
    });
    let err_thread = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err_pipe.read_to_string(&mut s);
        s
    });

    let start = Instant::now();
    let mut killed = false;
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => break status,
            None => {
                if start.elapsed() > RUN_TIMEOUT {
                    let _ = child.kill();
                    killed = true;
                    break child.wait().expect("wait after kill");
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
    };

    let stdout = out_thread.join().unwrap_or_default();
    let stderr = err_thread.join().unwrap_or_default();
    drop(dir);
    let combined = format!("{stdout}{stderr}");
    (combined, if killed { None } else { status.code() })
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
// Natural-language arithmetic must bind tighter than comparison, both `divided
// by` and the `/` symbol lex as division, `modulo` works, and `is between` is a
// real range check. These are silent-wrong-result footguns, so they get a value
// assertion. (Broad `/` coverage also lives in the natural-language
// TestProgram; this is the focused binary-level guard.)
// https://github.com/WebFirstLanguage/wfl/issues/571

#[test]
fn issue_571_precedence_division_modulo_between() {
    let (out, code) = run_src(
        "store a as 2 plus 3 times 4\n\
         display \"A=\" with a\n\
         store b as 10 divided by 4\n\
         display \"B=\" with b\n\
         store c as 10 / 4\n\
         display \"C=\" with c\n\
         store m as 17 modulo 5\n\
         display \"M=\" with m\n\
         check if 5 is between 1 and 10:\n\
        \x20   display \"BETWEEN=yes\"\n\
         end check\n",
    );
    // `times` binds tighter than `plus`: 2 + (3 * 4) = 14, not (2 + 3) * 4 = 20.
    assert!(out.contains("A=14"), "operator precedence (#571): {out}");
    assert!(
        out.contains("B=2.5"),
        "`divided by` must be division (#571): {out}"
    );
    assert!(
        out.contains("C=2.5"),
        "the `/` symbol must lex as division (#571): {out}"
    );
    assert!(out.contains("M=2"), "`modulo` must work (#571): {out}");
    assert!(
        out.contains("BETWEEN=yes"),
        "`is between` must be a range check (#571): {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

// ===========================================================================
// OPEN DEFECTS — assert desired behaviour, `#[ignore]`d until the fix lands.
// Remove `#[ignore]` (and update the linked issue) when the fix lands.
// Run all of them with:
//   cargo test --test phase1_correctness_regression_test -- --ignored
// ===========================================================================

// --- #592 ------------------------------------------------------------------
// A zero-argument include-exposed action referenced by its BARE name (no `of`,
// no `call`) is fatal — both at top level AND inside an action body — with
// `Variable '…' is not defined` (exit 3), while `call greet` and the `of` form
// work. Desired: it resolves like the other call forms in BOTH contexts.
// Parameterized so a fix that only covers one context cannot make this green.
// https://github.com/WebFirstLanguage/wfl/issues/592
//
// CURRENT (26.7.37): fatal `error[ANALYZE-SEMANTIC]: Variable 'greet' is not
// defined`, exit 3, in both contexts.
const MOD_GREET: &str =
    "define action called greet:\n    return \"hello from greet\"\nend action\n";

fn assert_greet_resolves(main_src: &str, context: &str) {
    let (out, code) = run_files(
        &[("mod.wfl", MOD_GREET), ("main.wfl", main_src)],
        "main.wfl",
    );
    assert!(
        out.contains("hello from greet"),
        "bare zero-arg included action must resolve ({context}, #592): {out}"
    );
    assert!(
        !out.contains("is not defined"),
        "must not be a fatal undefined-variable error ({context}, #592): {out}"
    );
    assert_eq!(
        code,
        Some(0),
        "program should exit 0 ({context}, #592): {out}"
    );
}

#[test]
#[ignore = "open defect #592: bare zero-arg included action is fatal at top level"]
fn issue_592_bare_zero_arg_included_action_top_level() {
    assert_greet_resolves(
        "include from \"mod.wfl\"\nstore x as greet\ndisplay x\n",
        "top level",
    );
}

#[test]
#[ignore = "open defect #592: bare zero-arg included action is fatal inside an action body"]
fn issue_592_bare_zero_arg_included_action_in_action_body() {
    assert_greet_resolves(
        "include from \"mod.wfl\"\n\
         define action called run_it:\n    store x as greet\n    return x\nend action\n\
         display run_it\n",
        "action body",
    );
}

// --- #578 (reproducible confirmed functional bugs from the umbrella issue) --
// https://github.com/WebFirstLanguage/wfl/issues/578

// `list files in <dir> with pattern <glob>` drops every match and returns 0.
// CURRENT (26.7.37): COUNT=0 even when matching files exist.
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
// CURRENT (26.7.37): NMATCHES=16.
#[test]
#[ignore = "open defect #578: pattern-VM ignores the `one or more` quantifier"]
fn issue_578_pattern_one_or_more_quantifier() {
    let (out, code) = run_src(
        "create pattern word:\n    one or more letter\nend pattern\n\
         store results as pattern_find_all of \"the quick brown fox\" and word\n\
         display \"NMATCHES=\" with length of results\n",
    );
    assert!(
        out.contains("NMATCHES=4"),
        "`one or more letter` must match 4 whole words, not per-char (#578): {out}"
    );
    // Guard against a false pass where the program errors out before printing.
    assert_eq!(code, Some(0), "program should exit 0 (#578): {out}");
}

// `repeat N times:` is not accepted (it collides with the `times` multiply
// operator), even though it is the most natural counted loop.
// CURRENT (26.7.37): parse error at the numeric literal.
#[test]
#[ignore = "open defect #578: `repeat N times` counted loop is unsupported"]
fn issue_578_repeat_n_times() {
    let (out, code) = run_src("repeat 3 times:\n    display \"hi\"\nend repeat\n");
    let hi_count = out.matches("hi").count();
    assert_eq!(
        hi_count, 3,
        "`repeat 3 times` must run its body 3 times (#578): {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0 (#578): {out}");
}

// `Number plus Text` must be a compile-time type error (the docs promise it),
// not a silent string concatenation.
// CURRENT (26.7.37): prints `25Alice` and exits 0.
#[test]
#[ignore = "open defect #578: `Number plus Text` silently concatenates instead of erroring"]
fn issue_578_number_plus_text_is_a_type_error() {
    let (out, code) = run_src("store age as 25\nstore name as \"Alice\"\ndisplay age plus name\n");
    assert!(
        !out.contains("25Alice"),
        "`Number plus Text` must not silently concatenate (#578): {out}"
    );
    // The strong, non-fragile signal that it was rejected: a non-zero exit.
    assert_ne!(
        code,
        Some(0),
        "`Number plus Text` should be rejected with a non-zero exit (#578): {out}"
    );
}

// There is no text→number conversion, yet the type checker's own hint tells the
// user to "convert to number". `convert <text> to number` does not parse.
// CURRENT (26.7.37): parse error at `to`.
#[test]
#[ignore = "open defect #578: no text→number conversion builtin/syntax"]
fn issue_578_text_to_number_conversion() {
    let (out, code) =
        run_src("store t as \"42\"\nstore n as convert t to number\ndisplay n plus 1\n");
    assert!(
        out.contains("43"),
        "`convert t to number` then +1 must be 43 (#578): {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0 (#578): {out}");
}

// `format_date`/`format_datetime` ignore friendly (`YYYY-MM-DD`) patterns and
// pass them straight to chrono strftime, so only `%Y-%m-%d` works while the
// documented friendly form returns the literal pattern string. Silently wrong
// (exit 0). `current time formatted as "yyyy-MM-dd"` *does* translate, so the
// two paths are inconsistent.
// CURRENT (26.7.37): `format_date of d and "YYYY-MM-DD"` returns "YYYY-MM-DD".
#[test]
#[ignore = "open defect #578: format_date/format_datetime ignore friendly patterns"]
fn issue_578_format_date_friendly_pattern() {
    let (out, code) = run_src(
        "store d as create_date of 2025 and 8 and 9\n\
         display \"R=\" with format_date of d and \"YYYY-MM-DD\"\n",
    );
    assert!(
        out.contains("R=2025-08-09"),
        "friendly `YYYY-MM-DD` pattern must format the date, not echo the literal (#578): {out}"
    );
    assert!(
        !out.contains("R=YYYY-MM-DD"),
        "the literal pattern string must not be returned verbatim (#578): {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0 (#578): {out}");
}
