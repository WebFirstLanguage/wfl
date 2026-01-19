use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Helper function to get the path to the WFL binary
///
/// Returns the path to the WFL binary based on the current OS.
/// Panics if the binary doesn't exist, prompting the user to build it.
pub fn get_wfl_binary_path() -> PathBuf {
    let wfl_binary = if cfg!(target_os = "windows") {
        "target/release/wfl.exe"
    } else {
        "target/release/wfl"
    };

    let binary_path = env::current_dir().unwrap().join(wfl_binary);

    if !binary_path.exists() {
        panic!(
            "WFL binary not found at {:?}. Run 'cargo build --release' first.",
            binary_path
        );
    }

    binary_path
}

/// Helper function to create a unique test file path
///
/// Creates a unique temporary file path for WFL test programs to avoid
/// race conditions when tests run in parallel.
pub fn get_unique_test_file_path(prefix: &str) -> PathBuf {
    let temp_dir = env::temp_dir();
    temp_dir.join(format!("{}_{}.wfl", prefix, std::process::id()))
}

/// Helper function to run a WFL program and return the output
///
/// Executes the given WFL program content and returns the Command output.
/// Automatically handles temporary file creation and cleanup.
pub fn run_wfl_program(program_content: &str, test_name: &str) -> std::process::Output {
    let binary_path = get_wfl_binary_path();
    let test_file = get_unique_test_file_path(test_name);

    // Write test program to temporary file
    fs::write(&test_file, program_content).expect("Failed to write test file");

    // Execute WFL program
    let output = Command::new(&binary_path)
        .arg(&test_file)
        .output()
        .expect("Failed to execute WFL binary");

    // Clean up temporary file
    fs::remove_file(&test_file).ok();

    output
}

/// Helper function to assert WFL program execution was successful and contains expected output
///
/// Verifies that the program executed successfully and contains expected strings in stdout.
/// Also verifies that none of the fail_strings appear in stdout.
pub fn assert_wfl_success_with_output(
    output: &std::process::Output,
    pass_strings: &[&str],
    fail_strings: &[&str],
) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "WFL program failed to execute.\nStdout: {}\nStderr: {}",
        stdout,
        stderr
    );

    for pass_string in pass_strings {
        assert!(
            stdout.contains(pass_string),
            "Expected '{}' in output, but got:\nStdout: {}\nStderr: {}",
            pass_string,
            stdout,
            stderr
        );
    }

    for fail_string in fail_strings {
        assert!(
            !stdout.contains(fail_string),
            "Found '{}' in output:\nStdout: {}\nStderr: {}",
            fail_string,
            stdout,
            stderr
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_wfl_binary_path_returns_correct_platform_binary() {
        // This test assumes the binary exists (integration tests require it)
        // If it doesn't exist, it should panic with helpful message
        let path = get_wfl_binary_path();

        if cfg!(target_os = "windows") {
            assert!(path.to_string_lossy().ends_with("wfl.exe"));
        } else {
            assert!(path.to_string_lossy().ends_with("wfl"));
            assert!(!path.to_string_lossy().ends_with(".exe"));
        }

        // Verify it's an absolute path to target/release/
        assert!(path.is_absolute());
        assert!(path.to_string_lossy().contains("target/release/"));
    }

    #[test]
    fn test_get_unique_test_file_path_creates_unique_paths() {
        let path1 = get_unique_test_file_path("test_example");
        let path2 = get_unique_test_file_path("test_example");

        // Both should have the same prefix but same suffix (since same process)
        assert!(path1.to_string_lossy().contains("test_example"));
        assert!(path2.to_string_lossy().contains("test_example"));

        // Should end with .wfl
        assert!(path1.to_string_lossy().ends_with(".wfl"));
        assert!(path2.to_string_lossy().ends_with(".wfl"));

        // Should contain process ID
        let pid = std::process::id().to_string();
        assert!(path1.to_string_lossy().contains(&pid));
        assert!(path2.to_string_lossy().contains(&pid));
    }

    #[test]
    fn test_run_wfl_program_executes_simple_program() {
        // Test with simple WFL program that just displays output
        let program = r#"display "TEST_OUTPUT""#;

        let output = run_wfl_program(program, "test_run_simple");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "WFL program should execute successfully.\nStdout: {}\nStderr: {}",
            stdout,
            stderr
        );

        assert!(
            stdout.contains("TEST_OUTPUT"),
            "Output should contain TEST_OUTPUT, got: {}",
            stdout
        );
    }

    #[test]
    fn test_assert_wfl_success_with_output_passes_on_success() {
        let program = r#"display "PASS""#;
        let output = run_wfl_program(program, "test_assert_success");

        // This should not panic
        assert_wfl_success_with_output(&output, &["PASS"], &["FAIL"]);
    }

    #[test]
    #[should_panic(expected = "Expected 'MISSING' in output")]
    fn test_assert_wfl_success_with_output_panics_on_missing_pass_string() {
        let program = r#"display "PASS""#;
        let output = run_wfl_program(program, "test_assert_missing");

        // This should panic because "MISSING" is not in output
        assert_wfl_success_with_output(&output, &["MISSING"], &[]);
    }

    #[test]
    #[should_panic(expected = "Found 'FAIL' in output")]
    fn test_assert_wfl_success_with_output_panics_on_fail_string() {
        let program = r#"display "FAIL: something went wrong""#;
        let output = run_wfl_program(program, "test_assert_fail");

        // This should panic because "FAIL" is found in output
        assert_wfl_success_with_output(&output, &[], &["FAIL"]);
    }
}
