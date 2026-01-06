use std::process::Command;
use std::time::Duration;

#[test]
fn test_version_flag_exits_immediately() {
    // Test that --version flag exits immediately and doesn't hang
    let output = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("wfl-lsp")
        .arg("--")
        .arg("--version")
        .current_dir("../") // Run from wfl root
        .output()
        .expect("Failed to execute wfl-lsp --version");

    // Check that the command completed (didn't hang)
    assert!(output.status.success() || output.status.code() == Some(0));

    // Check that version information is printed
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wfl-lsp") && stdout.contains("0.1.0"));
}

#[test]
fn test_help_flag_exits_immediately() {
    // Test that --help flag also exits immediately
    let output = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("wfl-lsp")
        .arg("--")
        .arg("--help")
        .current_dir("../") // Run from wfl root
        .output()
        .expect("Failed to execute wfl-lsp --help");

    // Check that the command completed (didn't hang)
    assert!(output.status.success() || output.status.code() == Some(0));

    // Check that help information is printed
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wfl-lsp"));
}

#[test]
fn test_version_flag_with_timeout() {
    // Test that --version completes within a reasonable time (not hanging)
    let start = std::time::Instant::now();

    let output = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("wfl-lsp")
        .arg("--")
        .arg("--version")
        .current_dir("../")
        .output()
        .expect("Failed to execute wfl-lsp --version");

    let elapsed = start.elapsed();

    // Should complete within 5 seconds (way more than needed for version display)
    assert!(
        elapsed < Duration::from_secs(5),
        "Command took too long: {:?}",
        elapsed
    );

    // Should exit with code 0
    assert!(output.status.success());
}
