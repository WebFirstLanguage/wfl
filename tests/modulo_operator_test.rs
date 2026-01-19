/// Test that the modulo operator (%) works correctly
///
/// This test verifies the implementation of the % operator for computing remainders.
mod test_helpers;
use test_helpers::*;

#[test]
fn test_modulo_operator_basic() {
    let test_program = r#"
// Test basic modulo operations
store r1 as 5 % 2
store r2 as 10 % 3
store r3 as 7 % 7
store r4 as 6 % 4

store result as "PASS"

check if r1 is equal to 1:
    // OK
otherwise:
    change result to "FAIL: 5 % 2 should be 1"
end check

check if r2 is equal to 1:
    // OK
otherwise:
    change result to "FAIL: 10 % 3 should be 1"
end check

check if r3 is equal to 0:
    // OK
otherwise:
    change result to "FAIL: 7 % 7 should be 0"
end check

check if r4 is equal to 2:
    // OK
otherwise:
    change result to "FAIL: 6 % 4 should be 2"
end check

display result
"#;

    // Run the WFL program and assert results
    let output = run_wfl_program(test_program, "test_modulo_basic");
    assert_wfl_success_with_output(&output, &["PASS"], &["FAIL"]);
}

#[test]
fn test_modulo_with_even_odd_check() {
    let test_program = r#"
// Test modulo for even/odd checking (like the nexus test)
store total as 0
store i as 0

repeat while i is less than 5:
    change i to i plus 1
    check if (i % 2) is equal to 0:
        skip  // Skip even numbers
    end check
    change total to total plus i
end repeat

// Should sum only odd numbers: 1 + 3 + 5 = 9
check if total is equal to 9:
    display "PASS"
otherwise:
    display "FAIL: expected 9, got " with total
end check
"#;

    // Run the WFL program and assert results
    let output = run_wfl_program(test_program, "test_modulo_even_odd");
    assert_wfl_success_with_output(&output, &["PASS"], &[]);
}

#[test]
fn test_modulo_by_zero_error() {
    let test_program = r#"
// Test that modulo by zero raises an error
store result as "FAIL"

try:
    store r as 5 % 0
    change result to "FAIL: No error raised"
catch:
    change result to "PASS"
end try

display result
"#;

    // Run the WFL program and assert results
    let output = run_wfl_program(test_program, "test_modulo_zero");
    assert_wfl_success_with_output(&output, &["PASS"], &[]);
}
