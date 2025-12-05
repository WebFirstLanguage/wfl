use std::env;
/// Test that errors from zero-argument user-defined actions are properly propagated
/// when the action is called without arguments (auto-call behavior).
///
/// This test verifies the fix for a bug where `store res as faulty` would store
/// the function value instead of calling it and catching errors.
use std::fs;
use std::process::Command;

#[test]
fn test_zero_arg_action_error_propagation() {
    // Get the path to the WFL binary
    let wfl_binary = if cfg!(target_os = "windows") {
        "target/release/wfl.exe"
    } else {
        "target/release/wfl"
    };

    // Verify the binary exists
    let binary_path = env::current_dir().unwrap().join(wfl_binary);

    if !binary_path.exists() {
        panic!(
            "WFL binary not found at {:?}. Run 'cargo build --release' first.",
            binary_path
        );
    }

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

    let test_file = "test_zero_arg_error.wfl";
    fs::write(test_file, test_program).expect("Failed to write test file");

    // Run the WFL program
    let output = Command::new(&binary_path)
        .arg(test_file)
        .output()
        .expect("Failed to execute WFL binary");

    // Clean up
    fs::remove_file(test_file).ok();

    // Check the output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "WFL program failed to execute.\nStdout: {}\nStderr: {}",
        stdout,
        stderr
    );

    assert!(
        stdout.contains("PASS"),
        "Expected 'PASS' in output, but got:\nStdout: {}\nStderr: {}",
        stdout,
        stderr
    );

    assert!(
        !stdout.contains("FAIL"),
        "Found 'FAIL' in output:\nStdout: {}\nStderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn test_zero_arg_action_auto_call() {
    // Get the path to the WFL binary
    let wfl_binary = if cfg!(target_os = "windows") {
        "target/release/wfl.exe"
    } else {
        "target/release/wfl"
    };

    // Verify the binary exists
    let binary_path = env::current_dir().unwrap().join(wfl_binary);

    if !binary_path.exists() {
        panic!(
            "WFL binary not found at {:?}. Run 'cargo build --release' first.",
            binary_path
        );
    }

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

    let test_file = "test_zero_arg_autocall.wfl";
    fs::write(test_file, test_program).expect("Failed to write test file");

    // Run the WFL program
    let output = Command::new(&binary_path)
        .arg(test_file)
        .output()
        .expect("Failed to execute WFL binary");

    // Clean up
    fs::remove_file(test_file).ok();

    // Check the output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "WFL program failed to execute.\nStdout: {}\nStderr: {}",
        stdout,
        stderr
    );

    assert!(
        stdout.contains("PASS"),
        "Expected 'PASS' in output, but got:\nStdout: {}\nStderr: {}",
        stdout,
        stderr
    );

    assert!(
        !stdout.contains("FAIL"),
        "Found 'FAIL' in output:\nStdout: {}\nStderr: {}",
        stdout,
        stderr
    );
}
