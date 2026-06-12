// TDD tests for web route parameter helpers: path_params and path_matches
// These tests are written before the implementation (stdlib/web.rs).

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

#[cfg(test)]
mod route_param_tests {
    use super::*;

    /// Run WFL code and return the value of the `result` variable.
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

        if let Some(result_value) = interpreter.global_env().borrow().get("result") {
            Ok(result_value)
        } else {
            Err("Variable 'result' not found after execution".to_string())
        }
    }

    fn expect_object_key(value: &Value, key: &str) -> Value {
        match value {
            Value::Object(obj) => obj
                .borrow()
                .get(key)
                .cloned()
                .unwrap_or_else(|| panic!("Object missing key '{key}'")),
            other => panic!("Expected object, got {other:?}"),
        }
    }

    fn expect_text(value: &Value) -> String {
        match value {
            Value::Text(t) => t.to_string(),
            other => panic!("Expected text, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_path_params_single_capture() {
        let code = r#"
            store result as path_params of "/users/42" and "/users/:id"
        "#;
        let result = run_wfl_code(code).await.expect("path_params should run");
        let id = expect_object_key(&result, "id");
        assert_eq!(expect_text(&id), "42");
    }

    #[tokio::test]
    async fn test_path_params_multiple_captures() {
        let code = r#"
            store result as path_params of "/users/42/posts/7" and "/users/:user_id/posts/:post_id"
        "#;
        let result = run_wfl_code(code).await.expect("path_params should run");
        assert_eq!(expect_text(&expect_object_key(&result, "user_id")), "42");
        assert_eq!(expect_text(&expect_object_key(&result, "post_id")), "7");
    }

    #[tokio::test]
    async fn test_path_params_no_match_returns_nothing() {
        let code = r#"
            store result as path_params of "/posts/42" and "/users/:id"
        "#;
        let result = run_wfl_code(code).await.expect("path_params should run");
        assert!(
            matches!(result, Value::Nothing),
            "Non-matching path should return nothing, got {result:?}"
        );
    }

    #[tokio::test]
    async fn test_path_params_segment_count_mismatch() {
        // Too few segments
        let code = r#"
            store result as path_params of "/users" and "/users/:id"
        "#;
        let result = run_wfl_code(code).await.expect("path_params should run");
        assert!(matches!(result, Value::Nothing));

        // Too many segments
        let code = r#"
            store result as path_params of "/users/1/extra" and "/users/:id"
        "#;
        let result = run_wfl_code(code).await.expect("path_params should run");
        assert!(matches!(result, Value::Nothing));
    }

    #[tokio::test]
    async fn test_path_params_exact_match_returns_empty_object() {
        let code = r#"
            store result as path_params of "/about" and "/about"
        "#;
        let result = run_wfl_code(code).await.expect("path_params should run");
        match &result {
            Value::Object(obj) => assert!(obj.borrow().is_empty()),
            other => panic!("Exact match should return empty object, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_path_params_trailing_slash_tolerated() {
        let code = r#"
            store result as path_params of "/users/42/" and "/users/:id"
        "#;
        let result = run_wfl_code(code).await.expect("path_params should run");
        assert_eq!(expect_text(&expect_object_key(&result, "id")), "42");
    }

    #[tokio::test]
    async fn test_path_params_strips_query_string() {
        let code = r#"
            store result as path_params of "/users/42?page=2&limit=10" and "/users/:id"
        "#;
        let result = run_wfl_code(code).await.expect("path_params should run");
        assert_eq!(expect_text(&expect_object_key(&result, "id")), "42");
    }

    #[tokio::test]
    async fn test_path_params_percent_decodes_captures() {
        let code = r#"
            store result as path_params of "/users/John%20Doe" and "/users/:name"
        "#;
        let result = run_wfl_code(code).await.expect("path_params should run");
        assert_eq!(expect_text(&expect_object_key(&result, "name")), "John Doe");
    }

    #[tokio::test]
    async fn test_path_params_wildcard_tail() {
        let code = r#"
            store result as path_params of "/static/css/main.css" and "/static/*filepath"
        "#;
        let result = run_wfl_code(code).await.expect("path_params should run");
        assert_eq!(
            expect_text(&expect_object_key(&result, "filepath")),
            "css/main.css"
        );
    }

    #[tokio::test]
    async fn test_path_params_wildcard_requires_at_least_one_segment() {
        let code = r#"
            store result as path_params of "/static" and "/static/*filepath"
        "#;
        let result = run_wfl_code(code).await.expect("path_params should run");
        assert!(matches!(result, Value::Nothing));
    }

    #[tokio::test]
    async fn test_path_params_literal_segments_must_match() {
        let code = r#"
            store result as path_params of "/users/42/comments/7" and "/users/:user_id/posts/:post_id"
        "#;
        let result = run_wfl_code(code).await.expect("path_params should run");
        assert!(matches!(result, Value::Nothing));
    }

    #[tokio::test]
    async fn test_path_matches_true() {
        let code = r#"
            store result as path_matches of "/users/42" and "/users/:id"
        "#;
        let result = run_wfl_code(code).await.expect("path_matches should run");
        assert!(matches!(result, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_path_matches_false() {
        let code = r#"
            store result as path_matches of "/posts/42" and "/users/:id"
        "#;
        let result = run_wfl_code(code).await.expect("path_matches should run");
        assert!(matches!(result, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_path_params_usable_in_check_if_nothing() {
        // End-to-end routing flow as documented for web servers
        let code = r#"
            store request_path as "/users/42"
            store params as path_params of request_path and "/users/:id"
            check if params is nothing:
                store result as "not found"
            otherwise:
                store result as params["id"]
            end check
        "#;
        let result = run_wfl_code(code).await.expect("routing flow should run");
        assert_eq!(expect_text(&result), "42");
    }

    #[tokio::test]
    async fn test_path_params_wrong_arg_type_errors() {
        let code = r#"
            store result as path_params of 42 and "/users/:id"
        "#;
        let result = run_wfl_code(code).await;
        assert!(result.is_err(), "Non-text path should be a runtime error");
    }
}
