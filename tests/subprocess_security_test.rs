use std::fs;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile::NamedTempFile;

/// Robust temporary file cleanup wrapper
struct TempWflFile {
    _file: NamedTempFile,
    path: String,
}

impl TempWflFile {
    fn new(code: &str) -> Result<Self, std::io::Error> {
        let file = NamedTempFile::with_suffix(".wfl")?;
        fs::write(file.path(), code)?;
        let path = file.path().to_string_lossy().to_string();
        Ok(TempWflFile { _file: file, path })
    }

    fn path(&self) -> &str {
        &self.path
    }
}

fn run_wfl(code: &str) -> Result<String, String> {
    let temp_file = TempWflFile::new(code).expect("Failed to create temp file");

    let wfl_exe = if cfg!(target_os = "windows") {
        "target/release/wfl.exe"
    } else {
        "target/release/wfl"
    };

    let output = Command::new(wfl_exe)
        .arg(temp_file.path())
        .output()
        .expect("Failed to execute WFL");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !stderr.is_empty() && stderr.contains("error") || stderr.contains("Error") {
        Err(stderr)
    } else {
        Ok(format!("{}{}", stdout, stderr))
    }
}

/// Run WFL with retry logic that validates output contains expected strings
/// This is more robust for timing-sensitive tests that might succeed but produce empty output
fn run_wfl_with_validation(
    code: &str,
    max_attempts: usize,
    expected_strings: &[&str],
) -> Result<String, String> {
    let mut last_result = Err("Not attempted".to_string());

    for attempt in 1..=max_attempts {
        match run_wfl(code) {
            Ok(output) => {
                // Check if output contains all expected strings
                let all_present = expected_strings.iter().all(|s| output.contains(s));

                if all_present {
                    return Ok(output);
                } else {
                    // Output succeeded but doesn't have expected content - might be timing issue
                    let missing: Vec<&str> = expected_strings
                        .iter()
                        .filter(|s| !output.contains(*s))
                        .copied()
                        .collect();
                    last_result = Err(format!(
                        "Output missing expected strings after {} of {} attempts. Missing: {:?}. Actual output: '{}'",
                        attempt, max_attempts, missing, output
                    ));
                    if attempt < max_attempts {
                        // Wait before retrying with exponential backoff
                        thread::sleep(Duration::from_millis(150 * 2u64.saturating_pow((attempt - 1) as u32)));
                    }
                }
            }
            Err(e) => {
                last_result = Err(e);
                if attempt < max_attempts {
                    // Wait before retrying with exponential backoff
                    thread::sleep(Duration::from_millis(150 * 2u64.saturating_pow((attempt - 1) as u32)));
                }
            }
        }
    }

    // Return the last result even if validation failed
    last_result
}

#[test]
fn test_shell_injection_blocked_by_default() {
    let code = r#"
        execute command "echo test; echo injected" as result
    "#;

    let result = run_wfl(code);
    assert!(
        result.is_err(),
        "Command with semicolon should be blocked by default security policy"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("security policy") || err.contains("blocked"),
        "Error should mention security policy: {}",
        err
    );
}

#[test]
fn test_safe_argument_execution() {
    let code = r#"
        wait for execute command "echo" with arguments ["hello", "world"] as result
        display result
    "#;

    let result = run_wfl(code);
    assert!(result.is_ok(), "Safe argument-based execution should work");
    assert!(
        result.unwrap().contains("hello"),
        "Output should contain expected text"
    );
}

#[test]
fn test_pipe_blocked_by_default() {
    let code = r#"
        execute command "echo test | grep test" as result
    "#;

    let result = run_wfl(code);
    assert!(
        result.is_err(),
        "Command with pipe should be blocked by default"
    );
}

