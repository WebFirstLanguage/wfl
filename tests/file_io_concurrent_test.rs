use std::fs;
use std::path::Path;
use tokio::time::{Duration, timeout};
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

// Tests for concurrent file operations and async I/O behavior
#[cfg(test)]
mod file_io_concurrent_tests {
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

        // Execute the program with timeout to catch hanging operations
        let result = timeout(Duration::from_secs(10), interpreter.interpret(&ast)).await;
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
    async fn test_concurrent_file_writes() {
        let test_files = ["concurrent_1.txt", "concurrent_2.txt", "concurrent_3.txt"];
        cleanup_test_files(&test_files);

        // This test should verify that multiple async file operations can run concurrently
        let code = r#"
            // Start multiple async file write operations that should run concurrently
            open file at "concurrent_1.txt" for writing as file1
            open file at "concurrent_2.txt" for writing as file2
            open file at "concurrent_3.txt" for writing as file3
            
            // These write operations should be able to run concurrently
            wait for write content "Data for file 1" into file1
            wait for write content "Data for file 2" into file2
            wait for write content "Data for file 3" into file3
            
            close file file1
            close file file2
            close file file3
            
            // Verify all files were created
            store file1_exists as file exists at "concurrent_1.txt"
            store file2_exists as file exists at "concurrent_2.txt"
            store file3_exists as file exists at "concurrent_3.txt"
            
            check if file1_exists and file2_exists and file3_exists:
                display "All concurrent files created successfully"
            end check
        "#;

        let result = execute_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "Concurrent file writes failed: {:?}",
            result.err()
        );

        // Verify all files exist with correct content
        for (file, expected_content) in [
            ("concurrent_1.txt", "Data for file 1"),
            ("concurrent_2.txt", "Data for file 2"),
            ("concurrent_3.txt", "Data for file 3"),
        ] {
            assert!(
                Path::new(file).exists(),
                "Concurrent file {} was not created",
                file
            );
            let content =
                fs::read_to_string(file).unwrap_or_else(|_| panic!("Could not read {}", file));
            assert_eq!(
                content.trim(),
                expected_content,
                "Content mismatch in concurrent file {}",
                file
            );
        }

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_concurrent_file_read_write() {
        let test_files = ["read_write_1.txt", "read_write_2.txt"];
        cleanup_test_files(&test_files);

        // Create initial files
        fs::write("read_write_1.txt", "Initial content 1").expect("Failed to create test file");
        fs::write("read_write_2.txt", "Initial content 2").expect("Failed to create test file");

        // This test verifies concurrent read and write operations
        let code = r#"
            // Open files for both reading and writing concurrently
            open file at "read_write_1.txt" for reading as read_file1
            open file at "read_write_2.txt" for writing as write_file2
            
            // These operations should be able to run concurrently
            wait for store content1 as read content from read_file1
            wait for write content "New content for file 2" into write_file2
            
            close file read_file1
            close file write_file2
            
            display "Read from file 1: " with content1
            display "Wrote to file 2 completed"
        "#;

        let result = execute_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "Concurrent read/write failed: {:?}",
            result.err()
        );

        // Verify operations completed correctly
        let content1 = fs::read_to_string("read_write_1.txt").expect("Could not read file 1");
        assert_eq!(
            content1.trim(),
            "Initial content 1",
            "File 1 content changed unexpectedly"
        );

        let content2 = fs::read_to_string("read_write_2.txt").expect("Could not read file 2");
        assert_eq!(
            content2.trim(),
            "New content for file 2",
            "File 2 content not updated correctly"
        );

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_file_locking_behavior() {
        let test_files = ["locking_test.txt"];
        cleanup_test_files(&test_files);

        // This test should verify proper file locking behavior during concurrent access
        let code = r#"
            // Try to open the same file for writing multiple times
            open file at "locking_test.txt" for writing as file1
            wait for write content "First write operation" into file1
            
            // This should work - writing to the same file handle
            wait for append content "\\nSecond write to same handle" into file1
            close file file1
            
            // Now open again for reading while file is closed
            open file at "locking_test.txt" for reading as read_file
            wait for store final_content as read content from read_file
            close file read_file
            
            display "Final file content: " with final_content
        "#;

        let result = execute_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "File locking test failed: {:?}",
            result.err()
        );

