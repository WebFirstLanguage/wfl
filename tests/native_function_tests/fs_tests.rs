use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use std::fs;
use std::io::Write;
use tempfile::tempdir;

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
async fn test_dirlist_function_exists() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let dir_path = temp_dir.path().to_str().unwrap();

    let code = format!(r#"
    store result as dirlist("{}")
    "#, dir_path);

    let result = execute_wfl(&code).await;
    assert!(result.is_ok(), "dirlist() function should be available");
}

#[tokio::test]
async fn test_dirlist_returns_list() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let dir_path = temp_dir.path().to_str().unwrap();

    let code = format!(r#"
    store result as dirlist("{}")
    "#, dir_path);

    let tokens = lex_wfl_with_positions(&code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse program");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Failed to execute program: {:?}", result);

    let env = interpreter.global_env();
    let result_value = env.borrow().get("result").expect("Result not found");

    match result_value {
        Value::List(_) => {},
        _ => panic!("Expected list, got {:?}", result_value),
    }
}

#[tokio::test]
async fn test_dirlist_finds_files() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let dir_path = temp_dir.path();

    let file1_path = dir_path.join("test1.txt");
    let file2_path = dir_path.join("test2.md");
    
    fs::write(&file1_path, "content1").expect("Failed to write test file");
    fs::write(&file2_path, "content2").expect("Failed to write test file");

    let code = format!(r#"
    store result as dirlist("{}")
    store len as length(result)
    "#, dir_path.to_str().unwrap());

    let tokens = lex_wfl_with_positions(&code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse program");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Failed to execute program: {:?}", result);

    let env = interpreter.global_env();
    let len_value = env.borrow().get("len").expect("Length not found");

    match len_value {
        Value::Number(n) => assert_eq!(n, 2.0, "Expected 2 files"),
        _ => panic!("Expected number, got {:?}", len_value),
    }
}

#[tokio::test]
async fn test_dirlist_with_pattern() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let dir_path = temp_dir.path();

    let file1_path = dir_path.join("test1.txt");
    let file2_path = dir_path.join("test2.md");
    
    fs::write(&file1_path, "content1").expect("Failed to write test file");
    fs::write(&file2_path, "content2").expect("Failed to write test file");

    let code = format!(r#"
    store result as dirlist("{}", false, "*.txt")
    store len as length(result)
    "#, dir_path.to_str().unwrap());

    let tokens = lex_wfl_with_positions(&code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse program");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Failed to execute program: {:?}", result);

    let env = interpreter.global_env();
    let len_value = env.borrow().get("len").expect("Length not found");

    match len_value {
        Value::Number(n) => assert_eq!(n, 1.0, "Expected 1 .txt file"),
        _ => panic!("Expected number, got {:?}", len_value),
    }
}

#[tokio::test]
async fn test_filemtime_function_exists() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "content").expect("Failed to write test file");

    let code = format!(r#"
    store result as filemtime("{}")
    "#, file_path.to_str().unwrap());

    let result = execute_wfl(&code).await;
    assert!(result.is_ok(), "filemtime() function should be available");
}

#[tokio::test]
async fn test_filemtime_returns_number() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "content").expect("Failed to write test file");

    let code = format!(r#"
    store result as filemtime("{}")
    "#, file_path.to_str().unwrap());

    let tokens = lex_wfl_with_positions(&code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse program");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Failed to execute program: {:?}", result);

    let env = interpreter.global_env();
    let result_value = env.borrow().get("result").expect("Result not found");

    match result_value {
        Value::Number(n) => assert!(n > 0.0, "Expected positive timestamp"),
        _ => panic!("Expected number, got {:?}", result_value),
    }
}

#[tokio::test]
async fn test_filemtime_nonexistent_file() {
    let code = r#"
    store result as filemtime("/nonexistent/file.txt")
    "#;

    let result = execute_wfl(code).await;
    assert!(result.is_err(), "filemtime() should fail for nonexistent file");
}

#[tokio::test]
async fn test_dirlist_nonexistent_directory() {
    let code = r#"
    store result as dirlist("/nonexistent/directory")
    "#;

    let result = execute_wfl(code).await;
    assert!(result.is_err(), "dirlist() should fail for nonexistent directory");
}