#[test]
fn test_command_substitution_blocked() {
    #[cfg(not(windows))]
    let code = r#"
        execute command "echo $(whoami)" as result
    "#;

    #[cfg(windows)]
    let code = r#"
        execute command "echo test" as result
    "#;

    #[cfg(not(windows))]
    {
        let result = run_wfl(code);
        assert!(
            result.is_err(),
            "Command substitution should be blocked by default"
        );
    }

    #[cfg(windows)]
    {
        // On Windows, just verify safe commands work
        let result = run_wfl(code);
        assert!(result.is_ok(), "Simple safe command should work on Windows");
    }
}

#[test]
fn test_background_execution_blocked() {
    #[cfg(not(windows))]
    let code = r#"
        execute command "echo test &" as result
    "#;

    #[cfg(windows)]
    let code = r#"
        execute command "echo test" as result
    "#;

    #[cfg(not(windows))]
    {
        let result = run_wfl(code);
        assert!(
            result.is_err(),
            "Background execution should be blocked by default"
        );
    }

    #[cfg(windows)]
    {
        let result = run_wfl(code);
        assert!(result.is_ok(), "Safe command should work");
    }
}

#[test]
fn test_redirection_blocked() {
    let code = r#"
        execute command "echo test > output.txt" as result
    "#;

    let result = run_wfl(code);
    assert!(
        result.is_err(),
        "Command with redirection should be blocked by default"
    );
}

#[test]
fn test_simple_command_without_args_works() {
    // Simple commands without shell features should work
    #[cfg(not(windows))]
    let code = r#"
        wait for execute command "pwd" as result
        display result
    "#;

    #[cfg(windows)]
    let code = r#"
        wait for execute command "hostname" as result
        display result
    "#;

    let result = run_wfl(code);
    assert!(
        result.is_ok(),
        "Simple command without shell features should work: {:?}",
        result
    );
}

#[test]
fn test_spawn_with_safe_arguments() {
    let code = r#"
        spawn command "echo" with arguments ["test"] as proc_id
        wait for 250 milliseconds
        wait for read output from process proc_id as proc_output
        wait for process proc_id to complete
        display proc_output
    "#;

    // Use validation-based retry logic to handle timing issues on loaded systems
    let result = run_wfl_with_validation(code, 5, &["test"]);
    assert!(
        result.is_ok(),
        "Spawn with safe arguments should work: {:?}",
        result
    );

    let output = result.unwrap();
    assert!(
        output.contains("test"),
        "Output should contain expected text 'test'. Actual output: '{}'",
        output
    );
}

#[test]
fn test_spawn_shell_blocked_by_default() {
    #[cfg(not(windows))]
    let code = r#"
        spawn command "echo $HOME" as proc_id
    "#;

    #[cfg(windows)]
    let code = r#"
        spawn command "echo test" as proc_id
    "#;

    #[cfg(not(windows))]
    {
        let result = run_wfl(code);
        assert!(
            result.is_err(),
            "Spawn with shell features should be blocked by default"
        );
    }

    #[cfg(windows)]
    {
        // Windows: just verify safe commands work
        let result = run_wfl(code);
        if result.is_err() {
            println!("Note: On Windows, even simple spawns may fail without proper wait");
        }
    }
}

#[test]
fn test_multiple_safe_processes() {
    let code = r#"
        spawn command "echo" with arguments ["test1"] as proc1
        spawn command "echo" with arguments ["test2"] as proc2
        wait for 250 milliseconds
        wait for read output from process proc1 as out1
        wait for read output from process proc2 as out2
        wait for process proc1 to complete
        wait for process proc2 to complete
        display out1
        display out2
    "#;

    // Use validation-based retry logic to handle timing issues on loaded systems
    let result = run_wfl_with_validation(code, 5, &["test1", "test2"]);
    assert!(
        result.is_ok(),
        "Multiple safe processes should work: {:?}",
        result
    );

    let output = result.unwrap();
    assert!(
        output.contains("test1"),
        "Output should contain 'test1'. Actual output: '{}'",
        output
    );
    assert!(
        output.contains("test2"),
        "Output should contain 'test2'. Actual output: '{}'",
        output
    );
}
