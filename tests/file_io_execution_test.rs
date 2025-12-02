use std::fs;
use std::path::Path;
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

// Integration tests that actually execute file I/O operations using the interpreter
#[cfg(test)]
mod file_io_execution_tests {
    use super::*;

    fn cleanup_test_files(files: &[&str]) {
        for file in files {
            let _ = fs::remove_file(file);
        }
    }

    async fn execute_wfl_code(code: &str) -> Result<String, Box<dyn std::error::Error>> {
        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let ast = parser.parse().expect("Failed to parse WFL code");

        let mut interpreter = Interpreter::new();

        // Execute the program
        let result = interpreter.interpret(&ast).await;
        match result {
            Ok(_) => Ok("Program executed successfully".to_string()),
            Err(errors) => {
                let error_msg = errors
                    .iter()
                    .map(|e| format!("{}", e))
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(Box::new(std::io::Error::other(error_msg)))
            }
        }
    }

    #[tokio::test]
    async fn test_basic_file_write_read_execution() {
        let test_files = ["test_exec_basic.txt"];
        cleanup_test_files(&test_files);

        let code = r#"
            open file at "test_exec_basic.txt" for writing as test_file
            wait for write content "Hello from execution test!" into test_file
            close file test_file
            
            open file at "test_exec_basic.txt" for reading as read_file
            wait for store file_data as read content from read_file
            close file read_file
            
            display file_data
        "#;

        // This test should fail initially because we need to verify the interpreter
        // actually creates files and reads content correctly
        let result = execute_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "File I/O execution failed: {:?}",
            result.err()
        );

        // Verify the file was actually created
        assert!(
            Path::new("test_exec_basic.txt").exists(),
            "Test file was not created by interpreter"
        );

        // Verify file contents
        let file_contents =
            fs::read_to_string("test_exec_basic.txt").expect("Could not read test file");
        assert_eq!(
            file_contents.trim(),
            "Hello from execution test!",
            "File contents don't match expected value"
        );

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_file_append_execution() {
        let test_files = ["test_exec_append.txt"];
        cleanup_test_files(&test_files);

        let code = r#"
            open file at "test_exec_append.txt" for writing as initial_file
            wait for write content "Line 1" into initial_file
            close file initial_file
            
            open file at "test_exec_append.txt" for append as append_file
            wait for append content "\\nLine 2" into append_file
            close file append_file
            
            open file at "test_exec_append.txt" for reading as read_file
            wait for store final_content as read content from read_file
            close file read_file
            
            display final_content
        "#;

        let result = execute_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "File append execution failed: {:?}",
            result.err()
        );

        let file_contents =
            fs::read_to_string("test_exec_append.txt").expect("Could not read append test file");
        assert_eq!(
            file_contents.trim(),
            "Line 1\\nLine 2",
            "Appended file contents don't match expected value"
        );

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_file_exists_execution() {
        let test_files = ["test_exec_exists.txt"];
        cleanup_test_files(&test_files);

        // Create a test file first
        fs::write("test_exec_exists.txt", "test content").expect("Failed to create test file");

        let code = r#"
            store exists_result as file exists at "test_exec_exists.txt"
            check if exists_result:
                display "File exists check passed"
            end check
            
            store missing_result as file exists at "nonexistent_file.txt"
            check if not missing_result:
                display "Missing file check passed"
            end check
        "#;

        let result = execute_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "File exists execution failed: {:?}",
            result.err()
        );

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_directory_listing_execution() {
        let test_files = ["test_dir_1.txt", "test_dir_2.log", "test_dir_3.txt"];
        cleanup_test_files(&test_files);

        // Create test files
        for file in &test_files {
            fs::write(file, "test content").expect("Failed to create test file");
        }

        let code = r#"
            wait for store all_files as list files in "."
            wait for store txt_files as list files in "." with pattern "*.txt"
            
            display "Total files found: " with length of all_files
            display "TXT files found: " with length of txt_files
        "#;

        let result = execute_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "Directory listing execution failed: {:?}",
            result.err()
        );

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_file_deletion_execution() {
        let test_files = ["test_delete_me.txt"];

        // Create test file first
        fs::write("test_delete_me.txt", "This file should be deleted")
            .expect("Failed to create test file");
        assert!(
            Path::new("test_delete_me.txt").exists(),
            "Test file was not created for deletion test"
        );

        let code = r#"
            delete file at "test_delete_me.txt"
            store still_exists as file exists at "test_delete_me.txt"
            check if not still_exists:
                display "File successfully deleted"
            end check
        "#;

        let result = execute_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "File deletion execution failed: {:?}",
            result.err()
        );

        // Verify file was actually deleted
        assert!(
            !Path::new("test_delete_me.txt").exists(),
            "Test file was not properly deleted by interpreter"
        );

        cleanup_test_files(&test_files); // Just in case
    }

    #[tokio::test]
    async fn test_multiple_files_execution() {
        let test_files = ["multi_test_1.txt", "multi_test_2.log", "multi_test_3.dat"];
        cleanup_test_files(&test_files);

        let code = r#"
            // Create multiple files with different content
            open file at "multi_test_1.txt" for writing as file1
            wait for write content "Content for file 1" into file1
            close file file1
            
            open file at "multi_test_2.log" for writing as file2
            wait for write content "Log data for file 2" into file2
            close file file2
            
            open file at "multi_test_3.dat" for writing as file3
            wait for write content "Binary-like data for file 3" into file3
            close file file3
            
            // Verify all files were created
            store file1_exists as file exists at "multi_test_1.txt"
            store file2_exists as file exists at "multi_test_2.log"
            store file3_exists as file exists at "multi_test_3.dat"
            
            check if file1_exists and file2_exists and file3_exists:
                display "All multiple files created successfully"
            end check
        "#;

        let result = execute_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "Multiple files execution failed: {:?}",
            result.err()
        );

        // Verify all files exist with correct content
        for (file, expected_content) in [
            ("multi_test_1.txt", "Content for file 1"),
            ("multi_test_2.log", "Log data for file 2"),
            ("multi_test_3.dat", "Binary-like data for file 3"),
        ] {
            assert!(Path::new(file).exists(), "File {} was not created", file);
            let content =
                fs::read_to_string(file).unwrap_or_else(|_| panic!("Could not read {}", file));
            assert_eq!(
                content.trim(),
                expected_content,
                "Content mismatch in {}",
                file
            );
        }

        cleanup_test_files(&test_files);
    }
}
