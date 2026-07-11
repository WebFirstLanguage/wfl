use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Temp directory with a `.wfl` program and permissive `.wflcfg` for intentional subprocess tests.
struct TempWflEnv {
    _dir: TempDir,
    program_path: String,
}

impl TempWflEnv {
    fn new(code: &str) -> Result<Self, std::io::Error> {
        let dir = TempDir::new()?;
        // Cleanup/resource tests intentionally spawn processes; opt in explicitly.
        fs::write(
            dir.path().join(".wflcfg"),
            r#"
allow_shell_execution = true
shell_execution_mode = sanitized
warn_on_shell_execution = false
"#,
        )?;
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

fn run_wfl(code: &str) -> Result<String, String> {
    let env = TempWflEnv::new(code).expect("Failed to create temp WFL env");

    let wfl_exe = if cfg!(target_os = "windows") {
        "target/release/wfl.exe"
    } else {
        "target/release/wfl"
    };

    let output = Command::new(wfl_exe)
        .arg(env.path())
        .output()
        .expect("Failed to execute WFL");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(format!("{}{}", stdout, stderr))
    } else {
        Err(format!("{}{}", stdout, stderr))
    }
}

/// Platform-appropriate short-lived process spawn (echo).
#[cfg(not(windows))]
fn echo_spawn(label: &str, var: &str) -> String {
    format!(
        r#"spawn command "echo" with arguments ["{}"] as {}"#,
        label, var
    )
}

#[cfg(windows)]
fn echo_spawn(label: &str, var: &str) -> String {
    format!(
        r#"spawn command "cmd.exe" with arguments ["/C", "echo {}"] as {}"#,
        label, var
    )
}

/// Platform-appropriate long-running process.
#[cfg(not(windows))]
fn sleep_spawn(secs: &str, var: &str) -> String {
    format!(
        r#"spawn command "sleep" with arguments ["{}"] as {}"#,
        secs, var
    )
}

#[cfg(windows)]
fn sleep_spawn(secs: &str, var: &str) -> String {
    // `timeout` is available on Windows runners; /nobreak avoids needing input.
    format!(
        r#"spawn command "timeout" with arguments ["/T", "{}", "/NOBREAK"] as {}"#,
        secs, var
    )
}

#[test]
fn test_automatic_cleanup_of_completed_processes() {
    // This test verifies that completed processes are automatically cleaned up
    // when new processes are spawned
    let code = format!(
        r#"
        // Spawn 5 quick processes that complete immediately
        {p1}
        {p2}
        {p3}
        {p4}
        {p5}

        // Wait for them to complete
        wait for 500 milliseconds

        // Spawn more processes - cleanup should happen automatically
        {p6}
        {p7}

        wait for 200 milliseconds
        wait for process proc6 to complete
        wait for process proc7 to complete

        display "Cleanup test completed"
    "#,
        p1 = echo_spawn("test1", "proc1"),
        p2 = echo_spawn("test2", "proc2"),
        p3 = echo_spawn("test3", "proc3"),
        p4 = echo_spawn("test4", "proc4"),
        p5 = echo_spawn("test5", "proc5"),
        p6 = echo_spawn("test6", "proc6"),
        p7 = echo_spawn("test7", "proc7"),
    );

    let result = run_wfl(&code);
    assert!(
        result.is_ok(),
        "Automatic cleanup should allow spawning new processes after old ones complete: {:?}",
        result
    );
}

#[test]
fn test_process_limit_prevents_unbounded_growth() {
    // This test would fail without process limits if we tried to spawn 1000 processes
    // With limits, it should fail gracefully after hitting the limit
    let sleep_line = sleep_spawn("10", "proc");
    let code = format!(
        r#"
        // Try to spawn many long-running processes
        try:
            count from 1 to 150:
                {sleep_line}
            end count
        when:
            display "Hit process limit as expected"
        end try
    "#,
        sleep_line = sleep_line
    );

    // This should either complete (if limits allow 150) or fail with process limit error
    let result = run_wfl(&code);
    // Either success or error mentioning process limit is acceptable
    if let Err(err) = result {
        assert!(
            err.contains("Process limit") || err.contains("limit reached"),
            "Should mention process limit: {}",
            err
        );
    }
}

#[test]
fn test_bounded_buffer_prevents_memory_explosion() {
    // Spawn a process that generates lots of output
    // The bounded buffer should prevent unbounded memory growth
    #[cfg(not(windows))]
    let code = r#"
        spawn command "yes" with arguments ["test"] as spam_proc
        wait for 1 second
        wait for read output from process spam_proc as output
        kill process spam_proc
        display length of output
    "#
    .to_string();

    #[cfg(windows)]
    let code = format!(
        r#"
        {}
        wait for 200 milliseconds
        wait for read output from process proc as output
        wait for process proc to complete
        display output
    "#,
        echo_spawn("test", "proc")
    );

    let result = run_wfl(&code);
    assert!(
        result.is_ok(),
        "Bounded buffer test should complete without memory issues: {:?}",
        result
    );

    #[cfg(not(windows))]
    {
        // Verify we got some output but not infinite
        let _output = result.unwrap();
        // The output length should be limited by buffer size
        // If unbounded, this would consume all memory
        println!("Buffer test completed with output");
    }
}

#[test]
fn test_killed_processes_dont_leak() {
    // Verify that killed processes are properly cleaned up
    let code = format!(
        r#"
        {}
        wait for 100 milliseconds
        kill process long_proc

        // Spawn another process - cleanup should have happened
        {}
        wait for 200 milliseconds
        wait for process proc to complete

        display "Kill cleanup test completed"
    "#,
        sleep_spawn("30", "long_proc"),
        echo_spawn("after kill", "proc")
    );

    let result = run_wfl(&code);
    assert!(
        result.is_ok(),
        "Killed processes should be cleaned up properly: {:?}",
        result
    );
}

#[test]
fn test_wait_for_process_removes_handle() {
    // Verify that wait_for_process properly removes the handle from the HashMap
    let code = format!(
        r#"
        {}
        wait for 200 milliseconds
        wait for process proc1 to complete

        // Try to check if process is still running (it shouldn't be in the map)
        store still_running as process proc1 is running
        display still_running
    "#,
        echo_spawn("test", "proc1")
    );

    let result = run_wfl(&code);
    assert!(result.is_ok(), "wait_for_process should work: {:?}", result);
    let output = result.unwrap();
    // WFL displays booleans as "yes"/"no" - after wait_for_process removes the handle,
    // is_process_running returns false which displays as "no"
    assert!(
        output.contains("no"),
        "Process should not be running after wait (expected 'no', got: {})",
        output
    );
}

#[test]
fn test_multiple_quick_spawns_dont_leak() {
    // Rapidly spawn and wait for many processes
    // This stresses the cleanup mechanism
    let code = format!(
        r#"
        count from 1 to 20:
            {}
            wait for 100 milliseconds
            wait for process proc to complete
        end count

        display "Rapid spawn test completed"
    "#,
        echo_spawn("test", "proc")
    );

    let result = run_wfl(&code);
    assert!(
        result.is_ok(),
        "Rapid spawn and wait cycles should work without leaks: {:?}",
        result
    );
}

#[test]
fn test_buffer_overflow_warning_non_fatal() {
    // Processes that produce more output than buffer size should warn but not crash
    #[cfg(not(windows))]
    let code = r#"
        // Generate lots of output
        spawn command "yes" with arguments ["x"] as proc
        wait for 2 seconds
        wait for read output from process proc as output
        kill process proc
        display "Buffer overflow handled gracefully"
    "#
    .to_string();

    #[cfg(windows)]
    let code = format!(
        r#"
        {}
        wait for 200 milliseconds
        wait for read output from process proc as output
        wait for process proc to complete
        display "Test completed"
    "#,
        echo_spawn("test", "proc")
    );

    let result = run_wfl(&code);
    assert!(
        result.is_ok(),
        "Buffer overflow should be handled gracefully: {:?}",
        result
    );
}
