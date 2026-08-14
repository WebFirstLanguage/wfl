//! Regression tests for a batch of GitHub issues fixed together:
//!
//! * #698 — `replace <pattern> with <text> in <text>` silently returned the
//!   input unchanged (exit 0, no diagnostic), and no literal string
//!   replacement was reachable from WFL source at all.
//! * #699 — the `count` loop's iteration cap was keyed on the loop's *end
//!   value* rather than on the trip count, so `count from 1 to 20000` aborted
//!   while `count from 1 to 1000001` ran uncapped.
//! * #700 — `exit program` (the form in the keyword reference) failed to
//!   parse, and no spelling terminated the program.
//!
//! These run the real `wfl` binary so exit status is part of what is asserted
//! — a silently-wrong answer with status 0 is exactly the failure mode #698
//! and #700 describe.

mod common;
use common::{run_file_status, run_src};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// #698 — replace actually replaces
// ---------------------------------------------------------------------------

#[test]
fn pattern_replace_replaces_every_match() {
    let (out, code) = run_src(
        "create pattern w:\n    \"world\"\nend pattern\n\
         store s as \"hello world world\"\n\
         display \"[\" with (replace w with \"there\" in s) with \"]\"\n",
    );
    assert!(
        out.contains("[hello there there]"),
        "every match must be replaced: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn pattern_replace_leaves_a_non_matching_text_alone() {
    let (out, code) = run_src(
        "create pattern w:\n    \"absent\"\nend pattern\n\
         store s as \"hello world\"\n\
         display \"[\" with (replace w with \"x\" in s) with \"]\"\n",
    );
    assert!(
        out.contains("[hello world]"),
        "a pattern that never matches leaves the text unchanged: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn pattern_replace_is_character_correct_on_multibyte_text() {
    // The replacement walks character offsets from the pattern VM; multibyte
    // input must not be sliced on a byte boundary (which would panic) nor
    // replaced at the wrong offset.
    let (out, code) = run_src(
        "create pattern p:\n    \"é\"\nend pattern\n\
         store s as \"aébécé\"\n\
         display \"[\" with (replace p with \"E\" in s) with \"]\"\n",
    );
    assert!(
        out.contains("[aEbEcE]"),
        "multibyte text must be replaced at character offsets: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn pattern_replace_replaces_whole_multi_character_matches() {
    // The whole match is what gets replaced, not its first character, and the
    // scan resumes after it — so the trailing lone digit is left alone.
    let (out, code) = run_src(
        "create pattern two_digits:\n    exactly 2 digit\nend pattern\n\
         store s as \"a12b34c5\"\n\
         display \"[\" with (replace two_digits with \"#\" in s) with \"]\"\n",
    );
    assert!(
        out.contains("[a#b#c5]"),
        "each whole match is replaced exactly once: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn literal_text_replace_is_reachable_from_wfl_source() {
    // #698's compounding half: `replace` lexes as a keyword, so the 3-argument
    // native `replace` could never be called. A literal needle in the pattern
    // slot is the reachable spelling.
    let (out, code) = run_src(
        "store s as \"hello world world\"\n\
         display \"[\" with (replace \"world\" with \"there\" in s) with \"]\"\n",
    );
    assert!(
        out.contains("[hello there there]"),
        "a literal text needle must replace every occurrence: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn literal_text_replace_treats_the_needle_as_text_not_a_pattern() {
    // A literal needle is compared literally: pattern metacharacters in it are
    // just characters.
    let (out, code) = run_src(
        "store s as \"a.b.c\"\n\
         display \"[\" with (replace \".\" with \"-\" in s) with \"]\"\n",
    );
    assert!(
        out.contains("[a-b-c]"),
        "the needle is matched literally: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn replace_rejects_a_needle_that_is_neither_text_nor_pattern() {
    let (out, code) = run_src(
        "store s as \"hello\"\n\
         store n as 5\n\
         display replace n with \"x\" in s\n",
    );
    assert_ne!(code, Some(0), "a number needle must not be accepted: {out}");
}

// ---------------------------------------------------------------------------
// #699 — the count loop runs the iterations it was asked for
// ---------------------------------------------------------------------------

#[test]
fn count_loop_runs_twenty_thousand_iterations() {
    let (out, code) = run_src(
        "store total as 0\n\
         count from 1 to 20000:\n    change total to total plus 1\nend count\n\
         display \"total: \" with total\n",
    );
    assert!(out.contains("total: 20000"), "expected 20000 trips: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn count_loop_runs_just_past_the_old_cap() {
    let (out, code) = run_src(
        "store total as 0\n\
         count from 1 to 10001:\n    change total to total plus 1\nend count\n\
         display \"total: \" with total\n",
    );
    assert!(out.contains("total: 10001"), "expected 10001 trips: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn downward_count_loop_is_not_capped_by_its_end_value() {
    // The old guard read `end_num`, so a loop counting *down* to 1 was capped
    // at 10001 trips however many it actually needed.
    let (out, code) = run_src(
        "store total as 0\n\
         count from 20000 down to 1:\n    change total to total plus 1\nend count\n\
         display \"total: \" with total\n",
    );
    assert!(out.contains("total: 20000"), "expected 20000 trips: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn count_loop_with_a_step_still_runs_every_trip() {
    let (out, code) = run_src(
        "store total as 0\n\
         count from 1 to 40000 by 2:\n    change total to total plus 1\nend count\n\
         display \"total: \" with total\n",
    );
    assert!(out.contains("total: 20000"), "expected 20000 trips: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn count_loop_rejects_a_step_that_never_reaches_the_end() {
    // Removing the trip cap must not turn a typo into a 60-second hang: a step
    // that cannot advance the counter toward the end value is refused up front
    // with a diagnostic that names the problem.
    let (out, code) = run_src("count from 1 to 10 by 0:\n    display \"tick\"\nend count\n");
    assert_ne!(code, Some(0), "a zero step must be an error: {out}");
    assert!(
        out.to_lowercase().contains("step"),
        "the diagnostic must name the step: {out}"
    );
    assert!(
        !out.contains("tick"),
        "the loop body must not run at all: {out}"
    );
}

// ---------------------------------------------------------------------------
// #700 — `exit program` parses and terminates
// ---------------------------------------------------------------------------

#[test]
fn exit_program_stops_the_program_at_top_level() {
    let (out, code) = run_src("display \"before\"\nexit program\ndisplay \"after\"\n");
    assert!(
        out.contains("before"),
        "statements before it still run: {out}"
    );
    assert!(!out.contains("after"), "nothing after it runs: {out}");
    assert_eq!(code, Some(0), "a normal exit is status 0: {out}");
}

#[test]
fn exit_program_stops_the_program_from_inside_a_loop() {
    let (out, code) = run_src(
        "count from 1 to 5:\n\
         \x20   display \"tick \" with count\n\
         \x20   check if count is equal to 2:\n\
         \x20       exit program\n\
         \x20   end check\n\
         end count\n\
         display \"after\"\n",
    );
    assert!(out.contains("tick 1"), "the loop starts: {out}");
    assert!(out.contains("tick 2"), "the deciding trip runs: {out}");
    assert!(!out.contains("tick 3"), "later trips do not run: {out}");
    assert!(
        !out.contains("after"),
        "code after the loop does not run: {out}"
    );
    assert_eq!(code, Some(0), "a normal exit is status 0: {out}");
}

#[test]
fn exit_program_stops_the_program_from_inside_an_action() {
    let (out, code) = run_src(
        "define action called bail:\n\
         \x20   display \"bailing\"\n\
         \x20   exit program\n\
         end action\n\
         display \"before\"\n\
         call bail\n\
         display \"after\"\n",
    );
    assert!(
        out.contains("before"),
        "statements before it still run: {out}"
    );
    assert!(out.contains("bailing"), "the action body runs: {out}");
    assert!(!out.contains("after"), "the caller does not resume: {out}");
    assert_eq!(code, Some(0), "a normal exit is status 0: {out}");
}

#[test]
fn exit_program_is_not_caught_by_error_handling() {
    // Program termination is not a failure, so a `when error` handler must not
    // swallow it and carry on.
    let (out, code) = run_src(
        "try:\n\
         \x20   display \"trying\"\n\
         \x20   exit program\n\
         when error:\n\
         \x20   display \"caught\"\n\
         end try\n\
         display \"after\"\n",
    );
    assert!(out.contains("trying"), "the try block runs: {out}");
    assert!(!out.contains("caught"), "the handler must not run: {out}");
    assert!(!out.contains("after"), "nothing after the try runs: {out}");
    assert_eq!(code, Some(0), "a normal exit is status 0: {out}");
}

#[test]
fn exit_program_does_not_leak_an_error_message() {
    let (out, code) = run_src("display \"before\"\nexit program\n");
    assert!(
        !out.to_lowercase().contains("error"),
        "a clean exit prints no diagnostic: {out}"
    );
    assert_eq!(code, Some(0), "a normal exit is status 0: {out}");
}

#[test]
fn exit_program_skips_the_main_action() {
    // `main` runs after the top-level statements; terminating the program must
    // cancel that too.
    let (out, code) = run_src(
        "define action called main:\n\
         \x20   display \"main ran\"\n\
         end action\n\
         display \"before\"\n\
         exit program\n",
    );
    assert!(out.contains("before"), "top-level statements run: {out}");
    assert!(!out.contains("main ran"), "main must not run: {out}");
    assert_eq!(code, Some(0), "a normal exit is status 0: {out}");
}

#[test]
fn bare_exit_still_leaves_the_loop_and_keeps_going() {
    // Backward compatibility: bare `exit` / `exit loop` are the loop-exit
    // spelling and keep their existing meaning.
    let (out, code) = run_src(
        "count from 1 to 5:\n\
         \x20   display \"tick \" with count\n\
         \x20   check if count is equal to 2:\n\
         \x20       exit loop\n\
         \x20   end check\n\
         end count\n\
         display \"after\"\n",
    );
    assert!(out.contains("tick 2"), "the deciding trip runs: {out}");
    assert!(!out.contains("tick 3"), "the loop stops: {out}");
    assert!(out.contains("after"), "the program continues: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

// ---------------------------------------------------------------------------
// Follow-ups raised in review of the fix itself
// ---------------------------------------------------------------------------

#[test]
fn count_loop_with_an_empty_range_ignores_its_step() {
    // Backward compatibility: the step only matters if the loop is entered.
    // `count from 5 to 1` never runs its body, so a nonsense step was — and
    // must remain — harmless. Validating it unconditionally turned a working
    // program into a runtime error.
    let (out, code) = run_src(
        "count from 5 to 1 by 0:\n\
         \x20   display \"never\"\n\
         end count\n\
         display \"done\"\n",
    );
    assert!(!out.contains("never"), "the body must not run: {out}");
    assert!(out.contains("done"), "the program continues: {out}");
    assert_eq!(code, Some(0), "an empty range is not an error: {out}");
}

#[test]
fn count_loop_with_an_empty_downward_range_ignores_its_step() {
    let (out, code) = run_src(
        "count from 1 down to 5 by 0:\n\
         \x20   display \"never\"\n\
         end count\n\
         display \"done\"\n",
    );
    assert!(!out.contains("never"), "the body must not run: {out}");
    assert!(out.contains("done"), "the program continues: {out}");
    assert_eq!(code, Some(0), "an empty range is not an error: {out}");
}

#[test]
fn count_loop_rejects_a_step_too_small_to_advance_the_counter() {
    // A positive, finite step is not enough: below the counter's floating-point
    // resolution `count + step == count`, so the loop can never reach its end.
    // 100000000000000000 plus 1 is still 100000000000000000.
    let started = std::time::Instant::now();
    let (out, code) = run_src(
        "count from 100000000000000000 to 100000000000000100 by 1:\n\
         \x20   display \"tick\"\n\
         end count\n",
    );
    let elapsed = started.elapsed();
    assert_ne!(code, Some(0), "a non-advancing step must be refused: {out}");
    assert!(
        out.to_lowercase().contains("step"),
        "the diagnostic must name the step: {out}"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(20),
        "the loop must be refused up front, not spin until the timeout \
         (took {elapsed:?}): {out}"
    );
}

#[test]
fn exit_program_inside_a_test_block_stops_the_run_cleanly() {
    // A test block records a runtime error in its body as a test failure. The
    // stop sentinel is not a failure: it must pass straight through, leaving a
    // clean exit rather than a bogus failing test.
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(
        dir.path().join("main.wfl"),
        "describe \"stopping\":\n\
         \x20   test \"stops the run\":\n\
         \x20       display \"inside the test\"\n\
         \x20       exit program\n\
         \x20   end test\n\
         \x20   test \"must not run\":\n\
         \x20       display \"later test ran\"\n\
         \x20   end test\n\
         end describe\n",
    )
    .expect("write program");

    let (out, code) = run_file_status(&dir, "main.wfl", &["--test"]);
    assert!(out.contains("inside the test"), "the test body runs: {out}");
    assert!(
        !out.contains("later test ran"),
        "the run stops at the exit: {out}"
    );
    assert!(
        out.contains("Failed: 0"),
        "stopping is not a test failure: {out}"
    );
    assert_eq!(code, Some(0), "a clean stop is a successful stop: {out}");
}

#[test]
fn replace_needle_diagnostic_does_not_contradict_itself() {
    // The message says Pattern *or* Text is accepted; the structured
    // expected/found pair must not then render "Expected Pattern but found ...".
    let (out, code) = run_src("display replace 5 with \"a\" in \"text\"\n");
    assert_ne!(code, Some(0), "a number needle is refused: {out}");
    assert!(
        out.contains("Pattern or Text"),
        "the diagnostic names both accepted types: {out}"
    );
    assert!(
        !out.contains("Expected Pattern but found"),
        "the diagnostic must not contradict itself: {out}"
    );
}

#[test]
fn count_loop_rejects_a_negative_step() {
    // A negative step moves *away* from the end value. The step is a magnitude
    // — a downward loop is spelled `count from 10 down to 1 by 2`.
    let (out, code) = run_src(
        "count from 1 to 10 by (0 minus 1):\n\
         \x20   display \"tick\"\n\
         end count\n",
    );
    assert_ne!(code, Some(0), "a negative step must be refused: {out}");
    assert!(!out.contains("tick"), "the body must not run: {out}");
    assert!(
        out.to_lowercase().contains("step"),
        "the diagnostic must name the step: {out}"
    );
}

#[test]
fn bare_exit_leaves_the_loop_like_exit_loop() {
    // `exit` with no scope keeps its historic meaning: leave the loop, and let
    // the program carry on. Covered separately from `exit loop` so a regression
    // in either spelling is visible.
    let (out, code) = run_src(
        "count from 1 to 5:\n\
         \x20   display \"tick \" with count\n\
         \x20   check if count is equal to 2:\n\
         \x20       exit\n\
         \x20   end check\n\
         end count\n\
         display \"after\"\n",
    );
    assert!(out.contains("tick 2"), "the deciding trip runs: {out}");
    assert!(!out.contains("tick 3"), "the loop stops: {out}");
    assert!(out.contains("after"), "the program continues: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn bare_exit_at_top_level_does_not_stop_the_program() {
    // Backward compatibility: outside a loop, bare `exit` has always been a
    // no-op (like `break`). `exit program` is the spelling that stops the run,
    // so a program relying on the old no-op keeps working.
    let (out, code) = run_src("display \"before\"\nexit\ndisplay \"after\"\n");
    assert!(out.contains("before"), "statements before run: {out}");
    assert!(
        out.contains("after"),
        "bare exit outside a loop does not stop the program: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}
