//! A WFL CLI can report an intentional failure without a runtime-error workaround.

mod common;

use common::wfl_exe;
use std::{fs, process::Command};
use tempfile::TempDir;

#[test]
fn intentional_failure_writes_only_custom_stderr_and_exits_nonzero() {
    let dir = TempDir::new().expect("tempdir");
    let script = dir.path().join("diagnostic.wfl");
    fs::write(
        &script,
        "store pos as 42\n\
         call print_error with (\"jshrink: Unclosed string at position: \" with pos)\n\
         exit program with code 1\n",
    )
    .expect("write WFL script");

    let output = Command::new(wfl_exe())
        .arg(&script)
        .current_dir(dir.path())
        .output()
        .expect("run WFL CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(output.stdout.is_empty(), "stdout must stay empty");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        ["jshrink: Unclosed string at position: 42"]
    );
    assert!(!dir.path().join("diagnostic_debug.txt").exists());
}

#[test]
fn dynamic_nontext_message_is_rejected() {
    let dir = TempDir::new().expect("tempdir");
    let script = dir.path().join("invalid_message.wfl");
    fs::write(
        &script,
        "store message as parse_json of \"42\"\ncall print_error with message\n",
    )
    .expect("write WFL script");

    let output = Command::new(wfl_exe())
        .arg(&script)
        .current_dir(dir.path())
        .output()
        .expect("run WFL CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(output.stdout.is_empty(), "stdout must stay empty");
    assert!(stderr.contains("print_error expects text"), "{stderr}");
}
