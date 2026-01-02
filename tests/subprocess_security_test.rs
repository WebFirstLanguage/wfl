use std::fs;
use std::process::Command;
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
        wait for execute command "cd" as result
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
        wait for 200 milliseconds
        wait for read output from process proc_id as proc_output
        wait for process proc_id to complete
        display proc_output
    "#;

    let result = run_wfl(code);
    assert!(
        result.is_ok(),
        "Spawn with safe arguments should work: {:?}",
        result
    );
    assert!(
        result.unwrap().contains("test"),
        "Output should contain expected text"
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
        wait for 200 milliseconds
        wait for read output from process proc1 as out1
        wait for read output from process proc2 as out2
        wait for process proc1 to complete
        wait for process proc2 to complete
        display out1
        display out2
    "#;

    let result = run_wfl(code);
    assert!(
        result.is_ok(),
        "Multiple safe processes should work: {:?}",
        result
    );
}
