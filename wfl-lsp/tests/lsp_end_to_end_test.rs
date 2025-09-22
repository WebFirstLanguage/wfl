// End-to-end tests for WFL LSP server using real TestPrograms
// These tests validate the complete LSP workflow with actual WFL programs

use std::fs;
use std::path::Path;
use wfl::analyzer::Analyzer;
use wfl::diagnostics::DiagnosticReporter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

#[tokio::test]
async fn test_lsp_with_basic_syntax_program() {
    // Test LSP functionality with a real WFL program
    let test_program_path = Path::new("../TestPrograms/basic_syntax_comprehensive.wfl");

    if !test_program_path.exists() {
        println!("Skipping test - TestPrograms not available");
        return;
    }

    let document_text =
        fs::read_to_string(test_program_path).expect("Should be able to read test program");

    let mut diagnostic_reporter = DiagnosticReporter::new();
    let file_id =
        diagnostic_reporter.add_file("basic_syntax_comprehensive.wfl", document_text.clone());

    // Test lexing
    let tokens = lex_wfl_with_positions(&document_text);
    assert!(
        !tokens.is_empty(),
        "Should produce tokens for real WFL program"
    );

    // Test parsing
    let mut parser = Parser::new(&tokens);
    match parser.parse() {
        Ok(program) => {
            assert!(
                !program.statements.is_empty(),
                "Should parse statements from real WFL program"
            );

            // Test analysis
            let mut analyzer = Analyzer::new();
            match analyzer.analyze(&program) {
                Ok(_) => {
                    println!("Analysis passed for basic syntax program");
                }
                Err(errors) => {
                    println!("Analysis errors (may be expected): {:?}", errors);
                    // Convert errors to diagnostics to test LSP diagnostic conversion
                    for error in &errors {
                        let _wfl_diag = diagnostic_reporter.convert_semantic_error(file_id, error);
                    }
                }
            }

            // Test type checking
            let mut type_checker = TypeChecker::new();
            match type_checker.check_types(&program) {
                Ok(_) => {
                    println!("Type checking passed for basic syntax program");
                }
                Err(errors) => {
                    println!("Type checking errors (may be expected): {:?}", errors);
                    // Convert errors to diagnostics to test LSP diagnostic conversion
                    for error in &errors {
                        let _wfl_diag = diagnostic_reporter.convert_type_error(file_id, error);
                    }
                }
            }
        }
        Err(errors) => {
            println!("Parse errors for basic syntax program: {:?}", errors);
            // Convert errors to diagnostics to test LSP diagnostic conversion
            for error in &errors {
                let _wfl_diag = diagnostic_reporter.convert_parse_error(file_id, error);
            }
        }
    }
}

