//! Real CLI regressions for file-handle lifecycle and text path compatibility.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;

mod common;

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct RestoreFilePermissions {
    path: PathBuf,
    permissions: fs::Permissions,
}

impl Drop for RestoreFilePermissions {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.path, self.permissions.clone());
    }
}

fn run_wfl(directory: &Path, source: &str) -> Output {
    fs::write(directory.join("main.wfl"), source).expect("write WFL fixture");
    let global_config = NamedTempFile::new().expect("isolated global configuration");
    fs::write(
        global_config.path(),
        "logging_enabled = false\nexecution_logging = false\ndebug_report_enabled = false\n",
    )
    .expect("disable unrelated log artifacts in the file fixture");
    let stdout = NamedTempFile::new().expect("stdout capture");
    let stderr = NamedTempFile::new().expect("stderr capture");
    // Capture to files so a child cannot fill a pipe and deadlock the timeout.
    let mut child = ChildGuard(
        Command::new(common::wfl_exe())
            .arg("main.wfl")
            .current_dir(directory)
            .env("WFL_GLOBAL_CONFIG_PATH", global_config.path())
            .stdin(Stdio::null())
            .stdout(stdout.reopen().expect("open stdout capture"))
            .stderr(stderr.reopen().expect("open stderr capture"))
            .spawn()
            .expect("start WFL CLI"),
    );
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait().expect("poll WFL CLI") {
            break status;
        }
        if start.elapsed() >= Duration::from_secs(30) {
            child.0.kill().expect("kill timed-out WFL CLI");
            child.0.wait().expect("reap timed-out WFL CLI");
            panic!(
                "WFL CLI exceeded 30 seconds\nstdout: {}\nstderr: {}",
                fs::read_to_string(stdout.path()).unwrap_or_default(),
                fs::read_to_string(stderr.path()).unwrap_or_default(),
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: fs::read(stdout.path()).expect("read stdout capture"),
        stderr: fs::read(stderr.path()).expect("read stderr capture"),
    }
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "WFL CLI failed: {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn assert_closed_operation_rejected(mode: &str, operation: &str, original: &[u8]) {
    let directory = tempfile::tempdir().expect("isolated lifecycle fixture");
    let original_path = directory.path().join("original.dat");
    fs::write(&original_path, original).expect("write original bytes");
    let source = format!(
        r#"
open file at "original.dat" for {mode} as closed_handle
store handle_alias as closed_handle
close file closed_handle
try:
    {operation}
when error:
    display "REJECTED"
end try
display "DONE"
"#,
    );
    let output = run_wfl(directory.path(), &source);
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["REJECTED", "DONE"],
        "operation must reach the error handler: {operation}",
    );
    assert_eq!(
        fs::read(&original_path).expect("read original bytes"),
        original,
        "a rejected operation must not change the original file",
    );
    assert!(
        !directory.path().join("file1").exists(),
        "a closed handle must not become a new filesystem path",
    );
    let mut names: Vec<_> = fs::read_dir(directory.path())
        .expect("inspect lifecycle fixture")
        .map(|entry| entry.expect("fixture entry").file_name())
        .collect();
    names.sort();
    assert_eq!(names, ["main.wfl", "original.dat"]);
}

#[test]
fn closed_text_read_is_rejected_without_creating_a_file() {
    assert_closed_operation_rejected(
        "reading",
        "wait for store actual as read content from handle_alias",
        b" original content\nwith trailing whitespace \n",
    );
}

#[test]
fn closed_text_write_is_rejected_without_changing_files() {
    assert_closed_operation_rejected(
        "appending",
        "wait for write content \"must not be written\" into handle_alias",
        b"original content\n",
    );
}

#[test]
fn closed_append_is_rejected_without_changing_files() {
    assert_closed_operation_rejected(
        "appending",
        "wait for append content \"must not be appended\" into handle_alias",
        b"original content\n",
    );
}

#[test]
fn closed_binary_read_is_rejected_without_changing_files() {
    assert_closed_operation_rejected(
        "reading binary",
        "store actual as read binary from handle_alias",
        &[0, 1, 127, 128, 255],
    );
}

#[test]
fn closed_partial_binary_read_is_rejected_without_changing_files() {
    assert_closed_operation_rejected(
        "reading binary",
        "store actual as read 2 bytes from handle_alias",
        &[0, 1, 127, 128, 255],
    );
}

#[test]
fn closed_binary_write_is_rejected_without_changing_files() {
    assert_closed_operation_rejected(
        "reading binary",
        "write binary [42, 43] into handle_alias",
        &[0, 1, 127, 128, 255],
    );
}

#[test]
fn repeated_close_succeeds_and_preserves_exact_contents() {
    let directory = tempfile::tempdir().expect("isolated close fixture");
    let output = run_wfl(
        directory.path(),
        r#"
open file at "original.txt" for writing as my_handle
wait for write content " first line\nsecond line \n" into my_handle
close file my_handle
close file my_handle
open file at "original.txt" for reading as reader_handle
wait for store actual as read content from reader_handle
close file reader_handle
display actual
"#,
    );
    assert_success(&output);
    assert_eq!(
        fs::read(directory.path().join("original.txt")).expect("read closed file"),
        b" first line\nsecond line \n",
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"),
        " first line\nsecond line \n\n",
    );
    assert!(!directory.path().join("file1").exists());
}

