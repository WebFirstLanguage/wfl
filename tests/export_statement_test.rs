use std::fs;
use std::io::Write;
use tempfile::Builder;
use wfl::analyzer::Analyzer;
use wfl::interpreter::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

#[tokio::test]
async fn test_export_container_statement_syntax() {
    // Test basic export syntax for containers
    // Create a temporary file with .wfl extension
    let mut temp_file = Builder::new()
        .suffix(".wfl")
        .tempfile()
        .expect("Failed to create temp file");

    // Create file with export container statement
    let content = r#"
create container Person:
    property name: Text
    property age: Number
end

export container Person
"#;
    temp_file
        .write_all(content.as_bytes())
        .expect("Failed to write test file");

    // Test parsing
    let test_file_path = temp_file.path();
    let source = fs::read_to_string(test_file_path).expect("Failed to read test file");
    let tokens = lex_wfl_with_positions(&source);

    let mut parser = Parser::new(&tokens);
    let ast = parser
        .parse()
        .unwrap_or_else(|e| panic!("Parse failed: {:?}", e));

    // Should succeed after export syntax is implemented
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&ast).await;

    match result {
        Ok(_) => {
            assert!(true, "Export container statement should parse and execute");
        }
        Err(e) => {
            println!("Execution error: {:?}", e);
            assert!(
                false,
                "Export container statement should execute without error"
            );
        }
    }

    // Temp file is automatically cleaned up when dropped
}

#[tokio::test]
async fn test_export_action_statement_syntax() {
    // Test export syntax for actions
    // Create a temporary file with .wfl extension
    let mut temp_file = Builder::new()
        .suffix(".wfl")
        .tempfile()
        .expect("Failed to create temp file");

    // Create file with export action statement
    let content = r#"
define action called helper with parameters msg:
    display "Helper: " with msg
end action

export action helper
"#;
    temp_file
        .write_all(content.as_bytes())
        .expect("Failed to write test file");

    // Test parsing
    let test_file_path = temp_file.path();
    let source = fs::read_to_string(test_file_path).expect("Failed to read test file");
    let tokens = lex_wfl_with_positions(&source);

    let mut parser = Parser::new(&tokens);
    let ast = parser
        .parse()
        .unwrap_or_else(|e| panic!("Parse failed: {:?}", e));

    // Should succeed after export syntax is implemented
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&ast).await;

    match result {
        Ok(_) => {
            assert!(true, "Export action statement should parse and execute");
        }
        Err(e) => {
            println!("Execution error: {:?}", e);
            assert!(
                false,
                "Export action statement should execute without error"
            );
        }
    }

    // Temp file is automatically cleaned up when dropped
}

#[tokio::test]
async fn test_export_constant_statement_syntax() {
    // Test export syntax for constants
    // Create a temporary file with .wfl extension
    let mut temp_file = Builder::new()
        .suffix(".wfl")
        .tempfile()
        .expect("Failed to create temp file");

    // Create file with export constant statement
    let content = r#"
store new constant VERSION as "1.0.0"
store new constant MAX_SIZE as 100

export constant VERSION
export constant MAX_SIZE
"#;
    temp_file
        .write_all(content.as_bytes())
        .expect("Failed to write test file");

    // Test parsing
    let test_file_path = temp_file.path();
    let source = fs::read_to_string(test_file_path).expect("Failed to read test file");
    let tokens = lex_wfl_with_positions(&source);

    let mut parser = Parser::new(&tokens);
    let ast = parser
        .parse()
        .unwrap_or_else(|e| panic!("Parse failed: {:?}", e));

    // Should succeed after export syntax is implemented
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&ast).await;

    match result {
        Ok(_) => {
            assert!(true, "Export constant statement should parse and execute");
        }
        Err(e) => {
            println!("Execution error: {:?}", e);
            assert!(
                false,
                "Export constant statement should execute without error"
            );
        }
    }

    // Temp file is automatically cleaned up when dropped
}

