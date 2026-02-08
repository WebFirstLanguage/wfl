use std::process::Command;
use std::time::Duration;

#[test]
fn test_version_flag_exits_immediately() {
    // Test that --version flag exits immediately and doesn't hang
    let output = Command::new(env!("CARGO_BIN_EXE_wfl-lsp"))
        .arg("--version")
        .output()
        .expect("Failed to execute wfl-lsp --version");

    // Check that the command completed successfully (didn't hang)
    assert!(output.status.success());

    // Check that version information is printed
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wfl-lsp"));
    // Use dynamic version from environment instead of hardcoded
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_help_flag_exits_immediately() {
    // Test that --help flag also exits immediately
    let output = Command::new(env!("CARGO_BIN_EXE_wfl-lsp"))
        .arg("--help")
        .output()
        .expect("Failed to execute wfl-lsp --help");

    // Check that the command completed successfully (didn't hang)
    assert!(output.status.success());

    // Check that help information is printed
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wfl-lsp"));
}

#[test]
fn test_version_flag_with_timeout() {
    // Test that --version completes within a reasonable time (not hanging)
    let start = std::time::Instant::now();

    let output = Command::new(env!("CARGO_BIN_EXE_wfl-lsp"))
        .arg("--version")
        .output()
        .expect("Failed to execute wfl-lsp --version");

    let elapsed = start.elapsed();

    // Should complete within 5 seconds (no compilation overhead)
    assert!(
        elapsed < Duration::from_secs(5),
        "Command took too long: {:?}",
        elapsed
    );

    // Should exit successfully
    assert!(output.status.success());
}
