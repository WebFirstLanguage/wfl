use tempfile::NamedTempFile;
use wfl::interpreter::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use std::io::Write;

/// Test that demonstrates the expected idempotent behavior of file closing.
/// Closing an already-closed file handle should not cause an error.
/// This is a safety feature that makes file handling more robust.
#[tokio::test]
async fn test_double_close_file_is_noop() {
    let temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file.as_file(), "test content").unwrap();
    let file_path = temp_file.path().to_str().unwrap().to_string();
    
    let mut interpreter = Interpreter::new();
    
    let source = format!(
        r#"
        open file at "{}" as file_handle
        close file file_handle
        close file file_handle
        "#,
        file_path
    );
    
    let tokens = lex_wfl_with_positions(&source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Program should parse successfully");
    
    // This should succeed - double closing should be a safe no-op
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Double close should not cause an error - it should be idempotent");
}

/// Test that demonstrates error handling when trying to use a closed file handle
/// for operations other than closing (like reading or writing).
#[tokio::test]
async fn test_operations_on_closed_file_handle_error() {
    let temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file.as_file(), "test content").unwrap();
    let file_path = temp_file.path().to_str().unwrap().to_string();
    
    let mut interpreter = Interpreter::new();
    
    let source = format!(
        r#"
        open file at "{}" as file_handle
        close file file_handle
        wait for store file_content as read content from file_handle
        "#,
        file_path
    );
    
    let tokens = lex_wfl_with_positions(&source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Program should parse successfully");
    
    // This should fail because we're trying to read from a closed handle
    let result = interpreter.interpret(&program).await;
    assert!(result.is_err(), "Reading from a closed file handle should cause an error");
}