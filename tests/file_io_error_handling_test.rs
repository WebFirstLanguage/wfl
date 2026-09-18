mod common;

use std::fs;
use std::path::Path;
use tokio::time::{Duration, timeout};
use wfl::Interpreter;
use wfl::interpreter::value::Value;

fn wfl_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

async fn execute(code: &str) -> Interpreter {
    timeout(Duration::from_secs(5), common::run_wfl(code))
        .await
        .expect("Operation timed out")
        .expect("file I/O program failed")
}

fn assert_flag(interpreter: &Interpreter, name: &str, expected: bool) {
    assert!(
        matches!(common::get_global(interpreter, name), Value::Bool(value) if value == expected),
        "{name} must be {expected}"
    );
}

async fn expect_error(operation: &str) {
    let interpreter = execute(&format!(
        r#"
        store caught_error as false
        store completed_operation as false
        try:
            {operation}
            change completed_operation to true
        when error:
            change caught_error to true
        end try
        "#
    ))
    .await;
    assert_flag(&interpreter, "caught_error", true);
    assert_flag(&interpreter, "completed_operation", false);
}

#[tokio::test]
async fn test_nonexistent_file_read_error() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("missing.txt");
    expect_error(&format!(
        "open file at \"{}\" for reading as missing_file",
        wfl_path(&path)
    ))
    .await;
    assert!(!path.exists(), "a failed read must not create a file");
}

#[tokio::test]
async fn test_invalid_file_path_error() {
    let directory = tempfile::tempdir().unwrap();
    let missing_parent = directory.path().join("missing/child.txt");
    for path in [
        wfl_path(&missing_parent),
        String::new(),
        format!("{}/file\0name.txt", wfl_path(directory.path())),
        wfl_path(directory.path()),
    ] {
        expect_error(&format!(
            "open file at \"{path}\" for writing as invalid_file"
        ))
        .await;
    }
    assert!(!missing_parent.exists());

    // Reserved-name behavior depends on the native path API (including its
    // Windows extended-path normalization). Match that API's actual outcome.
    let reserved = directory.path().join("con");
    let rejected_by_os = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&reserved)
    {
        Ok(file) => {
            drop(file);
            fs::remove_file(&reserved).unwrap();
            false
        }
        Err(_) => true,
    };
    let operation = format!(
        r#"
        open file at "{}" for writing as reserved_file
        wait for write content "platform semantics" into reserved_file
        close file reserved_file
        "#,
        wfl_path(&reserved)
    );
    if rejected_by_os {
        expect_error(&operation).await;
        assert!(!reserved.exists());
    } else {
        execute(&operation).await;
        assert_eq!(fs::read_to_string(reserved).unwrap(), "platform semantics");
    }
}

// Restore permissions before TempDir is dropped, including during a panic.
struct RestorePermissions(std::path::PathBuf, fs::Permissions);

impl Drop for RestorePermissions {
    fn drop(&mut self) {
        fs::set_permissions(&self.0, self.1.clone()).expect("restore fixture permissions");
    }
}

#[test]
fn permission_restore_cleanup_tolerates_a_removed_fixture() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("removed.txt");
    fs::write(&path, "fixture").unwrap();
    let permissions = fs::metadata(&path).unwrap().permissions();
    let restore = RestorePermissions(path.clone(), permissions);
    fs::remove_file(&path).unwrap();

    let result = std::panic::catch_unwind(|| drop(restore));
    assert!(
        result.is_ok(),
        "permission cleanup must not panic when a fixture has already been removed"
    );
    assert!(
        !path.exists(),
        "cleanup must not recreate a removed fixture"
    );
}

#[test]
fn permission_restore_cleanup_restores_original_permissions() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("readonly.txt");
    fs::write(&path, "fixture").unwrap();
    let original = fs::metadata(&path).unwrap().permissions();
    let restore = RestorePermissions(path.clone(), original.clone());
    let mut readonly = original.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly).unwrap();

    drop(restore);

    assert_eq!(fs::metadata(&path).unwrap().permissions(), original);
    assert_eq!(fs::read_to_string(&path).unwrap(), "fixture");
}

