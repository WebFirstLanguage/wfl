//! A count loop's step is always checked, whether or not the loop would run.
//!
//! The step is a distance: `to` counts up, `down to` counts down, and the step
//! says how far each trip moves. Until this change the step was checked only
//! when the loop was entered, so a step pointing the "wrong" way on a range
//! that was empty in the written direction vanished without a word:
//! `count from 5 to 1 by -1` — which reads like a countdown — ran zero times
//! and exited 0, and so did the mirror `count from 1 down to 5 by -1` and a
//! zero step on either.
//!
//! A step of zero or less (or a non-finite one) is now always reported:
//!
//! * when the loop would run, it is a runtime error, as before;
//! * when the range is empty, the loop still runs zero times and the program
//!   carries on — GOVERNANCE.md §3.1 keeps existing programs working for a
//!   year — but a warning on stderr names the loop the program most likely
//!   meant and the date the warning becomes an error.
//!
//! These run the real `wfl` binary so exit status and the stdout/stderr split
//! are part of what is asserted: the defect was a silent no-op with status 0.

use crate::common::{run_src, wfl_exe};

/// The date in the warning after which an empty-range bad step is an error.
const DEADLINE: &str = "2027-12-01";

/// Run `src` and return (stdout, stderr, exit code) separately, so a test can
/// tell the program's output from a diagnostic.
fn run_split(src: &str) -> (String, String, Option<i32>) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("main.wfl");
    std::fs::write(&path, src).expect("write program");
    let output = std::process::Command::new(wfl_exe())
        .arg(&path)
        .output()
        .expect("failed to execute WFL");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code(),
    )
}

/// A count loop with `header`, whose body prints `tick`, followed by a
/// statement that prints `after`.
///
/// Both words are assembled at run time (`"aft" with "er"`) because a runtime
/// diagnostic quotes the surrounding source lines: a literal `"after"` in the
/// program would show up in that excerpt even though the line never ran.
fn loop_program(header: &str) -> String {
    format!(
        "{header}:\n    display \"ti\" with \"ck \" with count\nend count\n\
         display \"aft\" with \"er\"\n"
    )
}

/// The loop would run, so its bad step stops the program with an error.
fn assert_refused(header: &str, expected_hint: &str) {
    let (out, code) = run_src(&loop_program(header));
    assert_eq!(code, Some(1), "`{header}` must be a runtime error: {out}");
    assert!(
        out.contains("Count loop step must be a positive number"),
        "`{header}`: the diagnostic must name the step rule: {out}"
    );
    assert!(
        out.contains(expected_hint),
        "`{header}`: the diagnostic must contain `{expected_hint}`: {out}"
    );
    assert!(
        !out.contains("tick"),
        "`{header}`: the body must not run: {out}"
    );
    assert!(
        !out.contains("after"),
        "`{header}`: the program must stop at the loop: {out}"
    );
    assert!(
        !out.contains(DEADLINE),
        "`{header}`: a loop that would run is an error now, not a deprecation: {out}"
    );
}

