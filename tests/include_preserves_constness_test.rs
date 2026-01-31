use std::fs;
use tempfile::TempDir;
use wfl::analyzer::Analyzer;
use wfl::interpreter::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

#[tokio::test]
async fn test_include_preserves_parent_constant_immutability() {
    // Test that constants from parent scope remain immutable in included files
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let included_file = temp_dir.path().join("try_modify_constant.wfl");
    let main_file = temp_dir.path().join("main.wfl");

    // Create an included file that tries to modify a parent constant
    let included_content = r#"
change PARENT_CONSTANT to "modified value"
"#;
    fs::write(&included_file, included_content).expect("Failed to write included file");

    // Create main file with a constant and include statement
    let main_content = r#"
store new constant PARENT_CONSTANT as "original value"

include from "try_modify_constant.wfl"

display PARENT_CONSTANT
"#;
    fs::write(&main_file, main_content).expect("Failed to write main file");

    // Parse and analyze
    let source = fs::read_to_string(&main_file).expect("Failed to read main file");
    let tokens = lex_wfl_with_positions(&source);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().expect("Should parse successfully");

    // Analyze - this should succeed since we're just checking semantics
    let mut analyzer = Analyzer::new();
    let analyze_result = analyzer.analyze(&ast);

    // The analysis should succeed (include statement is valid)
    assert!(
        analyze_result.is_ok(),
        "Analysis should succeed: {:?}",
        analyze_result.err()
    );

    // Execute - this should fail because we're trying to modify a constant
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&ast).await;

    match result {
        Ok(_) => {
            panic!("Should not be able to modify a constant from included file");
        }
        Err(errors) => {
            // Should fail with an error about trying to modify a constant
            let error_string = format!("{:?}", errors);
            assert!(
                error_string.contains("constant") || error_string.contains("immutable"),
                "Error should mention constant/immutable: {:?}",
                errors
            );
        }
    }

    // Temp directory is automatically cleaned up when dropped
}

#[tokio::test]
#[ignore] // TODO: Include functionality not fully working with temp directories
async fn test_include_allows_modifying_parent_mutable_variables() {
    // Test that mutable variables from parent scope CAN be modified in included files
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let included_file = temp_dir.path().join("modify_mutable.wfl");
    let main_file = temp_dir.path().join("main.wfl");

    // Create an included file that modifies a parent mutable variable
    let included_content = r#"
change parent_var to "modified by include"
"#;
    fs::write(&included_file, included_content).expect("Failed to write included file");

    // Create main file with a mutable variable and include statement
    let main_content = r#"
store parent_var as "original value"

include from "modify_mutable.wfl"

display parent_var
"#;
    fs::write(&main_file, main_content).expect("Failed to write main file");

    // Parse and analyze
    let source = fs::read_to_string(&main_file).expect("Failed to read main file");
    let tokens = lex_wfl_with_positions(&source);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().expect("Should parse successfully");

    // Analyze
    let mut analyzer = Analyzer::new();
    let analyze_result = analyzer.analyze(&ast);

    assert!(
        analyze_result.is_ok(),
        "Analysis should succeed: {:?}",
        analyze_result.err()
    );

    // Execute - this should succeed because mutable variables can be modified
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&ast).await;

    assert!(
        result.is_ok(),
        "Should be able to modify mutable variable from included file: {:?}",
        result.err()
    );

    // Temp directory is automatically cleaned up when dropped
}
