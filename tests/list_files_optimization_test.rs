use std::fs;
use std::time::Instant;
use tokio::time::Duration;
use tokio::time::timeout;
use wfl::interpreter::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

// Performance test for file listing optimizations
#[cfg(test)]
mod list_files_optimization_test {
    use super::*;

    fn setup_test_files(dir_name: &str) {
        let _ = fs::remove_dir_all(dir_name);
        fs::create_dir_all(dir_name).expect("Failed to create test directory");

        // Create 2000 files:
        // 1000 matching files (.txt)
        // 1000 non-matching files (.dat)
        for i in 0..1000 {
            fs::write(format!("{}/match_{}.txt", dir_name, i), "content").unwrap();
            fs::write(format!("{}/nomatch_{}.dat", dir_name, i), "content").unwrap();
        }
    }

    fn cleanup_test_files(dir_name: &str) {
        let _ = fs::remove_dir_all(dir_name);
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
                Err(Box::new(std::io::Error::other(error_msg)))
            }
            Err(_) => Err(Box::new(std::io::Error::other("Operation timed out"))),
        }
    }

    #[tokio::test]
    async fn test_list_files_filtered_performance() {
        let test_dir = "perf_list_files_filtered";
        setup_test_files(test_dir);

        let code = format!(
            r#"
            // List files filtered
            wait for store txt_files as list files in "{}" with pattern "*.txt"
            display "Found " with length of txt_files with " matching files"
        "#,
            test_dir
        );

        // Run multiple times to average out noise (and warm up if applicable, though interpreter is fresh)
        let mut total_duration = Duration::new(0, 0);
        const ITERATIONS: u32 = 5;

        for _ in 0..ITERATIONS {
            let result = execute_wfl_code_with_timing(&code).await;
            assert!(
                result.is_ok(),
                "List files filtered performance test failed: {:?}",
                result.err()
            );
            let (_, elapsed) = result.unwrap();
            total_duration += elapsed;
        }

        let avg_duration = total_duration / ITERATIONS;
        println!("List files filtered avg time: {:?}", avg_duration);

        cleanup_test_files(test_dir);
    }

    #[tokio::test]
    async fn test_list_files_recursive_performance() {
        let test_dir = "perf_list_files_recursive";
        setup_test_files(test_dir);

        let code = format!(
            r#"
            // List files recursive with filter
            wait for store txt_files as list files recursively in "{}" with extension ".txt"
            display "Found " with length of txt_files with " matching files"
        "#,
            test_dir
        );

        let mut total_duration = Duration::new(0, 0);
        const ITERATIONS: u32 = 5;

        for _ in 0..ITERATIONS {
            let result = execute_wfl_code_with_timing(&code).await;
            assert!(
                result.is_ok(),
                "List files recursive performance test failed: {:?}",
                result.err()
            );
            let (_, elapsed) = result.unwrap();
            total_duration += elapsed;
        }

        let avg_duration = total_duration / ITERATIONS;
        println!("List files recursive avg time: {:?}", avg_duration);

        cleanup_test_files(test_dir);
    }
}
