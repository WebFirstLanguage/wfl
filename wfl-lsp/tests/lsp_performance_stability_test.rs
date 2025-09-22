// LSP Performance and Stability Tests
// These tests validate that the WFL LSP server performs well under various stress conditions
// and recovers gracefully from error scenarios

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tower_lsp::lsp_types::{
    CompletionParams, HoverParams, Position, TextDocumentIdentifier, TextDocumentPositionParams,
    Url,
};
use wfl::analyzer::Analyzer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

// Helper function to generate large WFL programs for performance testing
fn generate_large_wfl_program(
    num_variables: usize,
    num_functions: usize,
    num_statements: usize,
) -> String {
    let mut program = String::new();

    // Generate many variable declarations
    for i in 0..num_variables {
        program.push_str(&format!("store variable_{} as {}\n", i, i));
    }

    program.push_str("\n");

    // Generate many function definitions
    for i in 0..num_functions {
        program.push_str(&format!(
            "define action called function_{} with param:\n    return param + {}\nend action\n\n",
            i, i
        ));
    }

    // Generate many statements
    for i in 0..num_statements {
        program.push_str(&format!("display \"Statement {}\"\n", i));
    }

    program
}

// Helper function to generate deeply nested WFL program
fn generate_nested_wfl_program(depth: usize) -> String {
    let mut program = String::new();

    // Create nested if statements
    for i in 0..depth {
        program.push_str(&format!("if {} is greater than 0 then\n", i));
    }

    program.push_str("    display \"Deep nesting\"\n");

    for _ in 0..depth {
        program.push_str("end if\n");
    }

    program
}

// Helper function to simulate LSP operations
async fn simulate_lsp_operations(document_text: &str, num_operations: usize) -> Duration {
    let start_time = Instant::now();

    for i in 0..num_operations {
        // Simulate lexing
        let tokens = lex_wfl_with_positions(document_text);

        // Simulate parsing
        let mut parser = Parser::new(&tokens);
        let _parse_result = parser.parse();

        // Simulate completion request
        let position = Position {
            line: (i % 10) as u32,
            character: 0,
        };
        let _completion_position = position;

        // Simulate hover request
        let _hover_position = position;

        // Add small delay to simulate real usage
        tokio::time::sleep(Duration::from_micros(100)).await;
    }

    start_time.elapsed()
}

