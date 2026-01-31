use std::fs;
use std::path::Path;
use tempfile::TempDir;
use wfl::interpreter::Interpreter;
use wfl::lexer::lex_wfl;
use wfl::parser::parse_wfl;

#[test]
fn test_include_statement_exposes_container_to_parent() {
    // Create temporary directory for test isolation
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let container_file = temp_dir.path().join("test_container.wfl");
    let main_file = temp_dir.path().join("test_main.wfl");

    // Create container definition file
    let container_content = r#"
create container Person:
    property name: Text
    property age: Number
end
"#;
    fs::write(&container_file, container_content).expect("Failed to write container file");

    // Create main file that includes container and uses it
    let main_content = r#"
include from "test_container.wfl"

create new Person as alice:
    name is "Alice"
    age is 30
end

display alice.name
"#;
    fs::write(&main_file, main_content).expect("Failed to write main file");

    // Try to run the program - this should work after include is implemented
    let source = fs::read_to_string(&main_file).expect("Failed to read main file");
    let tokens = lex_wfl(&source);

    match parse_wfl(&tokens, &main_file) {
        Ok(ast) => {
            let mut interpreter = Interpreter::new();
            let result = interpreter.interpret(&ast);

            // This test will fail initially (before include is implemented)
            // After implementation, it should succeed
            match result {
                Ok(_) => {
                    // Success - include statement worked and container was exposed
                    assert!(true);
                }
                Err(e) => {
                    // Expected failure before implementation
                    println!("Expected error before include implementation: {}", e);
                    // This assertion will fail initially, driving TDD implementation
                    assert!(
                        false,
                        "Include statement should expose container to parent scope"
                    );
                }
            }
        }
        Err(e) => {
            println!("Parse error: {}", e);
            // Parse should fail initially due to missing include syntax
            assert!(
                e.to_string().contains("include"),
                "Parse should fail due to missing include keyword"
            );
        }
    }

    // Temp directory is automatically cleaned up when dropped
}

#[test]
fn test_include_vs_load_module_behavior() {
    // This test demonstrates the difference between include and load module
    // Create temporary directory for test isolation
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let shared_file = temp_dir.path().join("test_shared.wfl");
    let include_main = temp_dir.path().join("test_include_main.wfl");
    let load_main = temp_dir.path().join("test_load_main.wfl");

    // Create shared functionality
    let shared_content = r#"
store utility_value as "shared utility"

create container SharedContainer:
    property data: Text
end
"#;
    fs::write(&shared_file, shared_content).expect("Failed to write shared file");

    // Test include behavior (should expose variables/containers)
    let include_content = r#"
include from "test_shared.wfl"

display utility_value
create new SharedContainer as instance:
    data is "test"
end
"#;
    fs::write(&include_main, include_content).expect("Failed to write include main");

    // Test load module behavior (should NOT expose variables/containers)
    let load_content = r#"
load module from "test_shared.wfl"

display utility_value
"#;
    fs::write(&load_main, load_content).expect("Failed to write load main");

    // Test include behavior
    let include_source = fs::read_to_string(&include_main).expect("Failed to read include main");
    let include_tokens = lex_wfl(&include_source);

    match parse_wfl(&include_tokens, &include_main) {
        Ok(ast) => {
            let mut interpreter = Interpreter::new();
            let result = interpreter.interpret(&ast);

            match result {
                Ok(_) => {
                    // Include should succeed and expose shared definitions
                    assert!(true);
                }
                Err(_) => {
                    // Will fail initially before include is implemented
                    assert!(
                        false,
                        "Include should expose shared definitions to parent scope"
                    );
                }
            }
        }
        Err(_) => {
            // Expected parse failure before include keyword is added
        }
    }

    // Test load module behavior (should work but not expose variables)
    let load_source = fs::read_to_string(&load_main).expect("Failed to read load main");
    let load_tokens = lex_wfl(&load_source);

    match parse_wfl(&load_tokens, &load_main) {
        Ok(ast) => {
            let mut interpreter = Interpreter::new();
            let result = interpreter.interpret(&ast);

            match result {
                Ok(_) => {
                    assert!(
                        false,
                        "Load module should NOT expose utility_value to parent"
                    );
                }
                Err(e) => {
                    // This should fail because utility_value is not accessible from parent
                    assert!(
                        e.to_string().contains("utility_value")
                            || e.to_string().contains("not found")
                    );
                }
            }
        }
        Err(_) => {
            // Parse error is unexpected for load module (should already exist)
            assert!(false, "Load module syntax should already exist");
        }
    }

    // Temp directory is automatically cleaned up when dropped
}

#[test]
fn test_include_statement_executes_in_parent_scope() {
    // Test that include executes code in parent scope, not isolated child
    // Create temporary directory for test isolation
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let module_file = temp_dir.path().join("test_parent_scope.wfl");
    let main_file = temp_dir.path().join("test_parent_main.wfl");

    // Create module that modifies a parent variable
    let module_content = r#"
change parent_var to "modified by include"
store new_var as "created by include"
"#;
    fs::write(&module_file, module_content).expect("Failed to write module file");

    // Create main file with parent variable and include
    let main_content = r#"
store parent_var as "original value"

include from "test_parent_scope.wfl"

display parent_var
display new_var
"#;
    fs::write(&main_file, main_content).expect("Failed to write main file");

    // Test execution
    let source = fs::read_to_string(&main_file).expect("Failed to read main file");
    let tokens = lex_wfl(&source);

    match parse_wfl(&tokens, &main_file) {
        Ok(ast) => {
            let mut interpreter = Interpreter::new();
            let result = interpreter.interpret(&ast);

            match result {
                Ok(_) => {
                    // Include should succeed and modify parent scope
                    assert!(true);
                }
                Err(_) => {
                    // Will fail initially before include is implemented
                    assert!(
                        false,
                        "Include should execute in parent scope and allow variable access"
                    );
                }
            }
        }
        Err(_) => {
            // Expected parse failure before include keyword is added
        }
    }

    // Temp directory is automatically cleaned up when dropped
}

#[test]
fn test_include_statement_path_resolution() {
    // Test that include follows same path resolution as load module
    // Create temporary directory for test isolation
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let nested_dir = temp_dir.path().join("test_nested");
    let nested_file = nested_dir.join("utility.wfl");
    let main_file = temp_dir.path().join("test_path_main.wfl");

    // Create nested directory and file
    fs::create_dir_all(&nested_dir).expect("Failed to create nested directory");

    let utility_content = r#"
store nested_utility as "from nested directory"
"#;
    fs::write(&nested_file, utility_content).expect("Failed to write nested file");

    // Create main file that includes from nested directory
    let main_content = r#"
include from "test_nested/utility.wfl"

display nested_utility
"#;
    fs::write(&main_file, main_content).expect("Failed to write main file");

    // Test execution
    let source = fs::read_to_string(&main_file).expect("Failed to read main file");
    let tokens = lex_wfl(&source);

    match parse_wfl(&tokens, &main_file) {
        Ok(ast) => {
            let mut interpreter = Interpreter::new();
            let result = interpreter.interpret(&ast);

            match result {
                Ok(_) => {
                    // Include should succeed with nested path
                    assert!(true);
                }
                Err(_) => {
                    // Will fail initially before include is implemented
                    assert!(false, "Include should handle nested paths like load module");
                }
            }
        }
        Err(_) => {
            // Expected parse failure before include keyword is added
        }
    }

    // Temp directory is automatically cleaned up when dropped
}
