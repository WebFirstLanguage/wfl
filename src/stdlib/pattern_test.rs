// Tests for the pattern standard library module
//
// Note: Legacy IR parsing tests have been removed in favor of the new bytecode VM pattern system.
// The new pattern system is tested through integration tests in TestPrograms/pattern_*.wfl

#[cfg(test)]
mod tests {
    use crate::interpreter::value::Value;
    use crate::stdlib::pattern::{
        pattern_find_all_native, pattern_find_native, pattern_matches_native,
    };

    // These are basic tests for the native function signatures
    // Full functionality is tested through WFL integration tests

    #[test]
    fn test_pattern_matches_native_wrong_arg_count() {
        let result = pattern_matches_native(vec![]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expects 2 arguments")
        );
    }

    #[test]
    fn test_pattern_find_native_wrong_arg_count() {
        let result = pattern_find_native(vec![]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expects 2 arguments")
        );
    }

    #[test]
    fn test_pattern_find_all_native_wrong_arg_count() {
        let result = pattern_find_all_native(vec![]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expects 2 arguments")
        );
    }

    #[test]
    fn test_pattern_matches_native_wrong_first_arg_type() {
        let args = vec![
            Value::Number(42.0),
            Value::Null, // This will also be wrong but we test first arg first
        ];
        let result = pattern_matches_native(args);
        assert!(result.is_err());
        // Updated error message from expect_text helper
        assert!(result.unwrap_err().to_string().contains("Expected text"));
    }
}
