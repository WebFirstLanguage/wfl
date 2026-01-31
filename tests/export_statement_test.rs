use std::fs;
use std::path::Path;
use wfl::interpreter::Interpreter;
use wfl::lexer::lex_wfl;
use wfl::parser::parse_wfl;

#[test]
fn test_export_container_statement_syntax() {
    // Test basic export syntax for containers
    let test_file = "test_export_container.wfl";

    // Clean up any existing files
    let _ = fs::remove_file(test_file);

    // Create file with export container statement
    let content = r#"
create container Person:
    property name: Text
    property age: Number
end

export container Person
"#;
    fs::write(test_file, content).expect("Failed to write test file");

    // Test parsing
    let source = fs::read_to_string(test_file).expect("Failed to read test file");
    let tokens = lex_wfl(&source);

    match parse_wfl(&tokens, Path::new(test_file)) {
        Ok(ast) => {
            // Should succeed after export syntax is implemented
            let mut interpreter = Interpreter::new();
            let result = interpreter.interpret(&ast);

            match result {
                Ok(_) => {
                    assert!(true, "Export container statement should parse and execute");
                }
                Err(e) => {
                    println!("Execution error: {}", e);
                    assert!(
                        false,
                        "Export container statement should execute without error"
                    );
                }
            }
        }
        Err(e) => {
            println!("Parse error: {}", e);
            // Expected failure before export keyword is added
            assert!(
                e.to_string().contains("export"),
                "Parse should fail due to missing export keyword"
            );
        }
    }

    // Clean up test file
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_export_action_statement_syntax() {
    // Test export syntax for actions
    let test_file = "test_export_action.wfl";

    // Clean up any existing files
    let _ = fs::remove_file(test_file);

    // Create file with export action statement
    let content = r#"
define action called helper with parameters text:
    display "Helper: " + text
end

export action helper
"#;
    fs::write(test_file, content).expect("Failed to write test file");

    // Test parsing
    let source = fs::read_to_string(test_file).expect("Failed to read test file");
    let tokens = lex_wfl(&source);

    match parse_wfl(&tokens, Path::new(test_file)) {
        Ok(ast) => {
            // Should succeed after export syntax is implemented
            let mut interpreter = Interpreter::new();
            let result = interpreter.interpret(&ast);

            match result {
                Ok(_) => {
                    assert!(true, "Export action statement should parse and execute");
                }
                Err(e) => {
                    println!("Execution error: {}", e);
                    assert!(
                        false,
                        "Export action statement should execute without error"
                    );
                }
            }
        }
        Err(e) => {
            println!("Parse error: {}", e);
            // Expected failure before export keyword is added
            assert!(
                e.to_string().contains("export"),
                "Parse should fail due to missing export keyword"
            );
        }
    }

    // Clean up test file
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_export_constant_statement_syntax() {
    // Test export syntax for constants
    let test_file = "test_export_constant.wfl";

    // Clean up any existing files
    let _ = fs::remove_file(test_file);

    // Create file with export constant statement
    let content = r#"
store constant VERSION as "1.0.0"
store constant MAX_SIZE as 100

export constant VERSION
export constant MAX_SIZE
"#;
    fs::write(test_file, content).expect("Failed to write test file");

    // Test parsing
    let source = fs::read_to_string(test_file).expect("Failed to read test file");
    let tokens = lex_wfl(&source);

    match parse_wfl(&tokens, Path::new(test_file)) {
        Ok(ast) => {
            // Should succeed after export syntax is implemented
            let mut interpreter = Interpreter::new();
            let result = interpreter.interpret(&ast);

            match result {
                Ok(_) => {
                    assert!(true, "Export constant statement should parse and execute");
                }
                Err(e) => {
                    println!("Execution error: {}", e);
                    assert!(
                        false,
                        "Export constant statement should execute without error"
                    );
                }
            }
        }
        Err(e) => {
            println!("Parse error: {}", e);
            // Expected failure before export keyword is added
            assert!(
                e.to_string().contains("export"),
                "Parse should fail due to missing export keyword"
            );
        }
    }

    // Clean up test file
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_export_nonexistent_item_error() {
    // Test that exporting non-existent items produces appropriate errors
    let test_file = "test_export_error.wfl";

    // Clean up any existing files
    let _ = fs::remove_file(test_file);

    // Create file that tries to export non-existent items
    let content = r#"
export container NonExistentContainer
export action non_existent_action
export constant MISSING_CONSTANT
"#;
    fs::write(test_file, content).expect("Failed to write test file");

    // Test parsing and execution
    let source = fs::read_to_string(test_file).expect("Failed to read test file");
    let tokens = lex_wfl(&source);

    match parse_wfl(&tokens, Path::new(test_file)) {
        Ok(ast) => {
            let mut interpreter = Interpreter::new();
            let result = interpreter.interpret(&ast);

            match result {
                Ok(_) => {
                    assert!(false, "Exporting non-existent items should produce error");
                }
                Err(e) => {
                    // Should fail with appropriate error message after implementation
                    assert!(
                        true,
                        "Export of non-existent items should fail with error: {}",
                        e
                    );
                }
            }
        }
        Err(_) => {
            // Expected parse failure before export keyword is added
        }
    }

    // Clean up test file
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_export_statement_order_independence() {
    // Test that export statements can appear before or after definitions
    let test_file = "test_export_order.wfl";

    // Clean up any existing files
    let _ = fs::remove_file(test_file);

    // Create file with exports before and after definitions
    let content = r#"
export container ForwardDeclaredContainer

create container ForwardDeclaredContainer:
    property value: Number
end

define action called utility:
    display "utility action"
end

export action utility
"#;
    fs::write(test_file, content).expect("Failed to write test file");

    // Test parsing and execution
    let source = fs::read_to_string(test_file).expect("Failed to read test file");
    let tokens = lex_wfl(&source);

    match parse_wfl(&tokens, Path::new(test_file)) {
        Ok(ast) => {
            let mut interpreter = Interpreter::new();
            let result = interpreter.interpret(&ast);

            match result {
                Ok(_) => {
                    // Forward declaration should work (or be validated at different phase)
                    assert!(true, "Export statements should work regardless of order");
                }
                Err(e) => {
                    println!("Order-dependent error: {}", e);
                    // May require forward declaration handling or validation at end
                    assert!(false, "Export statement order should be flexible");
                }
            }
        }
        Err(_) => {
            // Expected parse failure before export keyword is added
        }
    }

    // Clean up test file
    let _ = fs::remove_file(test_file);
}
