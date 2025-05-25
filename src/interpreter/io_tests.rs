use super::{Environment, Interpreter, Value};
use crate::lexer::lex_wfl_with_positions;
use crate::parser::Parser;
use std::io::Write;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_file_io() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "Hello, WFL!").unwrap();
    let file_path = temp_file.path().to_str().unwrap().to_string();
    
    let mut interpreter = Interpreter::new();
    
    let source = format!(
        r#"
        wait for open file at "{}" and read content as content
        display content
        "#,
        file_path
    );
    
    let tokens = lex_wfl_with_positions(&source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_file_write_read_round_trip() {
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_str().unwrap().to_string();
    
    let mut interpreter = Interpreter::new();
    
    let source = format!(
        r#"
        wait for open file at "{}" and store handle as file_handle
        wait for write "Hello, WFL!" to file_handle
        wait for read from file_handle and store content as content
        display content
        wait for close file_handle
        "#,
        file_path
    );
    
    let tokens = lex_wfl_with_positions(&source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_close_file_then_fail_on_read() {
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_str().unwrap().to_string();
    
    let mut interpreter = Interpreter::new();
    
    let source = format!(
        r#"
        wait for open file at "{}" and store handle as file_handle
        wait for write "Test data" to file_handle
        wait for close file_handle
        
        wait for read from file_handle and store content as content
        "#,
        file_path
    );
    
    let tokens = lex_wfl_with_positions(&source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    
    let result = interpreter.interpret(&program).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_http_get() {
    let mut interpreter = Interpreter::new();
    
    let source = r#"
    wait for open url at "https://httpbin.org/status/200" and read content as response
    display response
    "#;
    
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_http_post() {
    let mut interpreter = Interpreter::new();
    
    let source = r#"
    wait for open url at "https://httpbin.org/post" with data "name=alice" and read content as response
    display response
    "#;
    
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_try_when_statement() {
    let mut interpreter = Interpreter::new();
    
    let source = r#"
    try:
        open url at "https://non-existent-url.example.com" and read content as response
        display response
    when error:
        display "Error handled: " with error
    end try
    "#;
    
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Try/when statement should handle the error gracefully");
}

#[tokio::test]
async fn test_try_when_error_propagation() {
    let mut interpreter = Interpreter::new();
    
    let source = r#"
    try:
        open url at "https://non-existent-url.example.com" and read content as response
        display response
    when error:
        display error_that_doesnt_exist
    end try
    "#;
    
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    
    let result = interpreter.interpret(&program).await;
    assert!(result.is_err(), "Error in when block should propagate upward");
}

#[tokio::test]
async fn test_file_handle_cleanup() {
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_str().unwrap().to_string();
    
    let mut interpreter = Interpreter::new();
    
    let source = format!(
        r#"
        wait for open file at "{}" and store handle as file1
        wait for write "First write" to file1
        wait for close file1
        
        wait for open file at "{}" and read content into content1
        display content1
        
        wait for open file at "{}" and store handle as file2
        wait for write "Second write" to file2
        wait for close file2
        
        wait for open file at "{}" and read content into content2
        display content2
        "#,
        file_path, file_path, file_path, file_path
    );
    
    let tokens = lex_wfl_with_positions(&source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Multiple operations on the same file should succeed");
}
