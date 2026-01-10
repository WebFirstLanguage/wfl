//! Tests for the export statement functionality
//!
//! The export statement allows modules to explicitly expose containers and definitions
//! to the parent scope when loaded with `load module`.

use std::path::PathBuf;
use std::fs;
use tempfile::TempDir;
use wfl::interpreter::Interpreter;
use wfl::parser::ast::{Program, Statement, Expression, Literal};
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

#[tokio::test]
async fn test_export_container_makes_it_available_to_parent() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a module with exported container
    let module_path = temp_dir.path().join("person_module.wfl");
    fs::write(&module_path, r#"
create container Person:
    property name: Text
    property age: Number
    action greet:
        display "Hello, I am " + name
    end
end

export container Person
"#).unwrap();

    // Main program that loads the module
    let main_content = format!(r#"
load module from "{}"

create new Person as alice:
    name is "Alice"
    age is 30
end
"#, module_path.to_string_lossy());

    let tokens = lex_wfl_with_positions(&main_content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse load module with export");
    
    let mut interpreter = Interpreter::new();
    
    // This should work because Person is exported
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Exported container should be available in parent scope");
}

#[tokio::test]
async fn test_non_exported_container_not_available_to_parent() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a module without export
    let module_path = temp_dir.path().join("private_module.wfl");
    fs::write(&module_path, r#"
create container PrivateContainer:
    property value: Number
end

# No export statement - container should remain private to module
"#).unwrap();

    // Main program tries to use non-exported container
    let main_content = format!(r#"
load module from "{}"

create new PrivateContainer as instance:
    value is 42
end
"#, module_path.to_string_lossy());

    let tokens = lex_wfl_with_positions(&main_content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse load module statement");
    
    let mut interpreter = Interpreter::new();
    
    // This should fail because PrivateContainer is not exported
    let result = interpreter.interpret(&program).await;
    assert!(result.is_err(), "Non-exported container should NOT be available in parent");
}

#[tokio::test]
async fn test_export_multiple_containers() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a module with multiple containers, some exported
    let module_path = temp_dir.path().join("multi_container_module.wfl");
    fs::write(&module_path, r#"
create container PublicContainer1:
    property name: Text
end

create container PrivateContainer:
    property secret: Text
end

create container PublicContainer2:
    property value: Number
end

export container PublicContainer1
export container PublicContainer2
# PrivateContainer is not exported
"#).unwrap();

    // Test that exported containers are available
    let main_content = format!(r#"
load module from "{}"

create new PublicContainer1 as pub1:
    name is "Public1"
end

create new PublicContainer2 as pub2:
    value is 42
end
"#, module_path.to_string_lossy());

    let tokens = lex_wfl_with_positions(&main_content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");
    
    let mut interpreter = Interpreter::new();
    // Environment is handled internally by interpreter
    
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Both exported containers should be available");

    // Test that non-exported container is not available
    let private_content = format!(r#"
load module from "{}"

create new PrivateContainer as private:
    secret is "hidden"
end
"#, module_path.to_string_lossy());

    let tokens = lex_wfl_with_positions(&private_content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");
    
    let mut interpreter = Interpreter::new();
    // Environment is handled internally by interpreter
    
    let result = interpreter.interpret(&program).await;
    assert!(result.is_err(), "Non-exported PrivateContainer should NOT be available");
}

#[tokio::test]
async fn test_export_variables_and_constants() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a module with exported variables/constants
    let module_path = temp_dir.path().join("exports_module.wfl");
    fs::write(&module_path, r#"
store shared_config as "production"
define APP_VERSION as "1.0.0"
store internal_cache as "private"

export variable shared_config
export constant APP_VERSION
# internal_cache is not exported
"#).unwrap();

    // Test that exported items are available
    let main_content = format!(r#"
load module from "{}"

display shared_config
display APP_VERSION
"#, module_path.to_string_lossy());

    let tokens = lex_wfl_with_positions(&main_content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");
    
    let mut interpreter = Interpreter::new();
    // Environment is handled internally by interpreter
    
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Exported variables/constants should be available");

    // Test that non-exported variable is not available
    let private_content = format!(r#"
load module from "{}"

display internal_cache
"#, module_path.to_string_lossy());

    let tokens = lex_wfl_with_positions(&private_content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");
    
    let mut interpreter = Interpreter::new();
    // Environment is handled internally by interpreter
    
    let result = interpreter.interpret(&program).await;
    assert!(result.is_err(), "Non-exported variable should NOT be available");
}

#[tokio::test]
async fn test_export_statement_parsing() {
    // Test export container parsing
    let content = r#"export container MyContainer"#;
    let tokens = lex_wfl_with_positions(content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse();
    
    assert!(program.is_ok(), "Should parse export container statement");
    let program = program.unwrap();
    assert_eq!(program.statements.len(), 1);
    
    match &program.statements[0] {
        Statement::ExportStatement { export_type, name, .. } => {
            assert_eq!(export_type, "container");
            assert_eq!(name, "MyContainer");
        }
        _ => panic!("Expected ExportStatement, got {:?}", program.statements[0]),
    }

    // Test export variable parsing
    let content = r#"export variable my_var"#;
    let tokens = lex_wfl_with_positions(content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse();
    
    assert!(program.is_ok(), "Should parse export variable statement");
    let program = program.unwrap();
    
    match &program.statements[0] {
        Statement::ExportStatement { export_type, name, .. } => {
            assert_eq!(export_type, "variable");
            assert_eq!(name, "my_var");
        }
        _ => panic!("Expected ExportStatement for variable"),
    }

    // Test export constant parsing
    let content = r#"export constant MY_CONST"#;
    let tokens = lex_wfl_with_positions(content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse();
    
    assert!(program.is_ok(), "Should parse export constant statement");
    let program = program.unwrap();
    
    match &program.statements[0] {
        Statement::ExportStatement { export_type, name, .. } => {
            assert_eq!(export_type, "constant");
            assert_eq!(name, "MY_CONST");
        }
        _ => panic!("Expected ExportStatement for constant"),
    }
}

#[tokio::test]
async fn test_export_action_functions() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a module with exported actions
    let module_path = temp_dir.path().join("actions_module.wfl");
    fs::write(&module_path, r#"
define action called public_helper with parameters value:
    display "Helper: " + value
    return value * 2
end

define action called private_helper with parameters x:
    return x + 1
end

export action public_helper
# private_helper is not exported
"#).unwrap();

    // Test that exported action is available
    let main_content = format!(r#"
load module from "{}"

call public_helper with "test"
"#, module_path.to_string_lossy());

    let tokens = lex_wfl_with_positions(&main_content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");
    
    let mut interpreter = Interpreter::new();
    // Environment is handled internally by interpreter
    
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Exported action should be available");

    // Test that non-exported action is not available
    let private_content = format!(r#"
load module from "{}"

call private_helper with 5
"#, module_path.to_string_lossy());

    let tokens = lex_wfl_with_positions(&private_content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");
    
    let mut interpreter = Interpreter::new();
    // Environment is handled internally by interpreter
    
    let result = interpreter.interpret(&program).await;
    assert!(result.is_err(), "Non-exported action should NOT be available");
}