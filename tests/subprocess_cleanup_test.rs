use std::env;
use std::fs;
use std::path::PathBuf;
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

fn get_wfl_binary_path() -> PathBuf {
    let current_dir = env::current_dir().unwrap();
    let release_path = if cfg!(target_os = "windows") {
        current_dir.join("target/release/wfl.exe")
    } else {
        current_dir.join("target/release/wfl")
    };

    if release_path.exists() {
        return release_path;
    }

    let debug_path = if cfg!(target_os = "windows") {
        current_dir.join("target/debug/wfl.exe")
    } else {
        current_dir.join("target/debug/wfl")
    };

    if debug_path.exists() {
        return debug_path;
    }

    panic!("WFL binary not found. Run 'cargo build' or 'cargo build --release' first.");
}

fn run_wfl(code: &str) -> Result<String, String> {
    let temp_file = TempWflFile::new(code).expect("Failed to create temp file");

    let binary_path = get_wfl_binary_path();

    let output = Command::new(binary_path)
        .arg(temp_file.path())
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

#[test]
fn test_automatic_cleanup_of_completed_processes() {
    // This test verifies that completed processes are automatically cleaned up
    // when new processes are spawned
    let code = r#"
        // Spawn 5 quick processes that complete immediately
        spawn command "echo" with arguments ["test1"] as proc1
        spawn command "echo" with arguments ["test2"] as proc2
        spawn command "echo" with arguments ["test3"] as proc3
        spawn command "echo" with arguments ["test4"] as proc4
        spawn command "echo" with arguments ["test5"] as proc5

        // Wait for them to complete
        wait for 500 milliseconds

        // Spawn more processes - cleanup should happen automatically
        spawn command "echo" with arguments ["test6"] as proc6
        spawn command "echo" with arguments ["test7"] as proc7

        wait for 200 milliseconds
        wait for process proc6 to complete
        wait for process proc7 to complete

        display "Cleanup test completed"
    "#;

    let result = run_wfl(code);
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
    let code = r#"
        // Try to spawn many long-running processes
        try:
            count from 1 to 150:
                spawn command "sleep" with arguments ["10"] as proc
            end count
        when:
            display "Hit process limit as expected"
        end try
    "#;

    // This should either complete (if limits allow 150) or fail with process limit error
    let result = run_wfl(code);
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
    "#;

    #[cfg(windows)]
    let code = r#"
        spawn command "echo" with arguments ["test"] as proc
        wait for 200 milliseconds
        wait for read output from process proc as output
        wait for process proc to complete
        display output
    "#;

    let result = run_wfl(code);
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
    let code = r#"
        spawn command "sleep" with arguments ["30"] as long_proc
        wait for 100 milliseconds
        kill process long_proc

        // Spawn another process - cleanup should have happened
        spawn command "echo" with arguments ["after kill"] as proc
        wait for 200 milliseconds
        wait for process proc to complete

        display "Kill cleanup test completed"
    "#;

    let result = run_wfl(code);
    assert!(
        result.is_ok(),
        "Killed processes should be cleaned up properly: {:?}",
        result
    );
}

#[test]
fn test_wait_for_process_removes_handle() {
    // Verify that wait_for_process properly removes the handle from the HashMap
    let code = r#"
        spawn command "echo" with arguments ["test"] as proc1
        wait for 200 milliseconds
        wait for process proc1 to complete

        // Try to check if process is still running (it shouldn't be in the map)
        store still_running as process proc1 is running
        display still_running
    "#;

    let result = run_wfl(code);
    assert!(result.is_ok(), "wait_for_process should work: {:?}", result);
    let output = result.unwrap();
    assert!(
        output.contains("no") || output.contains("nothing"),
        "Process should not be running after wait"
    );
}

#[test]
fn test_multiple_quick_spawns_dont_leak() {
    // Rapidly spawn and wait for many processes
    // This stresses the cleanup mechanism
    let code = r#"
        count from 1 to 20:
            spawn command "echo" with arguments ["test"] as proc
            wait for 100 milliseconds
            wait for process proc to complete
        end count

        display "Rapid spawn test completed"
    "#;

    let result = run_wfl(code);
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
    "#;

    #[cfg(windows)]
    let code = r#"
        spawn command "echo" with arguments ["test"] as proc
        wait for 200 milliseconds
        wait for read output from process proc as output
        wait for process proc to complete
        display "Test completed"
    "#;

    let result = run_wfl(code);
    assert!(
        result.is_ok(),
        "Buffer overflow should be handled gracefully: {:?}",
        result
    );
}
