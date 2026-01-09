use std::fs;
use tokio::time::{Duration, timeout};
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

// Tests for comprehensive error handling in file I/O operations
#[cfg(test)]
mod file_io_error_handling_tests {
    use super::*;

    fn cleanup_test_files(files: &[&str]) {
        for file in files {
            let _ = fs::remove_file(file);
        }
    }

    async fn execute_wfl_code_expect_success(
        code: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let ast = parser.parse().expect("Failed to parse WFL code");

        let mut interpreter = Interpreter::new();

        let result = timeout(Duration::from_secs(5), interpreter.interpret(&ast)).await;
        match result {
            Ok(Ok(_)) => Ok("Program executed successfully".to_string()),
            Ok(Err(errors)) => {
                let error_msg = errors
                    .iter()
                    .map(|e| format!("{}", e))
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(Box::new(std::io::Error::other(error_msg)))
            }
            Err(_) => Err(Box::new(std::io::Error::other("Operation timed out"))),
        }
    }

    #[tokio::test]
    async fn test_nonexistent_file_read_error() {
        let test_files = ["nonexistent_read_test.txt"];
        cleanup_test_files(&test_files); // Ensure file doesn't exist

        // This should fail when trying to read a non-existent file
        let code = r#"
            try:
                open file at "nonexistent_read_test.txt" for reading as missing_file
                wait for store file_data as read content from missing_file
                close file missing_file
                display "This should not execute"
            when error:
                display "Correctly caught file not found error"
            end try
        "#;

        let result = execute_wfl_code_expect_success(code).await;
        assert!(
            result.is_ok(),
            "Error handling for non-existent file read failed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_invalid_file_path_error() {
        // Test with various invalid file paths
        let invalid_paths = [
            "invalid/path/that/does/not/exist/file.txt",
            "",                 // Empty path
            "con",              // Reserved Windows filename
            "file\x00name.txt", // Null character in path
        ];

        for invalid_path in &invalid_paths {
            let code = format!(
                r#"
                try:
                    open file at "{}" for writing as invalid_file
                    wait for write content "This should fail" into invalid_file
                    close file invalid_file
                    display "This should not execute"
                when error:
                    display "Correctly caught invalid path error"
                end try
            "#,
                invalid_path
            );

            let result = execute_wfl_code_expect_success(&code).await;
            assert!(
                result.is_ok(),
                "Error handling for invalid path '{}' failed: {:?}",
                invalid_path,
                result.err()
            );

            // Clean up after each iteration in case the invalid path created unexpected files
            // For example, empty path "" might create "file1", "con" might create a file named "con"
            // Only include invalid_path in cleanup if it's a valid simple filename
            let mut files_to_cleanup = vec!["file1", "con"];
            // Validate that invalid_path is a simple filename without path separators or null bytes
            if !invalid_path.is_empty()
                && !invalid_path.contains('\0')
                && !invalid_path.contains('/')
                && !invalid_path.contains('\\')
            {
                files_to_cleanup.push(invalid_path);
            }
            cleanup_test_files(&files_to_cleanup);
        }

        // Final cleanup for any remaining test artifacts
        cleanup_test_files(&["file1", "con"]);
    }

    #[tokio::test]
    async fn test_write_to_readonly_file_error() {
        let test_files = ["readonly_test.txt"];
        cleanup_test_files(&test_files);

        // Create a file and try to make it read-only (platform dependent)
        fs::write("readonly_test.txt", "Initial content").expect("Failed to create test file");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata("readonly_test.txt").unwrap().permissions();
            perms.set_mode(0o444); // Read-only
            fs::set_permissions("readonly_test.txt", perms).expect("Failed to set permissions");
        }

        #[cfg(windows)]
        {
            // Windows file permissions are more complex, this test may behave differently
        }

        let code = r#"
            try:
                open file at "readonly_test.txt" for writing as readonly_file
                wait for write content "This should fail on read-only file" into readonly_file
                close file readonly_file
                display "This should not execute if file is truly read-only"
            when error:
                display "Correctly caught read-only file error"
            end try
        "#;

        let result = execute_wfl_code_expect_success(code).await;
        assert!(
            result.is_ok(),
            "Error handling for read-only file failed: {:?}",
            result.err()
        );

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_write_to_read_handle_error() {
        let test_files = ["read_handle_test.txt"];
        cleanup_test_files(&test_files);

        // Create a test file
        fs::write("read_handle_test.txt", "Test content").expect("Failed to create test file");

        // Try to write to a file opened for reading
        let code = r#"
            try:
                open file at "read_handle_test.txt" for reading as read_only_file
                wait for write content "This should fail" into read_only_file
                close file read_only_file
                display "This should not execute"
            when error:
                display "Correctly caught write to read handle error"
            end try
        "#;

        let result = execute_wfl_code_expect_success(code).await;
        assert!(
            result.is_ok(),
            "Error handling for writing to read handle failed: {:?}",
            result.err()
        );

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_double_close_file_error() {
        let test_files = ["double_close_test.txt"];
        cleanup_test_files(&test_files);

        // Try to close a file handle twice
        let code = r#"
            try:
                open file at "double_close_test.txt" for writing as test_file
                wait for write content "Test content" into test_file
                close file test_file
                close file test_file
                display "This should not execute if double close is an error"
            when error:
                display "Correctly caught double close error"
            end try
        "#;

        let result = execute_wfl_code_expect_success(code).await;
        assert!(
            result.is_ok(),
            "Error handling for double close failed: {:?}",
            result.err()
        );

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_use_closed_file_handle_error() {
        let test_files = ["closed_handle_test.txt"];
        cleanup_test_files(&test_files);

        // Try to use a file handle after closing it
        let code = r#"
            try:
                open file at "closed_handle_test.txt" for writing as test_file
                wait for write content "Initial content" into test_file
                close file test_file
                wait for write content "This should fail" into test_file
                display "This should not execute"
            when error:
                display "Correctly caught closed file handle error"
            end try
        "#;

        let result = execute_wfl_code_expect_success(code).await;
        assert!(
            result.is_ok(),
            "Error handling for closed file handle failed: {:?}",
            result.err()
        );

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_disk_full_simulation() {
        // This test is challenging to implement portably, but we can test large writes
        let test_files = ["large_write_test.txt"];
        cleanup_test_files(&test_files);

        // Try to write a moderately large string to test limits
        let large_content = "x".repeat(10_000); // 10KB of 'x' characters - more reasonable for testing
        let code = format!(
            r#"
            try:
                open file at "large_write_test.txt" for writing as large_file
                wait for write content "{}" into large_file
                close file large_file
                display "Large write completed successfully"
            when error:
                display "Correctly caught large write error (possibly disk full)"
            end try
        "#,
            large_content
        );

        let result = execute_wfl_code_expect_success(&code).await;
        assert!(
            result.is_ok(),
            "Large write test failed: {:?}",
            result.err()
        );

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_concurrent_access_same_file_error() {
        let test_files = ["concurrent_access_test.txt"];
        cleanup_test_files(&test_files);

        // Try to open the same file for writing multiple times (should this be an error?)
        let code = r#"
            try:
                open file at "concurrent_access_test.txt" for writing as file1
                open file at "concurrent_access_test.txt" for writing as file2
                wait for write content "From file1" into file1
                wait for write content "From file2" into file2
                close file file1
                close file file2
                display "Concurrent access completed - this behavior depends on implementation"
            when error:
                display "Correctly caught concurrent access error"
            end try
        "#;

        let result = execute_wfl_code_expect_success(code).await;
        assert!(
            result.is_ok(),
            "Concurrent access test failed: {:?}",
            result.err()
        );

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_delete_nonexistent_file_error() {
        let test_files = ["delete_nonexistent.txt"];
        cleanup_test_files(&test_files); // Ensure file doesn't exist

        // Try to delete a file that doesn't exist
        let code = r#"
            try:
                delete file at "delete_nonexistent.txt"
                display "Delete operation completed (may succeed even if file doesn't exist)"
            when error:
                display "Correctly caught delete nonexistent file error"
            end try
        "#;

        let result = execute_wfl_code_expect_success(code).await;
        assert!(
            result.is_ok(),
            "Delete nonexistent file test failed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_nested_error_handling() {
        let test_files = ["nested_error_test.txt"];
        cleanup_test_files(&test_files);

        // Test nested try/catch blocks with file operations
        let code = r#"
            try:
                try:
                    open file at "nested_error_test.txt" for writing as test_file
                    wait for write content "Outer try content" into test_file
                    close file test_file
                    
                    // Inner try that should fail
                    try:
                        open file at "nonexistent_dir/nested_file.txt" for reading as missing_file
                        wait for store file_data as read content from missing_file
                        close file missing_file
                        display "Inner try should not reach here"
                    when error:
                        display "Inner error caught successfully"
                    end try
                    
                    display "Outer try completed successfully"
                when error:
                    display "Outer error caught (this should not happen)"
                end try
            when error:
                display "Outermost error caught"
            end try
        "#;

        let result = execute_wfl_code_expect_success(code).await;
        assert!(
            result.is_ok(),
            "Nested error handling test failed: {:?}",
            result.err()
        );

        cleanup_test_files(&test_files);
    }
}
