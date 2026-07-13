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
//!   reproduces every open defect **encoded in this file** on demand (each
//!   currently fails).
//!
//! ## The "convert every known correctness defect" task is PARTIAL, not complete
//!
//! Stated plainly so the record can't be read as complete: this suite does
//! **not** finish that Phase 1 task. **#578 is an umbrella** of ~26 checkboxes;
//! only its reproducible confirmed functional/silent-wrong-result bugs are
//! encoded below. Its remaining sub-items (weak inference edges, ergonomics,
//! missing forms) have **no regression test yet** and are tracked on the issue.
//! So the "every known correctness defect" gate is **incomplete** — exhaustive
//! per-item #578 classification remains open Phase 1 work. This is *partial*
//! coverage; it is **not** a redefinition of "every defect" as "every issue".
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
//! | #590 | Medium (fixed) | ✅ | `recursive_action_return_type_test.rs` (in-process type-checker guard) **+** this file: `issue_590_*` (CLI-level end-to-end guard) |
//! | #592 | **High (open)** | ⏳ | this file: `issue_592_*` (`#[ignore]`, top-level + action-body) |
//! | #578 | **High (open, umbrella)** | ⏳ | this file: `issue_578_*` (`#[ignore]`) — see note below |
//! | #573 | Medium (**fixed**) | ✅ | Binary read (`read binary from …`), binary write, lossless byte round-trip, and MIME helpers shipped in #574; guarded by `web_server_binary_test.rs`, `binary_io_test.rs`, and `binary_file_and_mime_test.wfl`. The issue's own latest verification recommends closing; it is open only pending the close click. |
//! | #555 | Medium (open) | ⏳ | `TestPrograms/` `CI-SKIP` corpus (docs-in-CI gate) |
//! | #600 | **High (open, security)** | ⏳ | Not a WFL-program defect: #600's TLS-stack refactor is the vehicle for clearing open high-severity Dependabot alert #49 (`rustls-webpki 0.102.8` DoS via panic, pinned through `warp 0.3.7`→`tokio-rustls 0.25`→`rustls 0.22.4`). No in-line bump exists, so it's tracked as a security/release risk — not code-tested here. |
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
//! Full per-item classification of the remaining #578 checkboxes is **open
//! Phase 1 work** — it is part of the Phase 1 task *"convert every known
//! correctness defect into an end-to-end regression test"* and is tracked on the
//! issue. (*Fixing* those #578 defects is Phase 2; only their classification /
//! regression coverage belongs to Phase 1.)
//!
//! Every "fixed" verdict was measured against a fresh `cargo build --release`
//! binary, not inferred from commit history.

use std::fs;
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Absolute path to the `wfl` binary Cargo built for this test run. It matches
/// the profile the test itself is compiled with: the **debug** binary under a
/// plain `cargo test` / `cargo test --workspace` (what CI's test job runs), or
/// the **release** binary under `cargo test --release`. Either way
/// `CARGO_BIN_EXE_wfl` points at the freshly-built one, so there is no
/// stale-binary risk. (These regression programs are tiny, so the debug/release
/// distinction doesn't affect their outcome.)
fn wfl_exe() -> &'static str {
    env!("CARGO_BIN_EXE_wfl")
}

/// Hard wall-clock cap for a single program run. A regression that loops or
/// hangs is killed here instead of consuming the whole job timeout.
const RUN_TIMEOUT: Duration = Duration::from_secs(30);

/// Write `files` (relative `name`, `content`) into a fresh temp dir, run the
/// `entry` program with the temp dir as the working directory (so relative
/// `include from` / `list files in` paths resolve inside it), and return the
/// captured output — **stdout concatenated with stderr** (both captured in full,
/// but *not* interleaved by time) — and the process exit code (`None` if the run
/// was killed on timeout). Pipes are drained on background threads so a chatty program
/// cannot dead-lock on a full pipe buffer, and the child is killed if it exceeds
/// [`RUN_TIMEOUT`]. Output is drained as raw bytes and decoded with
/// [`String::from_utf8_lossy`], so non-UTF-8 bytes become `U+FFFD` rather than
/// aborting the read (which `read_to_string` would, silently truncating capture).
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
        let mut buf = Vec::new();
        let _ = out_pipe.read_to_end(&mut buf);
        String::from_utf8_lossy(&buf).into_owned()
    });
    let err_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = err_pipe.read_to_end(&mut buf);
        String::from_utf8_lossy(&buf).into_owned()
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

// --- #590 ------------------------------------------------------------------
// A self-recursive action whose result is indexed must not be typed `Nothing`
// (which produced a false `Cannot index into Nothing` and could poison runtime).
// `recursive_action_return_type_test.rs` guards the type-checker in-process;
// this is the CLI-level end-to-end guard the review asked for — it drives the
// binary and asserts the program runs and prints, with no Nothing diagnostic.
// https://github.com/WebFirstLanguage/wfl/issues/590

