use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::tempdir;

#[test]
fn test_no_step_flag_regression() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test_script.wfl");

    let test_content = r#"
store x as 42
display x
"#;
    fs::write(&file_path, test_content).expect("Failed to write test file");

    let output_no_step = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .args([file_path.to_str().unwrap()])
        .output()
        .expect("Failed to execute command");

    assert!(
        output_no_step.status.success(),
        "Command failed without --step flag"
    );

    let output_str = String::from_utf8_lossy(&output_no_step.stdout);
    assert!(
        !output_str.contains("continue (y/n)?"),
        "Output shouldn't contain step mode prompts: {output_str}"
    );
}

#[test]
fn test_step_flag_with_input() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test_script.wfl");

    let test_content = r#"
store x as 42
display x
store y as 100
"#;
    fs::write(&file_path, test_content).expect("Failed to write test file");

    let mut child = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .args(["--step", file_path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn command");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(b"y\ny\ny\ny\ny\n")
            .expect("Failed to write to stdin");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains("continue (y/n)?"),
        "Output should contain step mode prompts: {output_str}"
    );

    let prompt_count = output_str.matches("continue (y/n)?").count();
    assert!(
        prompt_count >= 1,
        "Expected at least 1 prompt, got {prompt_count}"
    );
}

#[test]
fn test_function_call_stack() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test_script.wfl");

    let test_content = r#"
define action called main:
    store x as 10
    helper_function with x
end action

define action called helper_function needs v:
    display "In helper with value: " with v
    nested_function
end action

define action called nested_function:
    display "In nested function"
end action

main
"#;
    fs::write(&file_path, test_content).expect("Failed to write test file");

    let mut child = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .args(["--step", file_path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn command");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        for _ in 0..20 {
            stdin.write_all(b"y\n").expect("Failed to write to stdin");
        }
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains("Boot phase: Configuration loaded"),
        "Output should show boot phase: {output_str}"
    );
    assert!(
        output_str.contains("continue (y/n)?"),
        "Output should contain prompts: {output_str}"
    );
    assert!(
        output_str.contains("Program has 4 statements"),
        "Output should show program statement count: {output_str}"
    );
}

#[test]
fn test_loop_iteration() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test_script.wfl");

    let test_content = r#"
count from 1 to 3:
    store loopcounter as count
    display "Count: " with loopcounter
end count
"#;
    fs::write(&file_path, test_content).expect("Failed to write test file");

    let mut child = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .args(["--step", file_path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn command");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        for _ in 0..15 {
            stdin.write_all(b"y\n").expect("Failed to write to stdin");
        }
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains("loopcounter"),
        "Output should show loopcounter variable: {output_str}"
    );
    assert!(
        output_str.contains("Count: 1"),
        "Output should show Count: 1: {output_str}"
    );
}

#[test]
fn test_invalid_input_handling() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test_script.wfl");

    let test_content = r#"
store x as 42
display x
"#;
    fs::write(&file_path, test_content).expect("Failed to write test file");

    let mut child = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .args(["--step", file_path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn command");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(b"foo\nbar\ny\nn\n")
            .expect("Failed to write to stdin");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    let output_str = String::from_utf8_lossy(&output.stdout);
    let prompt_count = output_str.matches("continue (y/n)?").count();
    assert!(
        prompt_count >= 1,
        "Expected at least one prompt, got {prompt_count}"
    );
}