#[tokio::test]
async fn test_write_to_readonly_file_error() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("readonly.txt");
    fs::write(&path, "Initial content").unwrap();
    let original = fs::metadata(&path).unwrap().permissions();
    let _restore = RestorePermissions(path.clone(), original.clone());
    let mut readonly = original;
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly).unwrap();

    // Privileged Unix users may bypass mode bits. Require the same outcome as
    // the OS access check instead of silently accepting either WFL branch.
    let denied_by_os = fs::OpenOptions::new().write(true).open(&path).is_err();
    let operation = format!(
        r#"
        open file at "{}" for writing as readonly_file
        wait for write content "Replacement content" into readonly_file
        close file readonly_file
        "#,
        wfl_path(&path)
    );
    if denied_by_os {
        expect_error(&operation).await;
        assert_eq!(fs::read_to_string(path).unwrap(), "Initial content");
    } else {
        execute(&operation).await;
        assert_eq!(fs::read_to_string(path).unwrap(), "Replacement content");
    }
}

#[tokio::test]
async fn test_write_to_read_handle_error() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("read_handle.txt");
    fs::write(&path, "Test content").unwrap();
    expect_error(&format!(
        r#"
        open file at "{}" for reading as read_only_file
        wait for write content "This should fail" into read_only_file
        "#,
        wfl_path(&path)
    ))
    .await;
    assert_eq!(fs::read_to_string(path).unwrap(), "Test content");
}

#[tokio::test]
async fn test_double_close_file_error() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("double_close.txt");
    // Close is intentionally idempotent; require success and preserved bytes.
    execute(&format!(
        r#"
        open file at "{}" for writing as test_file
        wait for write content "Test content" into test_file
        close file test_file
        close file test_file
        "#,
        wfl_path(&path)
    ))
    .await;
    assert_eq!(fs::read_to_string(path).unwrap(), "Test content");
}

#[tokio::test]
async fn test_use_closed_file_handle_error() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("closed_handle.txt");
    expect_error(&format!(
        r#"
        open file at "{}" for writing as test_file
        wait for write content "Initial content" into test_file
        close file test_file
        wait for write content "This should fail" into test_file
        "#,
        wfl_path(&path)
    ))
    .await;
    assert_eq!(fs::read_to_string(path).unwrap(), "Initial content");
}

#[tokio::test]
async fn test_disk_full_simulation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("large_write.txt");
    // This is a large-write regression, not a simulation of an exhausted disk.
    // An unexpected write failure must fail instead of entering a catch-all.
    let content = "x".repeat(10_000);
    execute(&format!(
        r#"
        open file at "{}" for writing as large_file
        wait for write content "{content}" into large_file
        close file large_file
        "#,
        wfl_path(&path)
    ))
    .await;
    assert_eq!(fs::read_to_string(path).unwrap(), content);
}

#[tokio::test]
async fn test_concurrent_access_same_file_error() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("same_file.txt");
    // Multiple handles are supported. Ordered writes leave the second write.
    execute(&format!(
        r#"
        open file at "{path}" for writing as file1
        open file at "{path}" for writing as file2
        wait for write content "From file1" into file1
        wait for write content "From file2" into file2
        close file file1
        close file file2
        "#,
        path = wfl_path(&path)
    ))
    .await;
    assert_eq!(fs::read_to_string(path).unwrap(), "From file2");
}

#[tokio::test]
async fn test_delete_nonexistent_file_error() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("missing.txt");
    expect_error(&format!("delete file at \"{}\"", wfl_path(&path))).await;
    assert!(!path.exists());
}

#[tokio::test]
async fn test_nested_error_handling() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("nested.txt");
    let missing = directory.path().join("missing/nested.txt");
    let interpreter = execute(&format!(
        r#"
        store caught_inner as false
        store caught_outer as false
        store completed_outer as false
        try:
            open file at "{}" for writing as test_file
            wait for write content "Outer try content" into test_file
            close file test_file
            try:
                open file at "{}" for reading as missing_file
            when error:
                change caught_inner to true
            end try
            change completed_outer to true
        when error:
            change caught_outer to true
        end try
        "#,
        wfl_path(&path),
        wfl_path(&missing)
    ))
    .await;
    assert_flag(&interpreter, "caught_inner", true);
    assert_flag(&interpreter, "caught_outer", false);
    assert_flag(&interpreter, "completed_outer", true);
    assert_eq!(fs::read_to_string(path).unwrap(), "Outer try content");
    assert!(!missing.exists());
}
