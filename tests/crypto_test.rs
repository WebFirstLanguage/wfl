// TDD Tests for WFLHASH Cryptographic Hash Functions
// These tests MUST FAIL FIRST before implementation

use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::interpreter::Interpreter;
use tokio;

#[cfg(test)]
mod crypto_function_tests {
    use super::*;

    /// Test helper to run WFL code and get the result from a variable
    async fn run_wfl_code(code: &str) -> Result<Value, String> {
        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let ast = parser.parse().map_err(|e| format!("Parse error: {:?}", e))?;

        let mut interpreter = Interpreter::new();
        let _ = interpreter.interpret(&ast).await.map_err(|e| format!("Runtime error: {:?}", e))?;

        // Extract the result from the 'result' variable
        if let Some(result_value) = interpreter.global_env().borrow().get("result") {
            Ok(result_value)
        } else {
            Err("Variable 'result' not found after execution".to_string())
        }
    }

    #[tokio::test]
    async fn test_wflhash256_basic() {
        // Test basic WFLHASH-256 functionality
        let code = r#"
            store result as wflhash256 of "hello world"
        "#;

        let result = run_wfl_code(code).await;
        assert!(result.is_ok(), "wflhash256 function should be available and work");

        if let Ok(Value::Text(hash)) = result {
            // WFLHASH-256 should produce a 64-character hex string (256 bits = 32 bytes = 64 hex chars)
            assert_eq!(hash.len(), 64, "WFLHASH-256 should produce 64-character hex string");
            assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "Hash should be valid hexadecimal");
        } else {
            panic!("wflhash256 should return a text value");
        }
    }

    #[tokio::test]
    async fn test_wflhash256_empty_input() {
        let code = r#"
            store result as wflhash256 of ""
        "#;

        let result = run_wfl_code(code).await;
        assert!(result.is_ok(), "wflhash256 should handle empty input");

        if let Ok(Value::Text(hash)) = result {
            assert_eq!(hash.len(), 64, "Empty input should still produce 64-character hash");
            assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "Hash should be valid hexadecimal");
        }
    }

    #[tokio::test]
    async fn test_wflhash256_deterministic() {
        // Test that same input produces same output
        let code1 = r#"
            store result as wflhash256 of "hello world"
        "#;
        let code2 = r#"
            store result as wflhash256 of "hello world"
        "#;

        let result1 = run_wfl_code(code1).await;
        let result2 = run_wfl_code(code2).await;

        assert!(result1.is_ok() && result2.is_ok(), "Both hash operations should succeed");
        assert_eq!(result1.unwrap(), result2.unwrap(), "Same input should produce same hash");
    }

    #[tokio::test]
    async fn test_wflhash256_different_inputs() {
        let code1 = r#"
            store result as wflhash256 of "hello"
        "#;
        let code2 = r#"
            store result as wflhash256 of "world"
        "#;

        let result1 = run_wfl_code(code1).await;
        let result2 = run_wfl_code(code2).await;

        assert!(result1.is_ok() && result2.is_ok(), "Both hash operations should succeed");
        assert_ne!(result1.unwrap(), result2.unwrap(), "Different inputs should produce different hashes");
    }

    #[tokio::test]
    async fn test_wflhash256_avalanche_effect() {
        // Test that small changes in input produce large changes in output
        let code1 = r#"
            store result as wflhash256 of "hello world"
        "#;
        let code2 = r#"
            store result as wflhash256 of "hello worlD"
        "#;

        let result1 = run_wfl_code(code1).await;
        let result2 = run_wfl_code(code2).await;

        assert!(result1.is_ok() && result2.is_ok(), "Both hash operations should succeed");

        if let (Ok(Value::Text(hash1)), Ok(Value::Text(hash2))) = (result1, result2) {
            assert_ne!(hash1, hash2, "Single character change should produce different hash");

            // Count different characters (should be roughly 50% for good avalanche effect)
            let diff_count = hash1.chars()
                .zip(hash2.chars())
                .filter(|(c1, c2)| c1 != c2)
                .count();

            // For a good hash function, at least 25% of bits should change
            assert!(diff_count >= 16, "Avalanche effect: at least 16 characters should differ, got {}", diff_count);
        }
    }

    #[tokio::test]
    async fn test_wflhash512_basic() {
        // Test WFLHASH-512 variant
        let code = r#"
            store result as wflhash512 of "hello world"
        "#;

        let result = run_wfl_code(code).await;
        assert!(result.is_ok(), "wflhash512 function should be available and work");

        if let Ok(Value::Text(hash)) = result {
            // WFLHASH-512 should produce a 128-character hex string (512 bits = 64 bytes = 128 hex chars)
            assert_eq!(hash.len(), 128, "WFLHASH-512 should produce 128-character hex string");
            assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "Hash should be valid hexadecimal");
        } else {
            panic!("wflhash512 should return a text value");
        }
    }

    #[tokio::test]
    async fn test_wflhash_variants_different_outputs() {
        // Ensure different variants produce different outputs for same input
        let code256 = r#"
            store result as wflhash256 of "test"
        "#;
        let code512 = r#"
            store result as wflhash512 of "test"
        "#;

        let result256 = run_wfl_code(code256).await;
        let result512 = run_wfl_code(code512).await;

        assert!(result256.is_ok() && result512.is_ok(), "Both hash operations should succeed");

        if let (Ok(Value::Text(h256)), Ok(Value::Text(h512))) = (result256, result512) {
            assert_ne!(h256.as_ref(), &h512[..64], "Different variants should produce different hashes");
        }
    }
}