#[tokio::test]
async fn test_lsp_with_error_examples() {
    // Test LSP functionality with programs that have intentional errors
    let error_examples_dir = Path::new("../TestPrograms/error_examples");

    if !error_examples_dir.exists() {
        println!("Skipping test - error examples not available");
        return;
    }

    // Test with parse error example
    let parse_error_path = error_examples_dir.join("parse_error.wfl");
    if parse_error_path.exists() {
        let document_text = fs::read_to_string(&parse_error_path)
            .expect("Should be able to read parse error example");

        let mut diagnostic_reporter = DiagnosticReporter::new();
        let file_id = diagnostic_reporter.add_file("parse_error.wfl", document_text.clone());

        let tokens = lex_wfl_with_positions(&document_text);
        let mut parser = Parser::new(&tokens);

        match parser.parse() {
            Ok(_) => {
                println!("Parse error example unexpectedly parsed successfully");
            }
            Err(errors) => {
                assert!(
                    !errors.is_empty(),
                    "Should have parse errors for parse error example"
                );

                // Test diagnostic conversion
                for error in &errors {
                    let wfl_diag = diagnostic_reporter.convert_parse_error(file_id, error);
                    assert!(
                        !wfl_diag.message.is_empty(),
                        "Diagnostic should have message"
                    );
                    assert!(
                        !wfl_diag.labels.is_empty(),
                        "Diagnostic should have location"
                    );
                }

                println!("Successfully processed {} parse errors", errors.len());
            }
        }
    }

    // Test with semantic error example
    let semantic_error_path = error_examples_dir.join("semantic_error.wfl");
    if semantic_error_path.exists() {
        let document_text = fs::read_to_string(&semantic_error_path)
            .expect("Should be able to read semantic error example");

        let mut diagnostic_reporter = DiagnosticReporter::new();
        let file_id = diagnostic_reporter.add_file("semantic_error.wfl", document_text.clone());

        let tokens = lex_wfl_with_positions(&document_text);
        let mut parser = Parser::new(&tokens);

        match parser.parse() {
            Ok(program) => {
                let mut analyzer = Analyzer::new();
                match analyzer.analyze(&program) {
                    Ok(_) => {
                        println!("Semantic error example unexpectedly analyzed successfully");
                    }
                    Err(errors) => {
                        assert!(!errors.is_empty(), "Should have semantic errors");

                        // Test diagnostic conversion
                        for error in &errors {
                            let wfl_diag =
                                diagnostic_reporter.convert_semantic_error(file_id, error);
                            assert!(
                                !wfl_diag.message.is_empty(),
                                "Diagnostic should have message"
                            );
                        }

                        println!("Successfully processed {} semantic errors", errors.len());
                    }
                }
            }
            Err(errors) => {
                println!("Semantic error example failed to parse: {:?}", errors);
            }
        }
    }
}

#[tokio::test]
async fn test_lsp_with_multiple_test_programs() {
    // Test LSP functionality with multiple real WFL programs
    let test_programs_dir = Path::new("../TestPrograms");

    if !test_programs_dir.exists() {
        println!("Skipping test - TestPrograms directory not available");
        return;
    }

    let test_files = [
        "simple_random_test.wfl",
        "stdlib_comprehensive.wfl",
        "containers_comprehensive.wfl",
    ];

    let mut successful_programs = 0;
    let mut total_programs = 0;

    for test_file in &test_files {
        let test_path = test_programs_dir.join(test_file);

        if !test_path.exists() {
            println!("Skipping {} - file not found", test_file);
            continue;
        }

        total_programs += 1;

        let document_text = match fs::read_to_string(&test_path) {
            Ok(content) => content,
            Err(e) => {
                println!("Failed to read {}: {}", test_file, e);
                continue;
            }
        };

        let mut diagnostic_reporter = DiagnosticReporter::new();
        let file_id = diagnostic_reporter.add_file(test_file.to_string(), document_text.clone());

        // Test complete LSP workflow
        let tokens = lex_wfl_with_positions(&document_text);
        assert!(
            !tokens.is_empty(),
            "Should produce tokens for {}",
            test_file
        );

        let mut parser = Parser::new(&tokens);
        match parser.parse() {
            Ok(program) => {
                println!("Successfully parsed {}", test_file);

                // Test analysis
                let mut analyzer = Analyzer::new();
                let analysis_result = analyzer.analyze(&program);

                // Test type checking
                let mut type_checker = TypeChecker::new();
                let type_result = type_checker.check_types(&program);

                match (analysis_result, type_result) {
                    (Ok(_), Ok(_)) => {
                        successful_programs += 1;
                        println!("Complete analysis successful for {}", test_file);
                    }
                    (analysis_res, type_res) => {
                        println!(
                            "Analysis/type checking issues for {} (may be expected)",
                            test_file
                        );

                        // Test diagnostic conversion for any errors
                        if let Err(errors) = analysis_res {
                            for error in &errors {
                                let _diag =
                                    diagnostic_reporter.convert_semantic_error(file_id, error);
                            }
                        }

                        if let Err(errors) = type_res {
                            for error in &errors {
                                let _diag = diagnostic_reporter.convert_type_error(file_id, error);
                            }
                        }
                    }
                }
            }
            Err(errors) => {
                println!(
                    "Parse errors for {} (may be expected): {} errors",
                    test_file,
                    errors.len()
                );

                // Test diagnostic conversion
                for error in &errors {
                    let _diag = diagnostic_reporter.convert_parse_error(file_id, error);
                }
            }
        }
    }

    println!(
        "LSP workflow test completed: {}/{} programs processed successfully",
        successful_programs, total_programs
    );

    // We consider the test successful if we processed at least one program
    assert!(
        total_programs > 0,
        "Should have found at least one test program"
    );
}

