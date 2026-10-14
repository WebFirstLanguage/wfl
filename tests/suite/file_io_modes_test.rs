use std::fs;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

// Test file I/O opening modes functionality that should work but currently fails
#[cfg(test)]
mod file_io_modes_tests {
    use super::*;

    fn cleanup_test_file(path: &str) {
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_open_file_for_reading_mode() {
        let code = r#"
            open file at "test_read.txt" for reading as test_file
            close file test_file
        "#;

        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let result = parser.parse();

        assert!(
            result.is_ok(),
            "Failed to parse file opening for reading: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_open_file_for_writing_mode() {
        let code = r#"
            open file at "test_write.txt" for writing as test_file
            wait for write content "test data" into test_file
            close file test_file
        "#;

        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let result = parser.parse();

        assert!(
            result.is_ok(),
            "Failed to parse file opening for writing: {:?}",
            result.err()
        );

        cleanup_test_file("test_write.txt");
    }

    #[test]
    fn test_list_files_with_pattern() {
        let code = r#"
            wait for store wfl_files as list files in "." with pattern "*.wfl"
            display "Found " with length of wfl_files with " WFL files"
        "#;

        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let result = parser.parse();

        assert!(
            result.is_ok(),
            "Failed to parse file listing with pattern: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_list_files_recursively() {
        let code = r#"
            wait for store all_files as list files recursively in "."
            display "Found " with length of all_files with " files total"
        "#;

        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let result = parser.parse();

        assert!(
            result.is_ok(),
            "Failed to parse recursive file listing: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_file_exists_operation() {
        let code = r#"
            store file_exists as file exists at "existing_file.txt"
            check if file_exists:
                display "File exists"
            end check
            
            store missing_exists as file exists at "nonexistent.txt"
            check if not missing_exists:
                display "Missing file correctly detected as non-existent"
            end check
        "#;

        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let result = parser.parse();

        assert!(
            result.is_ok(),
            "Failed to parse file exists operation: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_file_size_operation() {
        let code = r#"
            wait for store file_size as size of file at "size_test.txt"
            display "File size: " with file_size with " bytes"
        "#;

        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let result = parser.parse();

        assert!(
            result.is_ok(),
            "Failed to parse file size operation: {:?}",
            result.err()
        );
    }
}