#[tokio::test]
async fn test_export_nonexistent_item_error() {
    // Test that exporting non-existent items produces appropriate errors
    // Create a temporary file with .wfl extension
    let mut temp_file = Builder::new()
        .suffix(".wfl")
        .tempfile()
        .expect("Failed to create temp file");

    // Create file that tries to export non-existent items
    let content = r#"
export container NonExistentContainer
export action non_existent_action
export constant MISSING_CONSTANT
"#;
    temp_file
        .write_all(content.as_bytes())
        .expect("Failed to write test file");

    // Test parsing and execution
    let test_file_path = temp_file.path();
    let source = fs::read_to_string(test_file_path).expect("Failed to read test file");
    let tokens = lex_wfl_with_positions(&source);

    let mut parser = Parser::new(&tokens);
    let ast = parser
        .parse()
        .unwrap_or_else(|e| panic!("Parse failed: {:?}", e));

    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&ast).await;

    match result {
        Ok(_) => {
            assert!(false, "Exporting non-existent items should produce error");
        }
        Err(e) => {
            // Should fail with appropriate error message after implementation
            assert!(
                true,
                "Export of non-existent items should fail with error: {:?}",
                e
            );
        }
    }

    // Temp file is automatically cleaned up when dropped
}

#[tokio::test]
async fn test_export_statement_order_independence() {
    // Test that export statements can appear before or after definitions
    // Create a temporary file with .wfl extension
    let mut temp_file = Builder::new()
        .suffix(".wfl")
        .tempfile()
        .expect("Failed to create temp file");

    // Create file with exports after definitions (forward declaration not yet supported)
    let content = r#"
create container ForwardDeclaredContainer:
    property value: Number
end

export container ForwardDeclaredContainer

define action called utility with parameters:
    display "utility action"
end action

export action utility
"#;
    temp_file
        .write_all(content.as_bytes())
        .expect("Failed to write test file");

    // Test parsing and execution
    let test_file_path = temp_file.path();
    let source = fs::read_to_string(test_file_path).expect("Failed to read test file");
    let tokens = lex_wfl_with_positions(&source);

    let mut parser = Parser::new(&tokens);
    let ast = parser
        .parse()
        .unwrap_or_else(|e| panic!("Parse failed: {:?}", e));

    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&ast).await;

    match result {
        Ok(_) => {
            // Export statements should succeed when they come after definitions
            assert!(true, "Export statements should work when after definitions");
        }
        Err(e) => {
            panic!(
                "Export statements should work when definitions come first: {:?}",
                e
            );
        }
    }

    // Temp file is automatically cleaned up when dropped
}

#[test]
fn test_export_mutable_variable_as_constant_fails() {
    // Test that mutable variables cannot be exported as constants
    let code = r#"
store mutable_var as 42
export constant mutable_var
"#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    // Analyze the program
    let mut analyzer = Analyzer::new();
    analyzer.analyze(&program).expect("Should analyze");

    // Type check should fail because mutable_var is mutable
    let mut type_checker = TypeChecker::with_analyzer(analyzer);
    let result = type_checker.check_types(&program);

    assert!(
        result.is_err(),
        "Type checking should fail when exporting mutable variable as constant"
    );

    let errors = result.unwrap_err();
    assert!(
        !errors.is_empty(),
        "Should have type errors for mutable constant export"
    );

    let error_msg = errors[0].to_string();
    assert!(
        error_msg.contains("mutable") && error_msg.contains("cannot be exported as constant"),
        "Error message should indicate mutable variable cannot be exported as constant: {}",
        error_msg
    );
}

#[test]
fn test_export_immutable_variable_as_constant_succeeds() {
    // Test that immutable variables can be exported as constants
    let code = r#"
store new constant my_version as "1.0.0"
export constant my_version
"#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    // Analyze the program
    let mut analyzer = Analyzer::new();
    analyzer.analyze(&program).expect("Should analyze");

    // Type check should succeed because VERSION is immutable
    let mut type_checker = TypeChecker::with_analyzer(analyzer);
    let result = type_checker.check_types(&program);

    assert!(
        result.is_ok(),
        "Type checking should succeed when exporting immutable variable as constant: {:?}",
        result.err()
    );
}
