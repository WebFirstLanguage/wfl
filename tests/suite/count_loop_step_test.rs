//! A count loop's step is always checked, whether or not the loop would run.
//!
//! The step is a distance: `to` counts up, `down to` counts down, and the step
//! says how far each trip moves. Until this change the step was checked only
//! when the loop was entered, so a step pointing the "wrong" way on a range
//! that was empty in the written direction vanished without a word:
//! `count from 5 to 1 by -1` — which reads like a countdown — ran zero times
//! and exited 0, and so did the mirror `count from 1 down to 5 by -1` and a
//! zero step on either. The step is now validated up front, and a negative
//! step's diagnostic spells out the loop the program most likely meant.
//!
//! These run the real `wfl` binary so the exit status is part of what is
//! asserted: the defect was a silent no-op with status 0.

use crate::common::run_src;

/// Run `header` as a count loop whose body prints `tick`, followed by a
/// statement that prints `after`. A refused loop must print neither.
///
/// Both words are assembled at run time (`"aft" with "er"`) because a runtime
/// diagnostic quotes the surrounding source lines: a literal `"after"` in the
/// program would show up in that excerpt even though the line never ran.
fn run_loop(header: &str) -> (String, Option<i32>) {
    run_src(&format!(
        "{header}:\n    display \"ti\" with \"ck \" with count\nend count\n\
         display \"aft\" with \"er\"\n"
    ))
}

fn assert_refused(header: &str, expected_hint: &str) {
    let (out, code) = run_loop(header);
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
}

#[test]
fn negative_step_on_a_countdown_range_is_reported() {
    // The reported defect: this ran zero times and exited 0.
    assert_refused("count from 5 to 1 by -1", "`count from 5 down to 1`");
}

#[test]
fn negative_step_is_reported_for_any_numbers() {
    assert_refused("count from 12 to 2 by -2", "`count from 12 down to 2 by 2`");
    assert_refused(
        "count from 100 to 0 by -10",
        "`count from 100 down to 0 by 10`",
    );
}

#[test]
fn negative_step_on_an_empty_downward_range_is_reported() {
    // The mirror direction: `down to` plus a negative step on a range that
    // only runs upward was silent too.
    assert_refused("count from 1 down to 5 by -1", "`count from 1 to 5`");
}

#[test]
fn negative_step_on_a_running_loop_suggests_the_loop_it_meant() {
    // These were already refused; the diagnostic now names the program's own
    // loop instead of an unrelated example.
    assert_refused("count from 5 down to 1 by -1", "`count from 5 down to 1`");
    assert_refused("count from 1 to 10 by -3", "`count from 1 to 10 by 3`");
}

#[test]
fn computed_and_fractional_negative_steps_are_reported() {
    let (out, code) = run_src(
        "store stride as 0 minus 0.5\n\
         count from 9 to 3 by stride:\n    display \"ti\" with \"ck\"\nend count\n\
         display \"aft\" with \"er\"\n",
    );
    assert_eq!(code, Some(1), "a computed negative step is an error: {out}");
    assert!(
        out.contains("got -0.5") && out.contains("`count from 9 down to 3 by 0.5`"),
        "the diagnostic names the step and the loop it meant: {out}"
    );
    assert!(
        !out.contains("tick") && !out.contains("after"),
        "nothing runs: {out}"
    );
}

#[test]
fn zero_step_on_an_empty_range_is_reported() {
    // Reverses the #699 review decision that a zero step on an empty range
    // was harmless: a step of 0 is never meaningful, so it is never silent.
    for header in ["count from 5 to 1 by 0", "count from 1 down to 5 by 0"] {
        assert_refused(header, "A step of 0 never moves the counter");
    }
}

#[test]
fn step_error_can_be_caught() {
    let (out, code) = run_src(
        "store ran as 0\n\
         try:\n\
         \x20   count from 5 to 1 by -1:\n\
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
    // not an error, and neither is any other range that is empty in the
    // written direction — so long as the step is a positive distance.
    let (out, code) = run_src(
        "store items as []\n\
         count from 1 to length of items:\n    display \"tick\"\nend count\n\
         count from 5 to 1:\n    display \"tick\"\nend count\n\
         count from 5 to 1 by 2:\n    display \"tick\"\nend count\n\
         count from 1 down to 5:\n    display \"tick\"\nend count\n\
         count from 1 down to 5 by 2:\n    display \"tick\"\nend count\n\
         display \"done\"\n",
    );
    assert_eq!(code, Some(0), "an empty range is not an error: {out}");
    assert!(!out.contains("tick"), "no body runs: {out}");
    assert!(out.contains("done"), "the program continues: {out}");
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
