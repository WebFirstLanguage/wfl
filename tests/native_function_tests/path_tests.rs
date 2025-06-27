use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

async fn execute_wfl(code: &str) -> Result<Value, String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .map_err(|e| format!("Parse error: {:?}", e))?;

    let mut interpreter = Interpreter::default();
    interpreter
        .interpret(&program)
        .await
        .map_err(|e| format!("Runtime error: {:?}", e))
}

#[tokio::test]
async fn test_basename_function_exists() {
    let code = r#"
    store result as basename("/path/to/file.txt")
    "#;

    let result = execute_wfl(code).await;
    assert!(result.is_ok(), "basename() function should be available");
}

#[tokio::test]
async fn test_basename_extracts_filename() {
    let code = r#"
    store result as basename("/path/to/file.txt")
    "#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse program");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Failed to execute program: {:?}", result);

    let env = interpreter.global_env();
    let result_value = env.borrow().get("result").expect("Result not found");

    match result_value {
        Value::Text(s) => assert_eq!(s.as_ref(), "file.txt"),
        _ => panic!("Expected text, got {:?}", result_value),
    }
}

#[tokio::test]
async fn test_basename_with_no_directory() {
    let code = r#"
    store result as basename("file.txt")
    "#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse program");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Failed to execute program: {:?}", result);

    let env = interpreter.global_env();
    let result_value = env.borrow().get("result").expect("Result not found");

    match result_value {
        Value::Text(s) => assert_eq!(s.as_ref(), "file.txt"),
        _ => panic!("Expected text, got {:?}", result_value),
    }
}

#[tokio::test]
async fn test_dirname_function_exists() {
    let code = r#"
    store result as dirname("/path/to/file.txt")
    "#;

    let result = execute_wfl(code).await;
    assert!(result.is_ok(), "dirname() function should be available");
}

#[tokio::test]
async fn test_dirname_extracts_directory() {
    let code = r#"
    store result as dirname("/path/to/file.txt")
    "#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse program");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Failed to execute program: {:?}", result);

    let env = interpreter.global_env();
    let result_value = env.borrow().get("result").expect("Result not found");

    match result_value {
        Value::Text(s) => assert_eq!(s.as_ref(), "/path/to"),
        _ => panic!("Expected text, got {:?}", result_value),
    }
}

#[tokio::test]
async fn test_dirname_with_no_directory() {
    let code = r#"
    store result as dirname("file.txt")
    "#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse program");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Failed to execute program: {:?}", result);

    let env = interpreter.global_env();
    let result_value = env.borrow().get("result").expect("Result not found");

    match result_value {
        Value::Text(s) => assert_eq!(s.as_ref(), "."),
        _ => panic!("Expected text, got {:?}", result_value),
    }
}

#[tokio::test]
async fn test_pathjoin_function_exists() {
    let code = r#"
    store result as pathjoin("/path", "to", "file.txt")
    "#;

    let result = execute_wfl(code).await;
    assert!(result.is_ok(), "pathjoin() function should be available");
}

#[tokio::test]
async fn test_pathjoin_combines_paths() {
    let code = r#"
    store result as pathjoin("/path", "to", "file.txt")
    "#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse program");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Failed to execute program: {:?}", result);

    let env = interpreter.global_env();
    let result_value = env.borrow().get("result").expect("Result not found");

    match result_value {
        Value::Text(s) => {
            let path_str = s.as_ref();
            assert!(path_str.contains("path"));
            assert!(path_str.contains("to"));
            assert!(path_str.contains("file.txt"));
        },
        _ => panic!("Expected text, got {:?}", result_value),
    }
}

#[cfg(unix)]
#[tokio::test]
async fn test_pathjoin_unix_separators() {
    let code = r#"
    store result as pathjoin("/path", "to", "file.txt")
    "#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse program");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Failed to execute program: {:?}", result);

    let env = interpreter.global_env();
    let result_value = env.borrow().get("result").expect("Result not found");

    match result_value {
        Value::Text(s) => assert_eq!(s.as_ref(), "/path/to/file.txt"),
        _ => panic!("Expected text, got {:?}", result_value),
    }
}

#[cfg(windows)]
#[tokio::test]
async fn test_pathjoin_windows_separators() {
    let code = r#"
    store result as pathjoin("C:\\path", "to", "file.txt")
    "#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse program");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Failed to execute program: {:?}", result);

    let env = interpreter.global_env();
    let result_value = env.borrow().get("result").expect("Result not found");

    match result_value {
        Value::Text(s) => assert_eq!(s.as_ref(), "C:\\path\\to\\file.txt"),
        _ => panic!("Expected text, got {:?}", result_value),
    }
}

#[tokio::test]
async fn test_pathjoin_insufficient_arguments() {
    let code = r#"
    store result as pathjoin("/path")
    "#;

    let result = execute_wfl(code).await;
    assert!(result.is_err(), "pathjoin() should require at least 2 arguments");
}

#[tokio::test]
async fn test_basename_wrong_argument_count() {
    let code = r#"
    store result as basename()
    "#;

    let result = execute_wfl(code).await;
    assert!(result.is_err(), "basename() should require exactly 1 argument");
}

#[tokio::test]
async fn test_dirname_wrong_argument_count() {
    let code = r#"
    store result as dirname()
    "#;

    let result = execute_wfl(code).await;
    assert!(result.is_err(), "dirname() should require exactly 1 argument");
}
