//! Exact-stdout regression coverage for multi-value `display` (space-separated
//! values folded into a `Concatenation`, see `parse_display_statement` in
//! `src/parser/stmt/io.rs`).
//!
//! Prior coverage for this feature only checked AST shape (parser unit tests
//! in `src/parser/tests.rs`) or exit code (`TestPrograms/*.wfl` under the CI
//! runner, which redirects stdout to `/dev/null`). Neither would have caught
//! the mutation-order bug fixed alongside this file: the first cut folded
//! space-separated `display` values left-associatively, while explicit `with`
//! folds right-associatively, and `Concatenation` evaluates+stringifies left
//! before right — so the two forms could observably diverge whenever a later
//! value mutated something an earlier value referenced (see
//! `mutable_list_matches_with_byte_for_byte` below). These tests run the
//! actual `wfl` binary end-to-end and assert exact stdout.

mod test_helpers;

/// Runs `program` as a temporary `.wfl` file and returns its stdout as a
/// `String`, panicking with stderr if the process didn't exit successfully.
///
/// Delegates to `test_helpers::run_wfl_program`, which runs the `wfl` binary
/// under a 30-second timeout (so a regression that stalls the interpreter fails
/// the test instead of hanging CI) and cleans up the temporary script.
fn run_wfl(program: &str) -> String {
    let output = test_helpers::run_wfl_program(program, "display_multiple_values_stdout");

    assert!(
        output.status.success(),
        "program did not exit successfully.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("stdout should be valid UTF-8")
}

// --- Documented happy paths -------------------------------------------------

#[test]
fn documented_happy_path_matches_hello_world_docs() {
    // Docs/02-getting-started/hello-world.md, "Display Several Values at Once".
    let stdout = run_wfl(
        r#"
store name as "Alice"
display "Hello, " name "!"
"#,
    );
    assert_eq!(stdout, "Hello, Alice!\n");
}

#[test]
fn documented_spacing_examples_match_docs_exactly() {
    // Same doc section: spacing comes only from inside the quotes.
    let stdout = run_wfl(
        r#"
store age as 25
display "I am " age " years old"
display "I am" age "years old"
"#,
    );
    assert_eq!(stdout, "I am 25 years old\nI am25years old\n");
}

#[test]
fn original_bug_report_now_prints_every_value() {
    // The report this feature fixes: trailing values were silently dropped.
    let stdout = run_wfl(
        r#"
store user_age as 28
display "user age is " user_age
change user_age to 9
display "user age is " user_age
"#,
    );
    assert_eq!(stdout, "user age is 28\nuser age is 9\n");
}

// --- Action return values, arguments, and side effects ----------------------

#[test]
fn action_return_value_folds_into_display() {
    let stdout = run_wfl(
        r#"
define action called doubled with n:
    give back n times 2
end action
display "doubled: " call doubled with 21
"#,
    );
    assert_eq!(stdout, "doubled: 42\n");
}

#[test]
fn action_with_multiple_arguments_folds_into_display() {
    let stdout = run_wfl(
        r#"
define action called greeting with parameters person_name and times_of_day:
    give back "Good " with times_of_day with ", " with person_name
end action
display "greeting: " call greeting with "Bob" and "morning"
"#,
    );
    assert_eq!(stdout, "greeting: Good morning, Bob\n");
}

#[test]
fn action_with_side_effect_runs_in_display_order() {
    // Each call must both mutate state and interpolate its return value in
    // the order it appears, left to right across the whole display statement.
    let stdout = run_wfl(
        r#"
store counter as 0
define action called increment:
    change counter to counter plus 1
    give back counter
end action
display "first: " call increment " second: " call increment " third: " call increment
"#,
    );
    assert_eq!(stdout, "first: 1 second: 2 third: 3\n");
}

// --- Containers: instance, property, and method -----------------------------

#[test]
fn container_instance_property_and_method_all_fold() {
    let stdout = run_wfl(
        r#"
create container Counter:
    property value: Number

    action bump:
        change value to value plus 1
    end

    action summarize: Text
        return "Counter(" with value with ")"
    end
end

create new Counter as c:
    value is 5
end

c.bump()
display "instance: " c " value: " c.value " desc: " c.summarize()
"#,
    );
    assert_eq!(
        stdout,
        "instance: Counter instance value: 6 desc: Counter(6)\n"
    );
}

// --- Mutable list/container state: byte-for-byte equivalence with `with` ----

#[test]
fn mutable_list_matches_with_byte_for_byte() {
    // This is the central regression: space-separated `display` and explicit
    // `with` must be indistinguishable, including when a later value mutates
    // something an earlier value references. `Concatenation` evaluates left,
    // then right, then stringifies both — so association direction decides
    // whether the list is stringified before or after the `pop` runs. Both
    // forms below are now right-associative (`a with (b with c)`), so `pop`
    // always runs first and both print the post-pop list.
    let with_form = run_wfl(
        r#"
create list right_items:
    add "before"
    add "after"
end list
display right_items with "" with pop of right_items
"#,
    );
    let space_separated_form = run_wfl(
        r#"
create list left_items:
    add "before"
    add "after"
end list
display left_items "" pop of left_items
"#,
    );

    assert_eq!(with_form, "[before]after\n");
    assert_eq!(
        space_separated_form, with_form,
        "space-separated display must byte-for-byte match the equivalent `with` chain"
    );
}

#[test]
fn mutable_container_property_matches_with_byte_for_byte() {
    // Same evaluation-order guarantee as `mutable_list_matches_with_byte_for_byte`,
    // exercised on a list held in a container *property* (accessed via
    // `basket.items`) instead of a plain variable, since a container property
    // is looked up through a separate code path (`Expression::PropertyAccess`
    // in the interpreter) that also has to get the association direction
    // right for its `Rc`-shared `Value::List` to alias correctly.
    let program = |connector: &str| {
        format!(
            r#"
create container Basket:
    property items
end

create new Basket as basket:
    items is ["before", "after"]
end

display basket.items {connector} "" {connector} pop of basket.items
"#
        )
    };

    let with_form = run_wfl(&program("with"));
    let space_separated_form = run_wfl(&program(""));

    assert_eq!(with_form, "[before]after\n");
    assert_eq!(
        space_separated_form, with_form,
        "space-separated display must byte-for-byte match the equivalent `with` chain"
    );
}

// --- Newly-supported keyword-led values --------------------------------------

#[test]
fn keyword_led_values_fold_with_exact_output() {
    // A unique absolute path under the OS temp dir, guaranteed not to exist
    // (nothing ever creates it) and independent of the test's working
    // directory or any files a developer happens to have lying around in the
    // repo root.
    let missing_path = test_helpers::get_unique_test_file_path(
        "display_multiple_values_stdout_does_not_exist",
    )
    .with_extension("missing");
    let missing_path = missing_path.to_str().expect("path should be valid UTF-8");

    let stdout = run_wfl(&format!(
        r#"
store is_admin as no
display "is admin: " not is_admin
display "exists: " file exists at "{missing_path}"
count from 1 to 3:
    display "count is " count
end count
"#
    ));
    assert_eq!(
        stdout,
        "is admin: yes\nexists: no\ncount is 1\ncount is 2\ncount is 3\n"
    );
}

// --- Same-line statement boundaries: `count from ...` / `read output from process ...` ----

#[test]
fn count_from_after_display_on_same_line_stays_a_separate_statement() {
    // `count` alone folds into `display` as the count-loop variable (see
    // `keyword_led_values_fold_with_exact_output`), but `count from ...` on
    // the *same line* as a preceding `display` must still open a count loop,
    // exactly as it did before multi-value `display` existed — not get
    // partially folded into the display and leave `from 1 to 3:` stranded.
    let stdout = run_wfl(
        r#"
display "start" count from 1 to 3:
    display "n: " count
end count
"#,
    );
    assert_eq!(stdout, "start\nn: 1\nn: 2\nn: 3\n");
}

// `read output from process ...` after a same-line `display` is covered as a
// parser-level whole-program regression instead of here
// (`read_output_from_process_after_display_stays_a_separate_statement` in
// `src/parser/tests.rs`): it needs a real subprocess and a `wait for` to
// exercise end-to-end, which would make this suite timing-dependent for a
// case that's purely about statement *boundaries* — the AST shape settles the
// question without needing to actually run a process.