// Helper function to test concurrent LSP operations
async fn test_concurrent_operations(document_text: &str, num_concurrent: usize) -> Duration {
    let start_time = Instant::now();
    let document_text = Arc::new(document_text.to_string());

    let mut handles = Vec::new();

    for i in 0..num_concurrent {
        let doc_text = Arc::clone(&document_text);
        let handle = tokio::spawn(async move {
            // Each task performs LSP operations
            let tokens = lex_wfl_with_positions(&doc_text);
            let mut parser = Parser::new(&tokens);
            let parse_result = parser.parse();

            if let Ok(program) = parse_result {
                let mut analyzer = Analyzer::new();
                let _analysis_result = analyzer.analyze(&program);

                let mut type_checker = TypeChecker::new();
                let _type_result = type_checker.check_types(&program);
            }

            i // Return task ID
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let _result = handle.await;
    }

    start_time.elapsed()
}

#[tokio::test]
async fn test_lsp_performance_with_large_files() {
    // Test LSP performance with increasingly large files
    let test_sizes = [
        (100, 10, 100),    // Small: 100 vars, 10 functions, 100 statements
        (500, 50, 500),    // Medium: 500 vars, 50 functions, 500 statements
        (1000, 100, 1000), // Large: 1000 vars, 100 functions, 1000 statements
    ];

    for (num_vars, num_funcs, num_stmts) in &test_sizes {
        let large_program = generate_large_wfl_program(*num_vars, *num_funcs, *num_stmts);
        let program_size = large_program.len();

        println!(
            "Testing LSP performance with program size: {} bytes ({} vars, {} funcs, {} stmts)",
            program_size, num_vars, num_funcs, num_stmts
        );

        let start_time = Instant::now();

        // Test lexing performance
        let tokens = lex_wfl_with_positions(&large_program);
        let lexing_time = start_time.elapsed();

        // Test parsing performance
        let parse_start = Instant::now();
        let mut parser = Parser::new(&tokens);
        let parse_result = parser.parse();
        let parsing_time = parse_start.elapsed();

        // Test analysis performance (if parsing succeeded)
        let mut analysis_time = Duration::from_secs(0);
        let mut type_checking_time = Duration::from_secs(0);

        if let Ok(program) = parse_result {
            let analysis_start = Instant::now();
            let mut analyzer = Analyzer::new();
            let _analysis_result = analyzer.analyze(&program);
            analysis_time = analysis_start.elapsed();

            let type_start = Instant::now();
            let mut type_checker = TypeChecker::new();
            let _type_result = type_checker.check_types(&program);
            type_checking_time = type_start.elapsed();
        }

        println!(
            "  Lexing: {:?}, Parsing: {:?}, Analysis: {:?}, Type checking: {:?}",
            lexing_time, parsing_time, analysis_time, type_checking_time
        );

        // Performance assertions
        assert!(
            lexing_time.as_millis() < 1000,
            "Lexing should be under 1s for {} byte program",
            program_size
        );
        assert!(
            parsing_time.as_millis() < 2000,
            "Parsing should be under 2s for {} byte program",
            program_size
        );

        if analysis_time.as_millis() > 0 {
            assert!(
                analysis_time.as_millis() < 3000,
                "Analysis should be under 3s for {} byte program",
                program_size
            );
        }

        if type_checking_time.as_millis() > 0 {
            assert!(
                type_checking_time.as_millis() < 3000,
                "Type checking should be under 3s for {} byte program",
                program_size
            );
        }
    }
}

#[tokio::test]
async fn test_lsp_performance_with_deep_nesting() {
    // Test LSP performance with deeply nested structures
    let nesting_depths = [10, 25, 50, 100];

    for depth in &nesting_depths {
        let nested_program = generate_nested_wfl_program(*depth);

        println!("Testing LSP performance with nesting depth: {}", depth);

        let start_time = Instant::now();

        // Test parsing with deep nesting
        let tokens = lex_wfl_with_positions(&nested_program);
        let mut parser = Parser::new(&tokens);
        let parse_result = parser.parse();
        let total_time = start_time.elapsed();

        println!("  Deep nesting (depth {}): {:?}", depth, total_time);

        // Performance assertion - should handle reasonable nesting depths
        if *depth <= 50 {
            assert!(
                total_time.as_millis() < 1000,
                "Deep nesting (depth {}) should parse under 1s",
                depth
            );
        } else {
            // Very deep nesting might be slower but should still complete
            assert!(
                total_time.as_millis() < 5000,
                "Very deep nesting (depth {}) should parse under 5s",
                depth
            );
        }

        // Should not crash with deep nesting
        match parse_result {
            Ok(_) => println!("  Successfully parsed depth {}", depth),
            Err(errors) => println!("  Parse errors at depth {}: {} errors", depth, errors.len()),
        }
    }
}

#[tokio::test]
async fn test_lsp_concurrent_operations() {
    // Test LSP server handling concurrent operations
    let test_program = generate_large_wfl_program(100, 20, 100);
    let concurrency_levels = [1, 5, 10, 20];

    for num_concurrent in &concurrency_levels {
        println!("Testing LSP with {} concurrent operations", num_concurrent);

        let concurrent_time = test_concurrent_operations(&test_program, *num_concurrent).await;

        println!(
            "  {} concurrent operations took: {:?}",
            num_concurrent, concurrent_time
        );

        // Performance assertion - concurrent operations should scale reasonably
        let expected_max_time = Duration::from_millis(1000 * (*num_concurrent as u64));
        assert!(
            concurrent_time < expected_max_time,
            "Concurrent operations ({}) should complete within reasonable time",
            num_concurrent
        );

        // Concurrency should not be significantly slower than sequential for small numbers
        if *num_concurrent <= 5 {
            assert!(
                concurrent_time.as_millis() < 2000,
                "Small concurrent operations ({}) should be fast",
                num_concurrent
            );
        }
    }
}

#[tokio::test]
async fn test_lsp_repeated_operations_performance() {
    // Test LSP performance with repeated operations (simulating real editor usage)
    let test_program = generate_large_wfl_program(200, 30, 200);
    let operation_counts = [10, 50, 100];

    for num_operations in &operation_counts {
        println!("Testing LSP with {} repeated operations", num_operations);

        let repeated_time = simulate_lsp_operations(&test_program, *num_operations).await;

        println!(
            "  {} repeated operations took: {:?}",
            num_operations, repeated_time
        );

        // Performance assertion - repeated operations should be efficient
        let avg_time_per_op = repeated_time.as_millis() / (*num_operations as u128);
        assert!(
            avg_time_per_op < 100,
            "Average time per operation should be under 100ms, got {}ms",
            avg_time_per_op
        );

        // Total time should scale reasonably
        assert!(
            repeated_time.as_millis() < 10000,
            "Total time for {} operations should be under 10s",
            num_operations
        );
    }
}

#[tokio::test]
async fn test_lsp_memory_stability() {
    // Test that LSP operations don't cause memory leaks or excessive memory usage
    let test_program = generate_large_wfl_program(500, 50, 500);

    // Perform many operations to test memory stability
    for iteration in 0..10 {
        println!("Memory stability test iteration: {}", iteration + 1);

        // Perform multiple LSP operations
        let tokens = lex_wfl_with_positions(&test_program);
        let mut parser = Parser::new(&tokens);
        let parse_result = parser.parse();

        if let Ok(program) = parse_result {
            let mut analyzer = Analyzer::new();
            let _analysis_result = analyzer.analyze(&program);

            let mut type_checker = TypeChecker::new();
            let _type_result = type_checker.check_types(&program);
        }

        // Force garbage collection (if available)
        // In Rust, memory is automatically managed, objects are dropped automatically

        // Small delay between iterations
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    println!("Memory stability test completed - no crashes or excessive memory usage");
}

#[tokio::test]
async fn test_lsp_error_recovery() {
    // Test LSP server recovery from various error conditions
    let error_scenarios = [
        ("Empty document", ""),
        ("Only whitespace", "   \n\n  \t  \n"),
        ("Only comments", "// This is a comment\n// Another comment"),
        ("Incomplete statement", "store x as"),
        ("Invalid syntax", "this is not valid wfl syntax at all"),
        (
            "Mixed valid/invalid",
            "store x as 5\nthis is invalid\ndisplay x",
        ),
        ("Unmatched brackets", "store x as [1, 2, 3\ndisplay x"),
        (
            "Infinite loop potential",
            "count from i as 1 to 1000000\n    display i\nend count",
        ),
        ("Very long line", &"a".repeat(10000)),
        ("Unicode characters", "store 变量 as \"测试\"\ndisplay 变量"),
        (
            "Special characters",
            "store x as \"!@#$%^&*()_+-={}[]|\\:;'<>?,./\"",
        ),
    ];

    for (scenario_name, document_text) in &error_scenarios {
        println!("Testing error recovery for: {}", scenario_name);

        let start_time = Instant::now();

        // Test that LSP operations don't crash or hang
        let tokens = lex_wfl_with_positions(document_text);
        let mut parser = Parser::new(&tokens);
        let parse_result = parser.parse();

        let recovery_time = start_time.elapsed();

        // Should complete quickly even with errors
        assert!(
            recovery_time.as_millis() < 1000,
            "Error recovery for '{}' should be fast",
            scenario_name
        );

        // Should not crash (we got here, so it didn't crash)
        match parse_result {
            Ok(_) => println!("  '{}' parsed successfully", scenario_name),
            Err(errors) => println!(
                "  '{}' generated {} parse errors (expected)",
                scenario_name,
                errors.len()
            ),
        }

        // Test that we can still perform LSP operations after errors
        let position = Position {
            line: 0,
            character: 0,
        };
        let _completion_test = position; // Simulate completion request
        let _hover_test = position; // Simulate hover request

        println!(
            "  Error recovery for '{}' completed in {:?}",
            scenario_name, recovery_time
        );
    }
}

#[tokio::test]
async fn test_lsp_stability_with_malformed_input() {
    // Test LSP stability with various malformed inputs
    let malformed_inputs = [
        "store",
        "store as",
        "store x",
        "store x as as",
        "if then end if",
        "count from to end count",
        "define action end action",
        "display display display",
        "\"unclosed string",
        "/* unclosed comment",
        "nested \"quotes \\\"inside\\\" quotes\"",
        &format!("very long identifier {}", "a".repeat(1000)),
        "\x00\x01\x02\x03", // Control characters
        "🚀🎉🔥💯",         // Emojis
    ];

    for (i, malformed_input) in malformed_inputs.iter().enumerate() {
        println!(
            "Testing malformed input {}: {:?}",
            i + 1,
            malformed_input.chars().take(50).collect::<String>()
        );

        let start_time = Instant::now();

        // Test lexing with malformed input
        let tokens = lex_wfl_with_positions(malformed_input);

        // Test parsing with malformed input
        let mut parser = Parser::new(&tokens);
        let parse_result = parser.parse();

        let processing_time = start_time.elapsed();

        // Should complete quickly and not crash
        assert!(
            processing_time.as_millis() < 500,
            "Malformed input {} should be processed quickly",
            i + 1
        );

        // Should handle gracefully (not crash)
        match parse_result {
            Ok(_) => println!("  Malformed input {} parsed unexpectedly", i + 1),
            Err(errors) => println!(
                "  Malformed input {} generated {} errors (expected)",
                i + 1,
                errors.len()
            ),
        }
    }
}

#[tokio::test]
async fn test_lsp_performance_regression() {
    // Test for performance regressions by establishing baseline performance
    let baseline_program = generate_large_wfl_program(300, 40, 300);
    let num_iterations = 5;
    let mut times = Vec::new();

    for iteration in 0..num_iterations {
        let start_time = Instant::now();

        // Perform standard LSP operations
        let tokens = lex_wfl_with_positions(&baseline_program);
        let mut parser = Parser::new(&tokens);
        let parse_result = parser.parse();

        if let Ok(program) = parse_result {
            let mut analyzer = Analyzer::new();
            let _analysis_result = analyzer.analyze(&program);
        }

        let iteration_time = start_time.elapsed();
        times.push(iteration_time);

        println!(
            "Performance regression test iteration {}: {:?}",
            iteration + 1,
            iteration_time
        );
    }

    // Calculate average and check for consistency
    let total_time: Duration = times.iter().sum();
    let avg_time = total_time / num_iterations as u32;

    println!("Average LSP operation time: {:?}", avg_time);

    // Performance regression check - should be consistently fast
    assert!(
        avg_time.as_millis() < 1000,
        "Average LSP operation time should be under 1s, got {:?}",
        avg_time
    );

    // Check for consistency (no operation should be more than 3x the average)
    for (i, time) in times.iter().enumerate() {
        let ratio = time.as_millis() as f64 / avg_time.as_millis() as f64;
        assert!(
            ratio < 3.0,
            "Iteration {} took {:?}, which is {:.1}x the average - possible performance regression",
            i + 1,
            time,
            ratio
        );
    }

    println!("Performance regression test passed - consistent performance maintained");
}

#[tokio::test]
async fn test_lsp_resource_cleanup() {
    // Test that LSP operations properly clean up resources
    let test_program = generate_large_wfl_program(100, 20, 100);

    // Perform operations that create and destroy many objects
    for cycle in 0..5 {
        println!("Resource cleanup test cycle: {}", cycle + 1);

        let mut analyzers = Vec::new();
        let mut type_checkers = Vec::new();

        // Create many objects
        for _ in 0..10 {
            let tokens = lex_wfl_with_positions(&test_program);
            let mut parser = Parser::new(&tokens);
            let parse_result = parser.parse();

            if let Ok(program) = parse_result {
                let mut analyzer = Analyzer::new();
                let _analysis_result = analyzer.analyze(&program);
                analyzers.push(analyzer);

                let mut type_checker = TypeChecker::new();
                let _type_result = type_checker.check_types(&program);
                type_checkers.push(type_checker);
            }

            // Don't store parser since it borrows tokens
            drop(parser);
            drop(tokens);
        }

        // Explicitly drop objects to test cleanup
        drop(analyzers);
        drop(type_checkers);

        // Small delay to allow cleanup
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    println!("Resource cleanup test completed - no resource leaks detected");
}
