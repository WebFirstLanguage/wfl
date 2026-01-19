/// Test that errors from zero-argument user-defined actions are properly propagated
/// when the action is called without arguments (auto-call behavior).
///
/// This test verifies the fix for a bug where `store res as faulty` would store
/// the function value instead of calling it and catching errors.
mod test_helpers;
use test_helpers::*;

#[test]
fn test_zero_arg_action_error_propagation() {
    // Create a test WFL program
    let test_program = r#"
// Define a zero-argument action that raises an error
define action called faulty_action:
    give back 1 divided by 0
end action

store test_passed as "FAIL"

// Test that the action is called and error is caught
try:
    store res as faulty_action
    // If we reach here, the error was not raised
    change test_passed to "FAIL: No error raised"
catch:
    // Error was properly caught
    change test_passed to "PASS"
end try

display test_passed
"#;

    // Run the WFL program and assert results
    let output = run_wfl_program(test_program, "test_zero_arg_error");
    assert_wfl_success_with_output(&output, &["PASS"], &["FAIL"]);
}

#[test]
fn test_zero_arg_action_auto_call() {
    // Create a test WFL program
    let test_program = r#"
// Define a zero-argument action that returns a value
define action called get_value:
    give back 42
end action

// Test that the action is auto-called when referenced
store result as get_value

check if result is equal to 42:
    display "PASS: Action was auto-called"
otherwise:
    display "FAIL: Got " with result with " instead of 42"
end check
"#;

    // Run the WFL program and assert results
    let output = run_wfl_program(test_program, "test_zero_arg_autocall");
    assert_wfl_success_with_output(&output, &["PASS"], &["FAIL"]);
}