#[test]
fn direct_text_paths_keep_literal_and_variable_compatibility() {
    let directory = tempfile::tempdir().expect("isolated path shorthand fixture");
    let output = run_wfl(
        directory.path(),
        r#"
wait for write content "literal file1" into "file1"
wait for store first_result as read content from "file1"
display first_result
wait for write content "second use of file1" into "file1"
wait for store second_result as read content from "file1"
display second_result
store ordinary_path as "ordinary.txt"
wait for write content "variable path" into ordinary_path
wait for store variable_result as read content from ordinary_path
display variable_result
wait for write content "file prefix path" into "file-not-a-handle.txt"
wait for store prefix_result as read content from "file-not-a-handle.txt"
display prefix_result
"#,
    );
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .collect::<Vec<_>>(),
        [
            "literal file1",
            "second use of file1",
            "variable path",
            "file prefix path",
        ],
    );
    for (name, expected) in [
        ("file1", "second use of file1"),
        ("ordinary.txt", "variable path"),
        ("file-not-a-handle.txt", "file prefix path"),
    ] {
        assert_eq!(
            fs::read_to_string(directory.path().join(name)).expect("read shorthand output"),
            expected,
        );
    }
}

#[test]
fn path_variable_named_file1_remains_valid_after_a_handle_is_closed() {
    let directory = tempfile::tempdir().expect("isolated path collision fixture");
    let output = run_wfl(
        directory.path(),
        r#"
open file at "original.txt" for writing as original_handle
wait for write content "original remains intact" into original_handle
close file original_handle
store collision_path as "file1"
wait for write content "independent path" into collision_path
wait for store actual as read content from collision_path
display actual
"#,
    );
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "independent path",
    );
    assert_eq!(
        fs::read(directory.path().join("original.txt")).expect("read original file"),
        b"original remains intact",
    );
    assert_eq!(
        fs::read(directory.path().join("file1")).expect("read independent path"),
        b"independent path",
    );
}

#[test]
fn successive_partial_binary_reads_advance_the_same_handle() {
    let directory = tempfile::tempdir().expect("isolated binary cursor fixture");
    fs::write(directory.path().join("input.bin"), [0, 1, 127, 128, 255])
        .expect("write binary input");
    let output = run_wfl(
        directory.path(),
        r#"
open file at "input.bin" for reading binary as reader_handle
store first_chunk as read 2 bytes from reader_handle
store second_chunk as read 3 bytes from reader_handle
store final_chunk as read 1 bytes from reader_handle
close file reader_handle
open file at "first.bin" for writing binary as first_writer
write binary first_chunk into first_writer
close file first_writer
open file at "second.bin" for writing binary as second_writer
write binary second_chunk into second_writer
close file second_writer
display length of final_chunk
"#,
    );
    assert_success(&output);
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "0");
    assert_eq!(
        fs::read(directory.path().join("first.bin")).expect("read first chunk"),
        [0, 1],
    );
    assert_eq!(
        fs::read(directory.path().join("second.bin")).expect("read second chunk"),
        [127, 128, 255],
    );
    assert_eq!(
        fs::read(directory.path().join("input.bin")).expect("read original binary input"),
        [0, 1, 127, 128, 255],
    );
}

fn assert_missing_direct_path_read_is_rejected(path_expression: &str) {
    let directory = tempfile::tempdir().expect("isolated missing-path fixture");
    let output = run_wfl(
        directory.path(),
        &format!(
            r#"
store missing_path as "missing.txt"
try:
    wait for store actual as read content from {path_expression}
when error:
    display "REJECTED"
end try
display "DONE"
"#,
        ),
    );
    assert_success(&output);
    assert!(
        !directory.path().join("missing.txt").exists(),
        "a missing direct read path must not be created by reading it",
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["REJECTED", "DONE"],
        "a missing direct read path must reach the error handler",
    );
}

#[test]
fn direct_path_read_missing_literal_reports_error_without_creating_file() {
    assert_missing_direct_path_read_is_rejected("\"missing.txt\"");
}

#[test]
fn direct_path_read_missing_variable_reports_error_without_creating_file() {
    assert_missing_direct_path_read_is_rejected("missing_path");
}

#[test]
fn direct_path_read_readonly_file_preserves_exact_contents() {
    let directory = tempfile::tempdir().expect("isolated readonly fixture");
    let path = directory.path().join("readonly.txt");
    let original = b" first line\r\nsecond line \t\n";
    fs::write(&path, original).expect("write readonly input");
    let permissions = fs::metadata(&path)
        .expect("inspect input permissions")
        .permissions();
    // Restore the original permissions before TempDir cleanup, including when
    // the child or an assertion fails; Windows cannot delete readonly files.
    let _restore_permissions = RestoreFilePermissions {
        path: path.clone(),
        permissions: permissions.clone(),
    };
    let mut readonly = permissions;
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly).expect("make input readonly");
    let output = run_wfl(
        directory.path(),
        r#"
wait for store literal_result as read content from "readonly.txt"
display literal_result
store source_path as "readonly.txt"
wait for store variable_result as read content from source_path
display variable_result
"#,
    );
    assert_success(&output);
    assert_eq!(
        output.stdout,
        [original.as_slice(), b"\n", original.as_slice(), b"\n"].concat(),
        "both literal and variable direct paths must read the exact bytes",
    );
    assert_eq!(fs::read(&path).expect("read original input"), original);
    assert!(
        fs::metadata(&path)
            .expect("inspect readonly input")
            .permissions()
            .readonly(),
        "reading must not change file permissions",
    );
}
