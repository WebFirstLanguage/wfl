use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_lint_fix_diff_combined() {
    // Create a temporary directory for the test
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test_file.wfl");

    // Create a test file with a style issue (camelCase variable name)
    let test_content = r#"
store myVariable as 42
display myVariable
"#;

    fs::write(&file_path, test_content).expect("Failed to write test file");

    assert!(
        Command::new("cargo")
            .args(["build"])
            .status()
            .expect("Failed to build binary")
            .success(),
        "Failed to build the binary"
    );

    let binary_path = std::env::current_dir()
        .expect("Failed to get current directory")
        .join("target/debug/wfl");

    // Run the binary with --lint file_path --fix --diff
    let file_path_str = file_path.to_str().unwrap();
    println!("Running: {binary_path:?} --lint {file_path_str} --fix {file_path_str} --diff");

    let output = Command::new(binary_path)
        .args(["--lint", file_path_str, "--fix", file_path_str, "--diff"])
        .output()
        .expect("Failed to execute command");

    // Check that the command succeeded
    assert!(output.status.success(), "Command failed: {output:?}");

    // Convert output to string
    let output_str = String::from_utf8_lossy(&output.stdout);

    // Check that the diff contains the expected replacement
    assert!(
        output_str.contains("-store myVariable as 42"),
        "Diff doesn't contain the original line: {output_str}"
    );
    assert!(
        output_str.contains("+store my_variable as 42"),
        "Diff doesn't contain the fixed line: {output_str}"
    );
}