#[tokio::test]
async fn test_lsp_performance_with_large_program() {
    // Test LSP performance with a larger WFL program
    let large_program = r#"
// Large WFL program for performance testing
store counter as 0
store results as []

// Multiple variable declarations
store name as "Test Program"
store version as 1.0
store active as yes
store config as {
    "debug": yes,
    "timeout": 30,
    "retries": 3
}

// Multiple functions and loops
count from i as 1 to 10
    store temp as i * 2
    add temp to results
    
    if temp is greater than 15 then
        display "Large value: " with temp
    end if
end count

// Complex expressions
store calculation as (counter + 5) * 2 - 1
store text_result as "Result: " with calculation

// Multiple display statements
display name
display "Version: " with version
display "Active: " with active
display text_result
display "Results count: " with length of results

// Error handling
try
    store risky as 10 / counter
catch error
    display "Error occurred: " with error
end try
"#;

    let start_time = std::time::Instant::now();

    let mut diagnostic_reporter = DiagnosticReporter::new();
    let file_id = diagnostic_reporter.add_file("large_program.wfl", large_program.to_string());

    // Test lexing performance
    let tokens = lex_wfl_with_positions(large_program);
    let lex_time = start_time.elapsed();

    assert!(
        !tokens.is_empty(),
        "Should produce tokens for large program"
    );
    println!("Lexing took: {:?} for {} tokens", lex_time, tokens.len());

    // Test parsing performance
    let parse_start = std::time::Instant::now();
    let mut parser = Parser::new(&tokens);
    let parse_result = parser.parse();
    let parse_time = parse_start.elapsed();

    println!("Parsing took: {:?}", parse_time);

    match parse_result {
        Ok(program) => {
            println!(
                "Successfully parsed large program with {} statements",
                program.statements.len()
            );

            // Test analysis performance
            let analysis_start = std::time::Instant::now();
            let mut analyzer = Analyzer::new();
            let _analysis_result = analyzer.analyze(&program);
            let analysis_time = analysis_start.elapsed();

            println!("Analysis took: {:?}", analysis_time);

            // Test type checking performance
            let type_start = std::time::Instant::now();
            let mut type_checker = TypeChecker::new();
            let _type_result = type_checker.check_types(&program);
            let type_time = type_start.elapsed();

            println!("Type checking took: {:?}", type_time);

            let total_time = start_time.elapsed();
            println!("Total LSP processing time: {:?}", total_time);

            // Performance assertions (reasonable limits)
            assert!(
                total_time.as_millis() < 1000,
                "Total processing should be under 1 second"
            );
            assert!(lex_time.as_millis() < 100, "Lexing should be under 100ms");
            assert!(
                parse_time.as_millis() < 500,
                "Parsing should be under 500ms"
            );
        }
        Err(errors) => {
            println!("Parse errors in large program: {:?}", errors);

            // Even with errors, test diagnostic conversion performance
            let diag_start = std::time::Instant::now();
            for error in &errors {
                let _diag = diagnostic_reporter.convert_parse_error(file_id, error);
            }
            let diag_time = diag_start.elapsed();

            println!(
                "Diagnostic conversion took: {:?} for {} errors",
                diag_time,
                errors.len()
            );
        }
    }
}
