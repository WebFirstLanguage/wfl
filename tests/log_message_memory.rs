#[cfg(feature = "dhat-heap")]
mod tests {
    use std::fs;
    use std::rc::Rc;
    use wfl::interpreter::Interpreter;
    use wfl::lexer::lex_wfl_with_positions;
    use wfl::parser::Parser;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_log_message_memory_usage() {
        // Initialize the heap profiler for this test
        let _profiler = dhat::Profiler::builder().testing().build();

        let log_path = "temp_nexus.log";
        let _ = fs::remove_file(log_path); // Remove if exists

        // Create a minimal test case that creates a function with weak environment reference
        // This directly tests the fix for the closure reference cycle
        let source = r#"
        define action called log_message needs message_text:
            // Simple action definition to test memory leak fix
            display message_text
        end action
        
        // Call the action to make sure it works
        log_message with "This is a test message"
        "#;

        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);
        let program = parser.parse().unwrap();

        // Short timeout to ensure test completes quickly
        let mut interpreter = Interpreter::with_timeout(2);
        let result = interpreter.interpret(&program).await;
        assert!(result.is_ok(), "Error executing script: {result:?}");

        // Get memory stats after execution
        let stats = dhat::HeapStats::get();
        println!("Max memory usage: {} bytes", stats.max_bytes);
        println!("Total allocations: {}", stats.total_blocks);

        // Check that memory usage is reasonable
        // With the fixed Weak<RefCell<Environment>> references, memory should be much lower
        assert!(
            stats.max_bytes < 1024 * 1024, // Much lower limit than previous memory usage
            "Max memory usage was too high: {} bytes >= 1 MB",
            stats.max_bytes
        );

        // Also check the total number of allocations is reasonable
        assert!(
            stats.total_blocks < 20000,
            "Total allocations too high: {} >= 20000",
            stats.total_blocks
        );

        // Most importantly, verify we fixed the reference cycle
        // Check that the global environment has only one reference after execution
        // This means no lingering reference cycles involving function closures
        let global_env = interpreter.global_env();
        let rc_count = Rc::strong_count(global_env);
        assert_eq!(
            rc_count, 1,
            "Global environment should have exactly one reference, but had {rc_count}"
        );

        drop(interpreter); // Explicitly drop to ensure cleanup
    }
}
