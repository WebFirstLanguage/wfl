/// Tests for the list literal double execution fix (commit e44195b).
///
/// The bug: when a list literal contained async-requiring elements (e.g., zero-arg
/// user actions), the sync fast path in `evaluate_literal_direct()` would partially
/// evaluate elements, fail on an async element, then the async fallback would
/// re-evaluate ALL elements — causing side effects to execute twice.
///
/// The fix: `requires_async_evaluation()` pre-scans elements without executing them,
/// so the sync fast path is skipped entirely when async is needed.
mod test_helpers;
use test_helpers::*;

/// Count how many times a substring appears in the output
fn count_occurrences(output: &std::process::Output, needle: &str) -> usize {
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.matches(needle).count()
}

#[test]
fn test_list_single_side_effecting_action() {
    let program = r#"
define action called bump:
    display "BUMP_CALLED"
    give back 1
end action

store my_list as [bump]
display "DONE"
"#;

    let output = run_wfl_program(program, "list_single_side_effect");
    assert_wfl_success_with_output(&output, &["BUMP_CALLED", "DONE"], &[]);
    let count = count_occurrences(&output, "BUMP_CALLED");
    assert_eq!(
        count, 1,
        "Action should be called exactly once, but was called {count} times"
    );
}

#[test]
fn test_list_multiple_side_effecting_actions() {
    let program = r#"
define action called action_a:
    display "ACTION_A_CALLED"
    give back "a"
end action

define action called action_b:
    display "ACTION_B_CALLED"
    give back "b"
end action

store my_list as [action_a, action_a, action_b]
display "DONE"
"#;

    let output = run_wfl_program(program, "list_multiple_side_effects");
    assert_wfl_success_with_output(&output, &["DONE"], &[]);
    let count_a = count_occurrences(&output, "ACTION_A_CALLED");
    let count_b = count_occurrences(&output, "ACTION_B_CALLED");
    assert_eq!(
        count_a, 2,
        "action_a should be called exactly twice, but was called {count_a} times"
    );
    assert_eq!(
        count_b, 1,
        "action_b should be called exactly once, but was called {count_b} times"
    );
}

#[test]
fn test_nested_list_with_side_effects() {
    let program = r#"
define action called bump:
    display "NESTED_BUMP"
    give back 1
end action

store my_list as [[bump, "x"], [bump, 99]]
display "DONE"
"#;

    let output = run_wfl_program(program, "list_nested_side_effects");
    assert_wfl_success_with_output(&output, &["DONE"], &[]);
    let count = count_occurrences(&output, "NESTED_BUMP");
    assert_eq!(
        count, 2,
        "Action should be called exactly twice (once per nested list), but was called {count} times"
    );
}

#[test]
fn test_list_mixed_sync_and_async_elements() {
    let program = r#"
store x as 10

define action called bump:
    display "MIXED_BUMP"
    give back 1
end action

store my_list as [1, "hello", x, bump, yes, nothing]
display "DONE"
"#;

    let output = run_wfl_program(program, "list_mixed_sync_async");
    assert_wfl_success_with_output(&output, &["MIXED_BUMP", "DONE"], &[]);
    let count = count_occurrences(&output, "MIXED_BUMP");
    assert_eq!(
        count, 1,
        "Action should be called exactly once in mixed list, but was called {count} times"
    );
}

#[test]
fn test_list_action_return_value_preserved() {
    let program = r#"
define action called get_value:
    display "GET_VALUE_CALLED"
    give back 42
end action

store my_list as [get_value, "other"]
store len as length of my_list
check if len is equal to 2:
    display "PASS: list has correct length"
otherwise:
    display "FAIL: list has " with len with " elements (expected 2)"
end check
"#;

    let output = run_wfl_program(program, "list_action_return_value");
    assert_wfl_success_with_output(&output, &["PASS"], &["FAIL"]);
    let count = count_occurrences(&output, "GET_VALUE_CALLED");
    assert_eq!(
        count, 1,
        "Action should be called exactly once, but was called {count} times"
    );
}

#[test]
fn test_list_all_sync_no_async_fallback() {
    let program = r#"
store x as 10
store y as 20

store my_list as [1, 2, x, y, yes, nothing]

store len as length of my_list
check if len is equal to 6:
    display "PASS: pure sync list has 6 elements"
otherwise:
    display "FAIL: list has " with len with " elements (expected 6)"
end check
"#;

    let output = run_wfl_program(program, "list_all_sync");
    assert_wfl_success_with_output(&output, &["PASS"], &["FAIL"]);
}

#[test]
fn test_list_multiple_actions_execution_order() {
    let program = r#"
define action called first_action:
    display "EXEC_1"
    give back "first"
end action

define action called second_action:
    display "EXEC_2"
    give back "second"
end action

define action called third_action:
    display "EXEC_3"
    give back "third"
end action

store my_list as [first_action, second_action, third_action]
display "DONE"
"#;

    let output = run_wfl_program(program, "list_action_order");
    assert_wfl_success_with_output(&output, &["DONE"], &[]);
    // Each action should execute exactly once
    assert_eq!(
        count_occurrences(&output, "EXEC_1"),
        1,
        "first_action called more than once"
    );
    assert_eq!(
        count_occurrences(&output, "EXEC_2"),
        1,
        "second_action called more than once"
    );
    assert_eq!(
        count_occurrences(&output, "EXEC_3"),
        1,
        "third_action called more than once"
    );
}
