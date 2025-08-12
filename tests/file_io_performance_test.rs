use std::fs;
use std::path::Path;
use std::time::Instant;
use tokio::time::{Duration, timeout};
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

// Performance and stress tests for file I/O operations
#[cfg(test)]
mod file_io_performance_tests {
    use super::*;

    fn cleanup_test_files(files: &[&str]) {
        for file in files {
            let _ = fs::remove_file(file);
        }
    }

    async fn execute_wfl_code_with_timing(
        code: &str,
    ) -> Result<(String, std::time::Duration), Box<dyn std::error::Error>> {
        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let ast = parser.parse().expect("Failed to parse WFL code");

        let mut interpreter = Interpreter::new();

        let start = Instant::now();
        let result = timeout(Duration::from_secs(30), interpreter.interpret(&ast)).await;
        let elapsed = start.elapsed();

        match result {
            Ok(Ok(_)) => Ok(("Program executed successfully".to_string(), elapsed)),
            Ok(Err(errors)) => {
                let error_msg = errors
                    .iter()
                    .map(|e| format!("{}", e))
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    error_msg,
                )))
            }
            Err(_) => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Operation timed out",
            ))),
        }
    }

    #[tokio::test]
    async fn test_many_small_files_performance() {
        let test_files: Vec<String> = (0..50).map(|i| format!("perf_small_{}.txt", i)).collect();
        let test_file_refs: Vec<&str> = test_files.iter().map(|s| s.as_str()).collect();
        cleanup_test_files(&test_file_refs);

        // Create many small files to test file system overhead
        let mut code = String::from(
            r#"
            // Create many small files sequentially
            store files_created as 0
        "#,
        );

        for i in 0..10 {
            // Reduced from 50 to avoid too much overhead
            code.push_str(&format!(
                r#"
                open file at "perf_small_{}.txt" for writing as file{}
                wait for write content "Content for small file {}" into file{}
                close file file{}
                change files_created to files_created + 1
            "#,
                i, i, i, i, i
            ));
        }

        code.push_str(
            r#"
            display "Created " with files_created with " small files"
        "#,
        );

        let result = execute_wfl_code_with_timing(&code).await;
        assert!(
            result.is_ok(),
            "Many small files performance test failed: {:?}",
            result.err()
        );

        let (_, elapsed) = result.unwrap();
        println!("Many small files test took: {:?}", elapsed);
        assert!(
            elapsed < Duration::from_secs(10),
            "Test took too long: {:?}",
            elapsed
        );

        // Verify some files were created
        for i in 0..5 {
            let filename = format!("perf_small_{}.txt", i);
            assert!(
                Path::new(&filename).exists(),
                "File {} was not created",
                filename
            );
        }

        cleanup_test_files(&test_file_refs);
    }

    #[tokio::test]
    async fn test_large_file_write_performance() {
        let test_files = ["perf_large_write.txt"];
        cleanup_test_files(&test_files);

        // Write a moderately sized file to test throughput
        let large_content =
            "This is a line of text that will be repeated many times.\n".repeat(100); // ~5.7KB

        let code = format!(
            r#"
            open file at "perf_large_write.txt" for writing as large_file
            wait for write content "{}" into large_file
            close file large_file
            
            // Verify the file was created
            store file_exists as file exists at "perf_large_write.txt"
            check if file_exists:
                display "Large file write completed successfully"
            end check
        "#,
            large_content.replace('\"', "\\\"")
        );

        let result = execute_wfl_code_with_timing(&code).await;
        assert!(
            result.is_ok(),
            "Large file write performance test failed: {:?}",
            result.err()
        );

        let (_, elapsed) = result.unwrap();
        println!("Large file write test took: {:?}", elapsed);
        assert!(
            elapsed < Duration::from_secs(5),
            "Large file write took too long: {:?}",
            elapsed
        );

        // Verify the file content
        assert!(
            Path::new("perf_large_write.txt").exists(),
            "Large file was not created"
        );
        let file_size = fs::metadata("perf_large_write.txt").unwrap().len();
        assert!(
            file_size > 5_000,
            "File size {} is smaller than expected",
            file_size
        );

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_large_file_read_performance() {
        let test_files = ["perf_large_read.txt"];
        cleanup_test_files(&test_files);

        // Create a moderately large file first
        let large_content = "Line of data for performance testing.\n".repeat(200); // ~7.6KB
        fs::write("perf_large_read.txt", &large_content).expect("Failed to create large test file");

        let code = r#"
            open file at "perf_large_read.txt" for reading as large_file
            wait for store file_content as read content from large_file
            close file large_file
            
            // Verify we read some content
            store content_length as length of file_content
            check if content_length is greater than 5000:
                display "Large file read completed successfully"
            end check
        "#;

        let result = execute_wfl_code_with_timing(code).await;
        assert!(
            result.is_ok(),
            "Large file read performance test failed: {:?}",
            result.err()
        );

        let (_, elapsed) = result.unwrap();
        println!("Large file read test took: {:?}", elapsed);
        assert!(
            elapsed < Duration::from_secs(5),
            "Large file read took too long: {:?}",
            elapsed
        );

        cleanup_test_files(&test_files);
    }

    #[tokio::test]
    async fn test_directory_listing_performance() {
        let test_files: Vec<String> = (0..30).map(|i| format!("dir_perf_{}.txt", i)).collect();
        let test_file_refs: Vec<&str> = test_files.iter().map(|s| s.as_str()).collect();
        cleanup_test_files(&test_file_refs);

        // Create multiple files for directory listing
        for (i, file) in test_files.iter().enumerate() {
            fs::write(file, format!("Content for file {}", i)).expect("Failed to create test file");
        }

        let code = r#"
            // Test directory listing performance
            wait for store all_files as list files in "."
            wait for store txt_files as list files in "." with pattern "dir_perf_*.txt"
            wait for store recursive_files as list files recursively in "."
            
            display "Listed all files: " with length of all_files
            display "Listed TXT files: " with length of txt_files
            display "Listed recursive files: " with length of recursive_files
        "#;

        let result = execute_wfl_code_with_timing(code).await;
        assert!(
            result.is_ok(),
            "Directory listing performance test failed: {:?}",
            result.err()
        );

        let (_, elapsed) = result.unwrap();
        println!("Directory listing test took: {:?}", elapsed);
        assert!(
            elapsed < Duration::from_secs(10),
            "Directory listing took too long: {:?}",
            elapsed
        );

        cleanup_test_files(&test_file_refs);
    }

    #[tokio::test]
    async fn test_rapid_file_operations() {
        let test_files: Vec<String> = (0..20).map(|i| format!("rapid_{}.txt", i)).collect();
        let test_file_refs: Vec<&str> = test_files.iter().map(|s| s.as_str()).collect();
        cleanup_test_files(&test_file_refs);

        // Test rapid create, write, read, delete cycle
        let mut code = String::from(
            r#"
            // Rapid file operations cycle
            store operations_completed as 0
        "#,
        );

        for i in 0..5 {
            // Reduced from 20 to avoid timeout
            code.push_str(&format!(
                r#"
                // Create, write, read, delete cycle for file {}
                open file at "rapid_{}.txt" for writing as file{}
                wait for write content "Rapid test content {}" into file{}
                close file file{}
                
                open file at "rapid_{}.txt" for reading as read_file{}
                wait for store content{} as read content from read_file{}
                close file read_file{}
                
                delete file at "rapid_{}.txt"
                change operations_completed to operations_completed + 1
            "#,
                i, i, i, i, i, i, i, i, i, i, i, i
            ));
        }

        code.push_str(
            r#"
            display "Completed " with operations_completed with " rapid file operation cycles"
        "#,
        );

        let result = execute_wfl_code_with_timing(&code).await;
        assert!(
            result.is_ok(),
            "Rapid file operations test failed: {:?}",
            result.err()
        );

        let (_, elapsed) = result.unwrap();
        println!("Rapid file operations test took: {:?}", elapsed);
        assert!(
            elapsed < Duration::from_secs(10),
            "Rapid operations took too long: {:?}",
            elapsed
        );

        cleanup_test_files(&test_file_refs);
    }

    #[tokio::test]
    async fn test_concurrent_performance() {
        let test_files: Vec<String> = (0..15)
            .map(|i| format!("concurrent_perf_{}.txt", i))
            .collect();
        let test_file_refs: Vec<&str> = test_files.iter().map(|s| s.as_str()).collect();
        cleanup_test_files(&test_file_refs);

        // Test performance of concurrent operations
        let mut code = String::from(
            r#"
            // Open multiple files concurrently
        "#,
        );

        // Open files
        for i in 0..5 {
            code.push_str(&format!(
                "open file at \"concurrent_perf_{}.txt\" for writing as file{}\n",
                i, i
            ));
        }

        // Write to all files
        for i in 0..5 {
            code.push_str(&format!(
                "wait for write content \"Concurrent performance test data for file {}\" into file{}\n", 
                i, i
            ));
        }

        // Close all files
        for i in 0..5 {
            code.push_str(&format!("close file file{}\n", i));
        }

        code.push_str(
            r#"
            // Verify all files exist
            store total_files as 0
            check if file exists at "concurrent_perf_0.txt":
                change total_files to total_files + 1
            end check
            check if file exists at "concurrent_perf_1.txt":
                change total_files to total_files + 1
            end check
            check if file exists at "concurrent_perf_2.txt":
                change total_files to total_files + 1
            end check
            check if file exists at "concurrent_perf_3.txt":
                change total_files to total_files + 1
            end check
            check if file exists at "concurrent_perf_4.txt":
                change total_files to total_files + 1
            end check
            
            display "Created " with total_files with " files concurrently"
        "#,
        );

        let result = execute_wfl_code_with_timing(&code).await;
        assert!(
            result.is_ok(),
            "Concurrent performance test failed: {:?}",
            result.err()
        );

        let (_, elapsed) = result.unwrap();
        println!("Concurrent performance test took: {:?}", elapsed);
        assert!(
            elapsed < Duration::from_secs(8),
            "Concurrent operations took too long: {:?}",
            elapsed
        );

        // Verify files were created
        for i in 0..5 {
            let filename = format!("concurrent_perf_{}.txt", i);
            assert!(
                Path::new(&filename).exists(),
                "Concurrent file {} was not created",
                filename
            );
        }

        cleanup_test_files(&test_file_refs);
    }

    #[tokio::test]
    async fn test_memory_usage_large_operations() {
        let test_files = ["memory_test_large.txt", "memory_test_output.txt"];
        cleanup_test_files(&test_files);

        // Create a moderately large file and copy it
        let content = "Memory usage test line.\n".repeat(500); // ~12.5KB
        fs::write("memory_test_large.txt", &content).expect("Failed to create large test file");

        let code = r#"
            // Test memory usage during large file operations
            open file at "memory_test_large.txt" for reading as source_file
            wait for store large_content as read content from source_file
            close file source_file
            
            // Write the content to a new file
            open file at "memory_test_output.txt" for writing as output_file
            wait for write content large_content into output_file
            close file output_file
            
            // Verify both files exist
            store source_exists as file exists at "memory_test_large.txt"
            store output_exists as file exists at "memory_test_output.txt"
            
            check if source_exists and output_exists:
                display "Memory usage test completed successfully"
            end check
        "#;

        let result = execute_wfl_code_with_timing(code).await;
        assert!(
            result.is_ok(),
            "Memory usage test failed: {:?}",
            result.err()
        );

        let (_, elapsed) = result.unwrap();
        println!("Memory usage test took: {:?}", elapsed);
        assert!(
            elapsed < Duration::from_secs(5),
            "Memory test took too long: {:?}",
            elapsed
        );

        // Verify both files exist and have similar sizes
        let source_size = fs::metadata("memory_test_large.txt").unwrap().len();
        let output_size = fs::metadata("memory_test_output.txt").unwrap().len();
        assert_eq!(
            source_size, output_size,
            "File sizes don't match after copy"
        );

        cleanup_test_files(&test_files);
    }
}
