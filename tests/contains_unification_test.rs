use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn get_wfl_binary_path() -> PathBuf {
    let current_dir = env::current_dir().unwrap();
    let release_path = current_dir.join(if cfg!(target_os = "windows") {
        "target/release/wfl.exe"
    } else {
        "target/release/wfl"
    });
    if release_path.exists() {
        return release_path;
    }
    let debug_path = current_dir.join(if cfg!(target_os = "windows") {
        "target/debug/wfl.exe"
    } else {
        "target/debug/wfl"
    });
    if debug_path.exists() {
        return debug_path;
    }
    panic!("WFL binary not found.");
}

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn get_unique_test_file_path(prefix: &str) -> PathBuf {
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

fn cleanup_temp_file_with_retry(file_path: &PathBuf, max_retries: u32) {
    for attempt in 0..=max_retries {
        if fs::remove_file(file_path).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

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
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(child.wait_with_output()?),
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "Timeout"));
                } else {
                    thread::sleep(Duration::from_millis(100));
                }
            }
            Err(e) => return Err(e),
        }
    }
}

fn run_wfl_program(program_content: &str, test_name: &str) -> std::process::Output {
    let binary_path = get_wfl_binary_path();
    let test_file = get_unique_test_file_path(test_name);
    fs::write(&test_file, program_content).expect("Failed to write test file");
    let output = execute_with_timeout(&binary_path, &test_file, Duration::from_secs(30))
        .expect("Failed to execute WFL binary");
    cleanup_temp_file_with_retry(&test_file, 3);
    output
}

#[test]
fn test_contains_list_and_text_in_same_scope() {
    let source = r#"
create list myList:
    add 1
end list

store myText as "hello world"

store listHas1 as contains 1 in myList
store textHasWorld as contains "world" in myText

display listHas1
display textHasWorld
"#;

    // Use a slightly different syntax for list creation which might be less error prone if `store myList as create list` is flaky
    // But wait, the previous reproduction used `store myList as create list:`.
    // Let's use the explicit `create list` statement syntax which is definitely standard.

    let output = run_wfl_program(source, "test_contains_list_and_text");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT:\n{}", stdout);
    println!("STDERR:\n{}", stderr);

    assert!(
        stdout.contains("yes") || stdout.contains("true"),
        "Should contain true output"
    );

    // Check that we have two positive outputs
    // Note: boolean true is printed as "yes" in WFL
    let matches: Vec<_> = stdout.match_indices("yes").collect();
    let matches_true: Vec<_> = stdout.match_indices("true").collect();

    if matches.len() + matches_true.len() < 2 {
        panic!("Should have two true/yes outputs. Got: {}", stdout);
    }
}

#[test]
fn test_contains_type_checking() {
    // This test ensures the type checker accepts both usages
    // Note: Inline list creation might not be supported in expression context?
    // Let's use variables to be safe and match standard WFL patterns
    let source = r#"
create list myList:
    add "item"
end list

store listResult as contains "item" in myList
store textResult as contains "sub" in "substring"
display "Types check OK"
"#;

    let output = run_wfl_program(source, "test_contains_type_checking");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.contains("Types check OK") {
        println!("Stdout: {}", stdout);
        println!("Stderr: {}", stderr);
        panic!("Type checking failed or program execution failed");
    }
}
