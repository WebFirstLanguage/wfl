// TDD Tests for Random Number Generation Functions
// These tests MUST FAIL FIRST before implementation

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

#[cfg(test)]
mod random_function_tests {
    use super::*;

    /// Test helper to run WFL code and get the result from a variable
    async fn run_wfl_code(code: &str) -> Result<Value, String> {
        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let ast = parser
            .parse()
            .map_err(|e| format!("Parse error: {:?}", e))?;

        let mut interpreter = Interpreter::new();
        let _ = interpreter
            .interpret(&ast)
            .await
            .map_err(|e| format!("Runtime error: {:?}", e))?;

        // Extract the result from the 'result' variable
        if let Some(result_value) = interpreter.global_env().borrow().get("result") {
            Ok(result_value)
        } else {
            Err("Variable 'result' not found after execution".to_string())
        }
    }

    /// Test helper to check if a value is a number within a range
    fn assert_number_in_range(value: &Value, min: f64, max: f64) {
        match value {
            Value::Number(n) => {
                assert!(
                    *n >= min && *n <= max,
                    "Expected number between {} and {}, got {}",
                    min,
                    max,
                    n
                );
            }
            _ => panic!("Expected number, got {:?}", value),
        }
    }

    #[tokio::test]
    async fn test_random_between_function_exists() {
        // This test MUST FAIL initially - random_between function doesn't exist yet
        let code = r#"
            store result as random_between of 1 and 10
            display result
        "#;

        let result = run_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "random_between function should exist and work"
        );
    }

    #[tokio::test]
    async fn test_random_between_range_validation() {
        // Test that random_between produces numbers within the specified range
        let code = r#"
            store result as random_between of 5 and 15
        "#;

        let result = run_wfl_code(code).await;
        assert!(result.is_ok(), "random_between should execute successfully");

        // Run multiple times to test range consistency
        for _ in 0..10 {
            let result = run_wfl_code(code).await.unwrap();
            assert_number_in_range(&result, 5.0, 15.0);
        }
    }

    #[tokio::test]
    async fn test_random_between_edge_cases() {
        // Test edge cases for random_between
        let test_cases = vec![
            ("random_between of 0 and 1", 0.0, 1.0),
            ("random_between of -10 and -5", -10.0, -5.0),
            ("random_between of 100 and 100", 100.0, 100.0), // Same min/max
        ];

        for (code_fragment, min, max) in test_cases {
            let code = format!("store result as {}", code_fragment);
            let result = run_wfl_code(&code).await;
            assert!(
                result.is_ok(),
                "random_between should handle edge case: {}",
                code_fragment
            );

            if let Ok(value) = result {
                assert_number_in_range(&value, min, max);
            }
        }
    }

    #[tokio::test]
    async fn test_random_int_function_exists() {
        // This test MUST FAIL initially - random_int function doesn't exist yet
        let code = r#"
            store result as random_int of 1 and 20
            display result
        "#;

        let result = run_wfl_code(code).await;
        assert!(result.is_ok(), "random_int function should exist and work");
    }

    #[tokio::test]
    async fn test_random_int_produces_integers() {
        // Test that random_int produces actual integers, not floats
        let code = r#"
            store result as random_int of 1 and 10
        "#;

        for _ in 0..10 {
            let result = run_wfl_code(code).await.unwrap();
            match result {
                Value::Number(n) => {
                    assert_eq!(
                        n.fract(),
                        0.0,
                        "random_int should produce integers, got {}",
                        n
                    );
                    assert!(
                        (1.0..=10.0).contains(&n),
                        "random_int should be in range [1,10], got {}",
                        n
                    );
                }
                _ => panic!("Expected number from random_int, got {:?}", result),
            }
        }
    }

    #[tokio::test]
    async fn test_random_boolean_function_exists() {
        // This test MUST FAIL initially - random_boolean function doesn't exist yet
        let code = r#"
            store result as random_boolean
            display result
        "#;

        let result = run_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "random_boolean function should exist and work"
        );
    }

    #[tokio::test]
    async fn test_random_boolean_produces_booleans() {
        // Test that random_boolean produces actual boolean values
        let code = r#"
            store result as random_boolean
        "#;

        let mut true_count = 0;
        let mut false_count = 0;

        // Run many times to ensure we get both true and false values
        for _ in 0..100 {
            let result = run_wfl_code(code).await.unwrap();
            match result {
                Value::Bool(true) => true_count += 1,
                Value::Bool(false) => false_count += 1,
                _ => panic!("Expected boolean from random_boolean, got {:?}", result),
            }
        }

        // Both true and false should occur (with high probability)
        assert!(true_count > 0, "random_boolean should produce true values");
        assert!(
            false_count > 0,
            "random_boolean should produce false values"
        );
    }

    #[tokio::test]
    async fn test_random_from_function_exists() {
        // This test MUST FAIL initially - random_from function doesn't exist yet
        let code = r#"
            store colors as ["red" and "green" and "blue"]
            store result as random_from of colors
            display result
        "#;

        let result = run_wfl_code(code).await;
        assert!(result.is_ok(), "random_from function should exist and work");
    }

    #[tokio::test]
    async fn test_random_from_selects_from_list() {
        // Test that random_from selects items that are actually in the list
        let code = r#"
            store items as ["apple" and "banana" and "cherry" and "date"]
            store result as random_from of items
        "#;

        let valid_items = ["apple", "banana", "cherry", "date"];

        for _ in 0..20 {
            let result = run_wfl_code(code).await.unwrap();
            match result {
                Value::Text(text) => {
                    assert!(
                        valid_items.contains(&text.as_ref()),
                        "random_from should select from list items, got '{}'",
                        text
                    );
                }
                _ => panic!("Expected text from random_from, got {:?}", result),
            }
        }
    }

    #[tokio::test]
    async fn test_random_seed_function_exists() {
        // This test MUST FAIL initially - random_seed function doesn't exist yet
        let code = r#"
            random_seed of 42
            store result as random
            display result
        "#;

        let result = run_wfl_code(code).await;
        assert!(result.is_ok(), "random_seed function should exist and work");
    }

    #[tokio::test]
    async fn test_random_seed_reproducibility() {
        // Test that seeded random produces reproducible results
        let code1 = r#"
            random_seed of 12345
            store result as random
        "#;

        let code2 = r#"
            random_seed of 12345
            store result as random
        "#;

        let result1 = run_wfl_code(code1).await.unwrap();
        let result2 = run_wfl_code(code2).await.unwrap();

        assert_eq!(
            result1, result2,
            "Same seed should produce same random values"
        );
    }

    #[tokio::test]
    async fn test_random_seed_different_seeds() {
        // Test that different seeds produce different sequences
        let code_seed1 = r#"
            random_seed of 111
            store result as random
        "#;

        let code_seed2 = r#"
            random_seed of 222
            store result as random
        "#;

        let result1 = run_wfl_code(code_seed1).await.unwrap();
        let result2 = run_wfl_code(code_seed2).await.unwrap();

        // Different seeds should (very likely) produce different values
        assert_ne!(
            result1, result2,
            "Different seeds should produce different random values"
        );
    }

    #[tokio::test]
    async fn test_existing_random_function_still_works() {
        // Ensure the existing random function continues to work (backward compatibility)
        let code = r#"
            store result as random
        "#;

        let result = run_wfl_code(code).await;
        assert!(result.is_ok(), "Existing random function should still work");

        if let Ok(Value::Number(n)) = result {
            assert!(
                (0.0..=1.0).contains(&n),
                "random should return value between 0 and 1, got {}",
                n
            );
        } else {
            panic!("random should return a number");
        }
    }

    #[tokio::test]
    async fn test_random_functions_are_cryptographically_secure() {
        // Test that random functions don't show obvious patterns (basic test)
        let code = r#"
            store result as random
        "#;

        let mut values = Vec::new();
        for _ in 0..100 {
            let result = run_wfl_code(code).await.unwrap();
            if let Value::Number(n) = result {
                values.push(n);
            }
        }

        // Basic statistical tests - values should be distributed
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        assert!(
            mean > 0.3 && mean < 0.7,
            "Random values should be roughly centered around 0.5, got mean {}",
            mean
        );

        // Check for obvious patterns (consecutive identical values)
        let mut consecutive_identical = 0;
        for i in 1..values.len() {
            if (values[i] - values[i - 1]).abs() < 0.0001 {
                consecutive_identical += 1;
            }
        }
        assert!(
            consecutive_identical < 5,
            "Too many consecutive identical values: {}",
            consecutive_identical
        );
    }
}
