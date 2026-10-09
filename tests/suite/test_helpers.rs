use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Helper function to get the path to the WFL binary
///
/// Returns the path to the WFL binary based on the current OS.
/// Tries release build first, then debug build as fallback.
/// Panics if neither binary exists, prompting the user to build it.
pub fn get_wfl_binary_path() -> PathBuf {
    let current_dir = env::current_dir().unwrap();

    // Try release build first
    let release_binary = if cfg!(target_os = "windows") {
        "target/release/wfl.exe"
    } else {
        "target/release/wfl"
    };

    let release_path = current_dir.join(release_binary);
    if release_path.exists() {
        return release_path;
    }

    // Fall back to debug build
    let debug_binary = if cfg!(target_os = "windows") {
        "target/debug/wfl.exe"
    } else {
        "target/debug/wfl"
    };

    let debug_path = current_dir.join(debug_binary);
    if debug_path.exists() {
        return debug_path;
    }

    // Neither exists, panic with helpful message
    panic!(
        "WFL binary not found at {:?} or {:?}. Run 'cargo build --release' or 'cargo build' first.",
        release_path, debug_path
    );
}

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Helper function to create a unique test file path
///
/// Creates a unique temporary file path for WFL test programs to avoid
/// race conditions when tests run in parallel.
pub fn get_unique_test_file_path(prefix: &str) -> PathBuf {
    let temp_dir = env::temp_dir();
    let counter = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let thread_id = thread::current().id();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    temp_dir.join(format!(
        "{}_{}_{:?}_{}_{}.wfl",
        prefix,
        std::process::id(),
        thread_id,
        timestamp,
        counter
    ))
}

/// Helper function to clean up temporary files with retry logic
///
/// Attempts to remove a file with multiple retry attempts if the initial cleanup fails.
/// This helps handle cases where the file might be temporarily locked or in use.
fn cleanup_temp_file_with_retry(file_path: &PathBuf, max_retries: u32) {
    for attempt in 0..=max_retries {
        match fs::remove_file(file_path) {
            Ok(()) => return, // Success, we're done
            Err(e) if attempt == max_retries => {
                eprintln!(
                    "Error: Failed to clean up test file {:?} after {} attempts: {}",
                    file_path,
                    max_retries + 1,
                    e
                );
                return;
            }
            Err(_) => {
                // Failed, but we have more retries left
                thread::sleep(Duration::from_millis(50)); // Brief pause before retry
                continue;
            }
        }
    }
}

/// Helper function to run a WFL program and return the output
///
/// Executes the given WFL program content and returns the Command output.
/// Automatically handles temporary file creation, execution with timeout, and cleanup.
pub fn run_wfl_program(program_content: &str, test_name: &str) -> std::process::Output {
    let binary_path = get_wfl_binary_path();
    let test_file = get_unique_test_file_path(test_name);

    // Write test program to temporary file
    fs::write(&test_file, program_content).expect("Failed to write test file");

    // Execute WFL program with timeout
    let output = execute_with_timeout(&binary_path, &test_file, Duration::from_secs(30))
        .expect("Failed to execute WFL binary");

    // Clean up temporary file with retry logic
    cleanup_temp_file_with_retry(&test_file, 3);

    output
}

/// Helper function to execute a command with timeout
///
/// Spawns the command and waits for it to complete within the specified timeout.
/// Returns an error if the command doesn't complete within the timeout.
fn execute_with_timeout(
    binary_path: &PathBuf,
    test_file: &PathBuf,
    timeout: Duration,
) -> Result<std::process::Output, std::io::Error> {
    use std::process::Stdio;

    let mut child = Command::new(binary_path)
        .arg(test_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()?;

    // Use a simple timeout approach since we don't have external dependencies
    let start = std::time::Instant::now();
    let timeout_duration = timeout;

    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                // Process finished, collect output
                let output = child.wait_with_output()?;
                return Ok(output);
            }
            Ok(None) => {
                // Process still running, check timeout
                if start.elapsed() > timeout_duration {
                    // Timeout exceeded, kill the process
                    let _ = child.kill();
                    let _ = child.wait(); // Clean up zombie
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!(
                            "WFL program execution timed out after {:?}",
                            timeout_duration
                        ),
                    ));
                }
                // Wait a bit before checking again
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(e),
        }
    }
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

        // Verify it's an absolute path to target/release/ or target/debug/
        assert!(path.is_absolute());
        let has_target_release = path
            .components()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|w| w[0].as_os_str() == "target" && w[1].as_os_str() == "release");
        let has_target_debug = path
            .components()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|w| w[0].as_os_str() == "target" && w[1].as_os_str() == "debug");
        assert!(
            has_target_release || has_target_debug,
            "Path should contain target/release or target/debug: {:?}",
            path
        );
    }

    #[test]
    fn test_get_unique_test_file_path_creates_unique_paths() {
        let path1 = get_unique_test_file_path("test_example");
        let path2 = get_unique_test_file_path("test_example");

        // Both should contain the prefix
        assert!(path1.to_string_lossy().contains("test_example"));
        assert!(path2.to_string_lossy().contains("test_example"));

        // Should end with .wfl
        assert!(path1.to_string_lossy().ends_with(".wfl"));
        assert!(path2.to_string_lossy().ends_with(".wfl"));

        // Should contain process ID
        let pid = std::process::id().to_string();
        assert!(path1.to_string_lossy().contains(&pid));
        assert!(path2.to_string_lossy().contains(&pid));

        // Paths should be different due to counter and timestamp
        assert_ne!(path1, path2, "Paths should be unique");
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