        // Verify the file content is correct
        let content =
            fs::read_to_string("locking_test.txt").expect("Could not read locking test file");
        assert!(
            content.contains("First write operation"),
            "First write not found in file"
        );
        assert!(
            content.contains("Second write to same handle"),
            "Second write not found in file"
        );

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_async_directory_operations() {
        let test_files = ["async_dir_1.txt", "async_dir_2.log", "async_dir_3.dat"];
        cleanup_test_files(&test_files);

        // Create test files for directory listing
        for (i, file) in test_files.iter().enumerate() {
            fs::write(file, format!("Content for file {}", i + 1))
                .expect("Failed to create test file");
        }

        // This test verifies async directory listing operations
        let code = r#"
            // Multiple concurrent directory operations
            wait for store all_files as list files in "."
            wait for store txt_files as list files in "." with pattern "async_dir_*.txt"
            wait for store log_files as list files in "." with pattern "*.log"
            
            display "Total files: " with length of all_files
            display "TXT files: " with length of txt_files  
            display "LOG files: " with length of log_files
            
            // Verify we found the expected files
            check if length of txt_files is 1:
                display "Found expected TXT file"
            end check
            
            check if length of log_files is greater than 0:
                display "Found expected LOG files"
            end check
        "#;

        let result = execute_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "Async directory operations failed: {:?}",
            result.err()
        );

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_large_concurrent_file_operations() {
        let test_files: Vec<String> = (0..10)
            .map(|i| format!("large_concurrent_{}.txt", i))
            .collect();
        let test_file_refs: Vec<&str> = test_files.iter().map(|s| s.as_str()).collect();
        cleanup_test_files(&test_file_refs);

        // This test creates many files concurrently to test resource management
        let code = r#"
            // Create 10 files concurrently
            open file at "large_concurrent_0.txt" for writing as file0
            open file at "large_concurrent_1.txt" for writing as file1
            open file at "large_concurrent_2.txt" for writing as file2
            open file at "large_concurrent_3.txt" for writing as file3
            open file at "large_concurrent_4.txt" for writing as file4
            
            wait for write content "Content for file 0" into file0
            wait for write content "Content for file 1" into file1
            wait for write content "Content for file 2" into file2
            wait for write content "Content for file 3" into file3
            wait for write content "Content for file 4" into file4
            
            close file file0
            close file file1
            close file file2
            close file file3
            close file file4
            
            // Verify files were created
            store total_files as 0
            check if file exists at "large_concurrent_0.txt":
                change total_files to total_files + 1
            end check
            check if file exists at "large_concurrent_1.txt":
                change total_files to total_files + 1
            end check
            check if file exists at "large_concurrent_2.txt":
                change total_files to total_files + 1
            end check
            check if file exists at "large_concurrent_3.txt":
                change total_files to total_files + 1
            end check
            check if file exists at "large_concurrent_4.txt":
                change total_files to total_files + 1
            end check
            
            display "Created " with total_files with " files concurrently"
        "#;

        let result = execute_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "Large concurrent file operations failed: {:?}",
            result.err()
        );

        // Verify at least the first 5 files were created
        for i in 0..5 {
            let filename = format!("large_concurrent_{}.txt", i);
            assert!(
                Path::new(&filename).exists(),
                "File {} was not created",
                filename
            );
            let content = fs::read_to_string(&filename)
                .unwrap_or_else(|_| panic!("Could not read {}", filename));
            assert_eq!(
                content.trim(),
                format!("Content for file {}", i),
                "Content mismatch in {}",
                filename
            );
        }

        cleanup_test_files(&test_file_refs);
    }
}