#[test]
fn issue_590_self_recursive_indexed_result_runs_cli() {
    let (out, code) = run_src(
        "define action called other with parameters n:\n\
        \x20   create map m:\n\
        \x20       \"val\" is n\n\
        \x20   end map\n\
        \x20   return m\n\
         end action\n\n\
         define action called p_unary with parameters n:\n\
        \x20   check if n is greater than 0:\n\
        \x20       store r as p_unary of (n minus 1)\n\
        \x20       return other of (r[\"val\"])\n\
        \x20   end check\n\
        \x20   return other of n\n\
         end action\n\n\
         display \"VAL=\" with (p_unary of 3)[\"val\"]\n",
    );
    assert!(
        !out.contains("Cannot index into Nothing"),
        "self-recursive indexed result must not be typed Nothing (#590): {out}"
    );
    assert!(
        !out.contains("found Nothing"),
        "no spurious Nothing diagnostic on the recursive result (#590): {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0 (#590): {out}");
    // The base case returns `other of 0` → map {"val": 0}; indexing "val" prints 0.
    // Assert the exact labeled marker so unrelated output (other numbers,
    // timestamps, diagnostics) can't accidentally satisfy the guard.
    assert!(
        out.contains("VAL=0"),
        "program must run and print its labeled value VAL=0 (#590): {out}"
    );
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
    // Match only the *fatal* diagnostic form (`Variable 'greet' is not defined`,
    // the `error[ANALYZE-SEMANTIC]` #592 emits), not the bare substring
    // "is not defined" — a benign "This action is not defined in this file …"
    // note must not false-fail this once the fix lands.
    assert!(
        !out.contains("Variable 'greet' is not defined"),
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
        // Invoke run_it with an explicit `call` (not a bare `display run_it`),
        // so the test stays focused on the included-action name resolution
        // inside run_it's body (`store x as greet`) and does not depend on
        // top-level bare-call semantics.
        "include from \"mod.wfl\"\n\
         define action called run_it:\n    store x as greet\n    return x\nend action\n\
         store result as call run_it\n\
         display result\n",
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
    // Count lines that are exactly `hi` (not substring `matches("hi")`, which a
    // diagnostic containing "this"/"which" could inflate).
    let hi_count = out.lines().filter(|line| line.trim() == "hi").count();
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
    // "Rejected" must be policy-agnostic: WFL type errors are *non-fatal* — the
    // type checker prints a "Type checking warnings:" diagnostic and execution
    // still exits 0 (only ExecutionBudget breaches are fatal; see src/main.rs).
    // So accept EITHER a non-zero exit (if a future fix makes it fatal, e.g. a
    // Severity::Error semantic diagnostic → exit 3) OR an explicit type-checker
    // diagnostic on a *completed* run. `code == None` (timeout kill) fails both
    // branches, so a future hang can't pass as green.
    assert!(
        matches!(code, Some(c) if c != 0)
            || (code == Some(0) && out.contains("Type checking warnings:")),
        "`Number plus Text` must be rejected — a non-zero exit OR an explicit type-checker \
         diagnostic (WFL type errors are non-fatal), not a silent concat or a hang/timeout (#578): {out}"
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

// A `with`-form action call (`store r as double with 21`) silently builds the
// string `action double21` and exits 0 instead of calling `double` (the correct
// form is `double of 21`). A common mistake that passes without any error/warning.
// CURRENT (26.7.37): prints `action double21`, exit 0.
//
// (Re-verified with the release binary; the other #578 items the round-2 review
// named — `add` to a `List<Any>` dropping in `--test` mode, and residual
// return-type inference `double of 5 minus 1` → Nothing — did NOT reproduce on
// the current build, so they are not encoded as open defects here.)
#[test]
#[ignore = "open defect #578: `with`-form action call silently concatenates instead of calling"]
fn issue_578_with_form_action_call_is_not_a_silent_concat() {
    let (out, code) = run_src(
        "define action called double with parameters n:\n\
        \x20   return n times 2\n\
         end action\n\
         store r as double with 21\n\
         display r\n",
    );
    assert!(
        !out.contains("action double21") && !out.contains("double21"),
        "`double with 21` must not silently concatenate to a string (#578): {out}"
    );
    // Desired: it either calls `double` (→ prints exactly `42` AND exits 0) or is
    // rejected with an explicit non-zero exit; today it silently concatenates and
    // exits 0, which this guard fails on. Both branches require the run to have
    // *completed*: the success branch pins `code == Some(0)` so "prints 42 then
    // hangs" (`code == None`) can't pass, and the failure branch uses
    // `matches!(code, Some(c) if c != 0)` (not `code != Some(0)`) so a timeout kill
    // (`code == None`) does not count as "rejected". The exact-line match
    // (`line.trim() == "42"`) avoids a stray `42` inside diagnostics passing.
    assert!(
        (code == Some(0) && out.lines().any(|line| line.trim() == "42"))
            || matches!(code, Some(c) if c != 0),
        "`with`-form call must call the action (→ prints `42`, exit 0) or fail with a non-zero exit, not silently concat or hang (#578): {out}"
    );
}
