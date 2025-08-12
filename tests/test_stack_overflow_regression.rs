// Test for stack overflow regression in async recursion
// This test reproduces the issue described in issue #145

use std::process::Command;

#[test]
fn test_no_stack_overflow_with_complex_nested_conditionals() {
    // This test verifies that complex nested conditionals with string concatenation
    // and function calls don't cause stack overflow due to excessive async recursion

    // Create a temporary WFL program that mimics the problematic pattern
    let test_program = r#"
display "Testing stack overflow regression"

// Simulate the exact pattern from args_comprehensive.wfl that caused issues
store test_args as ["--azusa" and "is" and "cool"]

for each arg in test_args:
    check if arg is "--help" or arg is "-h":
        display "Help flag found"
    otherwise:
        check if arg is "--version" or arg is "-v":
            display "Version flag found"
        otherwise:
            check if arg is "--verbose":
                display "Verbose flag found"
            otherwise:
                check if substring of arg and 0 and 1 is "-":
                    display "Unknown flag: " with arg
                otherwise:
                    display "Regular arg: " with arg
                end check
            end check
        end check
    end check
end for

display "Test completed successfully"
"#;

    // Write the test program to a temporary file
    let test_file = "TestPrograms/temp_stack_overflow_test.wfl";
    std::fs::write(test_file, test_program).expect("Failed to write test file");

    // Run the WFL program with a timeout to detect stack overflow
    let output = Command::new("cargo")
        .args(["run", "--", test_file])
        .output()
        .expect("Failed to execute command");

    // Clean up
    let _ = std::fs::remove_file(test_file);

    // Check that the program completed successfully without stack overflow
    assert!(
        output.status.success(),
        "Program failed with exit code: {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Test completed successfully"),
        "Program did not complete properly. Output: {stdout}"
    );

    // Ensure no stack overflow occurred
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("stack overflow") && !stderr.contains("stack has overflowed"),
        "Stack overflow detected in stderr: {stderr}"
    );
}

#[test]
fn test_original_args_comprehensive_no_stack_overflow() {
    // Test the original args_comprehensive.wfl program with the problematic arguments
    // This reproduces the exact issue from the bug report

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "TestPrograms/args_comprehensive.wfl",
            "--azusa",
            "is",
            "cool",
        ])
        .output()
        .expect("Failed to execute command");

    // The program should complete without stack overflow
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check for stack overflow indicators
    assert!(
        !stderr.contains("stack overflow")
            && !stderr.contains("stack has overflowed")
            && !stderr.contains("STATUS_STACK_OVERFLOW"),
        "Stack overflow detected when running args_comprehensive.wfl with --azusa flags. stderr: {stderr}"
    );

    // Program should either succeed or fail gracefully, but not with stack overflow
    if !output.status.success() {
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        // If it fails, it should not be due to stack overflow
        assert!(
            !stderr_str.contains("stack overflow"),
            "Program failed due to stack overflow: {stderr_str}"
        );
    }
}