/// The range is empty, so the loop runs zero times and the program carries
/// on, but the bad step is reported on stderr with the deadline.
fn assert_warned(header: &str, expected_hint: &str) {
    let (stdout, stderr, code) = run_split(&loop_program(header));
    assert_eq!(
        code,
        Some(0),
        "`{header}` keeps working during the deprecation window:\n{stdout}\n{stderr}"
    );
    assert!(
        !stdout.contains("tick"),
        "`{header}`: the body must not run: {stdout}"
    );
    assert!(
        stdout.contains("after"),
        "`{header}`: the program must carry on after the loop: {stdout}"
    );
    assert!(
        stderr.contains("Warning: Count loop step must be a positive number"),
        "`{header}`: the warning must name the step rule: {stderr}"
    );
    assert!(
        stderr.contains(expected_hint),
        "`{header}`: the warning must contain `{expected_hint}`: {stderr}"
    );
    assert!(
        stderr.contains(&format!("from {DEADLINE} this will be an error")),
        "`{header}`: the warning must give the deadline: {stderr}"
    );
    assert!(
        !stdout.contains("Warning"),
        "`{header}`: the warning goes to stderr, not the program's output: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// Empty range: runs zero times, but no longer silently
// ---------------------------------------------------------------------------

#[test]
fn negative_step_on_a_countdown_range_is_reported() {
    // The reported defect: this ran zero times and exited 0 without a word.
    assert_warned("count from 5 to 1 by -1", "`count from 5 down to 1`");
}

#[test]
fn negative_step_is_reported_for_any_numbers() {
    assert_warned("count from 12 to 2 by -2", "`count from 12 down to 2 by 2`");
    assert_warned(
        "count from 100 to 0 by -10",
        "`count from 100 down to 0 by 10`",
    );
}

#[test]
fn negative_step_on_an_empty_downward_range_is_reported() {
    // The mirror direction: `down to` plus a negative step on a range that
    // only runs upward was silent too.
    assert_warned("count from 1 down to 5 by -1", "`count from 1 to 5`");
}

#[test]
fn computed_and_fractional_negative_steps_are_reported() {
    let (stdout, stderr, code) = run_split(
        "store stride as 0 minus 0.5\n\
         count from 9 to 3 by stride:\n    display \"ti\" with \"ck\"\nend count\n\
         display \"aft\" with \"er\"\n",
    );
    assert_eq!(code, Some(0), "an empty range keeps working: {stderr}");
    assert!(
        stderr.contains("got -0.5") && stderr.contains("`count from 9 down to 3 by 0.5`"),
        "the warning names the step and the loop it meant: {stderr}"
    );
    assert!(
        !stdout.contains("tick") && stdout.contains("after"),
        "zero trips, then the program carries on: {stdout}"
    );
}

#[test]
fn zero_step_on_an_empty_range_is_reported() {
    // #699 kept a zero step on an empty range working; it still does for now,
    // but a step of 0 is never meaningful, so it is no longer silent.
    for header in ["count from 5 to 1 by 0", "count from 1 down to 5 by 0"] {
        assert_warned(header, "A step of 0 never moves the counter");
    }
}

#[test]
fn negative_zero_step_is_reported_as_zero() {
    // `-0` is a zero step too; it must not slip past as "negative" or print
    // a confusing `-0`.
    assert_warned("count from 5 to 1 by -0", "got 0. A step of 0");
}

/// A program that builds inf at run time — WFL has no infinity literal, and
/// squaring 1e9 eight times overflows.
const BUILD_INF: &str = "store x as 1000000000\n\
                         count from 1 to 8 as j:\n    change x to x times x\nend count\n";

#[test]
fn non_finite_steps_on_an_empty_range_are_reported() {
    for (step, shown) in [
        ("x", "got inf."),
        ("(0 minus x)", "got -inf."),
        ("(x minus x)", "got NaN."),
    ] {
        let (stdout, stderr, code) = run_split(&format!(
            "{BUILD_INF}{}",
            loop_program(&format!("count from 5 to 1 by {step}"))
        ));
        assert_eq!(
            code,
            Some(0),
            "step `{step}`: empty range keeps working: {stderr}"
        );
        assert!(
            stderr.contains(&format!(
                "Warning: Count loop step must be a positive number, {shown}"
            )),
            "step `{step}`: the warning states the rule and `{shown}`: {stderr}"
        );
        assert!(
            !stdout.contains("tick") && stdout.contains("after"),
            "step `{step}`: zero trips, then the program carries on: {stdout}"
        );
    }
}

#[test]
fn each_empty_loop_warns_once_however_often_it_is_reached() {
    // A loop inside another loop is reached on every outer trip; repeating
    // the same warning would bury the program's output. Two different loops
    // are two different problems, so each warns once.
    let (stdout, stderr, code) = run_split(
        "for each item in [1, 2, 3]:\n\
         \x20   count from 5 to 1 by -1:\n\
         \x20       display \"ti\" with \"ck\"\n\
         \x20   end count\n\
         end for\n\
         count from 9 to 3 by -3:\n    display \"ti\" with \"ck\"\nend count\n\
         display \"aft\" with \"er\"\n",
    );
    assert_eq!(code, Some(0), "empty ranges keep working: {stderr}");
    assert!(
        !stdout.contains("tick") && stdout.contains("after"),
        "{stdout}"
    );
    assert_eq!(
        stderr.matches("`count from 5 down to 1`").count(),
        1,
        "the inner loop warns once, not on every outer trip: {stderr}"
    );
    assert_eq!(
        stderr.matches("`count from 9 down to 3 by 3`").count(),
        1,
        "a second loop gets its own warning: {stderr}"
    );
}

#[test]
fn the_warning_is_not_an_error_a_handler_could_catch() {
    let (stdout, stderr, code) = run_split(
        "store ran as 0\n\
         try:\n\
         \x20   count from 5 to 1 by -1:\n\
         \x20       change ran to ran plus 1\n\
         \x20   end count\n\
         when error:\n\
         \x20   display \"caught\"\n\
         end try\n\
         display \"ran: \" with ran\n",
    );
    assert_eq!(code, Some(0), "{stderr}");
    assert!(!stdout.contains("caught"), "no error was raised: {stdout}");
    assert!(stdout.contains("ran: 0"), "the body never ran: {stdout}");
    assert!(stderr.contains("Warning: Count loop step"), "{stderr}");
}

#[test]
fn loops_guide_quotes_the_real_warning() {
    // Run the snippet the loops guide shows and compare its stderr with the
    // warning the guide quotes right after it.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // A Windows checkout may convert the guide to CRLF line endings.
    let guide =
        std::fs::read_to_string(root.join("Docs/03-language-basics/loops-and-iteration.md"))
            .expect("read the loops guide")
            .replace("\r\n", "\n");
    let (before, after) = guide
        .split_once("**Warning (on stderr):**\n```\n")
        .expect("the loops guide quotes the empty-range warning");
    let quoted = after.split("\n```").next().expect("quoted warning");
    let snippet = before
        .rsplit("```wfl\n")
        .next()
        .and_then(|block| block.split("\n```").next())
        .expect("the guide shows the program before the warning");
    let (_, stderr, code) = run_split(&format!("{snippet}\n"));
    assert_eq!(code, Some(0), "the guide's program keeps working: {stderr}");
    assert!(
        stderr.contains(quoted),
        "the guide must quote the real warning.\n  guide:  {quoted}\n  actual: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// The loop would run: a bad step is an error, as before
// ---------------------------------------------------------------------------

#[test]
fn negative_step_on_a_running_loop_suggests_the_loop_it_meant() {
    // These were already refused; the diagnostic now names the program's own
    // loop instead of an unrelated example.
    assert_refused("count from 5 down to 1 by -1", "`count from 5 down to 1`");
    assert_refused("count from 1 to 10 by -3", "`count from 1 to 10 by 3`");
    assert_refused("count from 3 to 3 by -1", "`count from 3 to 3`");
}

#[test]
fn zero_step_on_a_running_loop_is_an_error() {
    assert_refused(
        "count from 1 to 5 by 0",
        "A step of 0 never moves the counter",
    );
}

#[test]
fn non_finite_steps_on_a_running_loop_are_errors() {
    for (step, shown) in [("x", "got inf."), ("(x minus x)", "got NaN.")] {
        let (out, code) = run_src(&format!(
            "{BUILD_INF}{}",
            loop_program(&format!("count from 1 to 5 by {step}"))
        ));
        assert_eq!(code, Some(1), "step `{step}` must be an error: {out}");
        assert!(
            out.contains(&format!(
                "Count loop step must be a positive number, {shown}"
            )),
            "step `{step}`: the diagnostic states the rule and `{shown}`: {out}"
        );
        assert!(
            !out.contains("tick") && !out.contains("after"),
            "step `{step}`: nothing runs: {out}"
        );
    }
}

#[test]
fn step_error_can_be_caught() {
    let (out, code) = run_src(
        "store ran as 0\n\
         try:\n\
         \x20   count from 1 to 5 by -1:\n\
         \x20       change ran to ran plus 1\n\
         \x20   end count\n\
         when error:\n\
         \x20   display \"caught: \" with error_message\n\
         end try\n\
         display \"ran: \" with ran\n",
    );
    assert_eq!(code, Some(0), "a handled error is not fatal: {out}");
    assert!(
        out.contains("caught: Count loop step must be a positive number"),
        "the handler sees the step diagnostic: {out}"
    );
    assert!(out.contains("ran: 0"), "the body never ran: {out}");
}

// ---------------------------------------------------------------------------
// Behaviour that must not change
// ---------------------------------------------------------------------------

#[test]
fn empty_ranges_with_a_valid_step_still_run_zero_times_quietly() {
    // `count from 1 to length of items` over an empty list is an empty range,
    // not a mistake, and neither is any other range that is empty in the
    // written direction — so long as the step is a positive distance. No
    // warning either: these programs are correct.
    let (stdout, stderr, code) = run_split(
        "store items as []\n\
         count from 1 to length of items:\n    display \"tick\"\nend count\n\
         count from 5 to 1:\n    display \"tick\"\nend count\n\
         count from 5 to 1 by 2:\n    display \"tick\"\nend count\n\
         count from 1 down to 5:\n    display \"tick\"\nend count\n\
         count from 1 down to 5 by 2:\n    display \"tick\"\nend count\n\
         display \"done\"\n",
    );
    assert_eq!(code, Some(0), "an empty range is not an error: {stderr}");
    assert!(!stdout.contains("tick"), "no body runs: {stdout}");
    assert!(stdout.contains("done"), "the program continues: {stdout}");
    assert!(
        !stderr.contains("Warning"),
        "a valid empty loop is not warned about: {stderr}"
    );
}

#[test]
fn positive_steps_still_count_in_both_directions() {
    let (out, code) = run_src(
        "store up as \"\"\n\
         count from 1 to 5 by 2:\n    change up to up with count with \" \"\nend count\n\
         store down as \"\"\n\
         count from 5 down to 1 by 2:\n    change down to down with count with \" \"\nend count\n\
         store single as \"\"\n\
         count from 3 to 3:\n    change single to single with count\nend count\n\
         display \"up=[\" with up with \"] down=[\" with down with \"] single=[\" with single with \"]\"\n",
    );
    assert_eq!(code, Some(0), "valid loops run: {out}");
    assert!(
        out.contains("up=[1 3 5 ] down=[5 3 1 ] single=[3]"),
        "each loop visits exactly its values: {out}"
    );
}
