//! Tests for the include statement functionality
//! 
//! The include statement executes modules in the parent scope instead of isolated child scope,
//! allowing modules to expose containers and other definitions to the parent.

use std::path::PathBuf;
use std::fs;
use tempfile::TempDir;
use wfl::interpreter::Interpreter;
use wfl::parser::ast::{Program, Statement, Expression, Literal};
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

#[tokio::test]
async fn test_include_statement_exposes_container_to_parent() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a module file with a container definition
    let module_path = temp_dir.path().join("person_module.wfl");
    fs::write(&module_path, r#"
create container Person:
    property name: Text
    property age: Number
    action greet:
        display "Hello, I am " + name
    end
end
"#).unwrap();

    // Main program that includes the module
    let main_content = format!(r#"
include from "{}"

create new Person as alice:
    name is "Alice"
    age is 30
end
"#, module_path.to_string_lossy());

    // This should work (unlike load module which would fail)
    let tokens = lex_wfl_with_positions(&main_content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse include statement");
    
    let mut interpreter = Interpreter::new();
    
    // This should execute successfully - the Person container should be available
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Include should make container available in parent scope");
}

#[tokio::test]
async fn test_include_statement_shares_parent_variables() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a module that modifies a parent variable
    let module_path = temp_dir.path().join("modifier_module.wfl");
    fs::write(&module_path, r#"
store shared_counter as shared_counter + 1
"#).unwrap();

    // Main program
    let main_content = format!(r#"
store shared_counter as 10
include from "{}"
"#, module_path.to_string_lossy());

    let tokens = lex_wfl_with_positions(&main_content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse include statement");
    
    let mut interpreter = Interpreter::new();
    
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok());
    
    // The shared_counter should be 11 (modified by included module)
    let env_borrowed = interpreter.global_env().borrow();
    let counter_value = env_borrowed.get("shared_counter").unwrap();
    if let wfl::interpreter::value::Value::Number(n) = counter_value {
        assert_eq!(n as i32, 11, "Include should allow module to modify parent variables");
    } else {
        panic!("Expected number value for shared_counter");
    }
}

#[tokio::test] 
async fn test_include_vs_load_module_behavior_difference() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a module with container
    let module_path = temp_dir.path().join("test_container.wfl");
    fs::write(&module_path, r#"
create container TestContainer:
    property value: Number
end
"#).unwrap();

    // Test load module (should fail to access container)
    let load_content = format!(r#"
load module from "{}"
create new TestContainer as instance:
    value is 42
end
"#, module_path.to_string_lossy());

    let tokens = lex_wfl_with_positions(&load_content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse load module statement");
    
    let mut interpreter = Interpreter::new();
    
    let load_result = interpreter.interpret(&program).await;
    assert!(load_result.is_err(), "Load module should NOT expose container to parent");

    // Test include (should succeed)
    let include_content = format!(r#"
include from "{}"
create new TestContainer as instance:
    value is 42  
end
"#, module_path.to_string_lossy());

    let tokens = lex_wfl_with_positions(&include_content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse include statement");
    
    let mut interpreter = Interpreter::new();
    
    let include_result = interpreter.interpret(&program).await;
    assert!(include_result.is_ok(), "Include should expose container to parent");
}

#[tokio::test]
async fn test_include_statement_parsing() {
    // Test basic include parsing
    let content = r#"include from "module.wfl""#;
    let tokens = lex_wfl_with_positions(content);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse();
    
    assert!(program.is_ok(), "Should parse include statement");
    let program = program.unwrap();
    assert_eq!(program.statements.len(), 1);
    
    match &program.statements[0] {
        Statement::IncludeStatement { path, .. } => {
            match path {
                Expression::Literal(Literal::String(s), _, _) => {
                    assert_eq!(s, "module.wfl");
                }
                _ => panic!("Expected text literal for include path"),
            }
        }
        _ => panic!("Expected IncludeStatement, got {:?}", program.statements[0]),
    }
}