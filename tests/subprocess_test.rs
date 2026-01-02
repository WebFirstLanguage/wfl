// Integration tests for subprocess support in WFL
// Tests execute, spawn, process control, and output streaming

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

#[cfg(test)]
mod subprocess_tests {
    use super::*;

    /// Test helper to run WFL code
    async fn run_wfl_code(code: &str) -> Result<(), String> {
        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let ast = parser
            .parse()
            .map_err(|e| format!("Parse error: {:?}", e))?;

        let mut interpreter = Interpreter::new();
        interpreter
            .interpret(&ast)
            .await
            .map_err(|e| format!("Runtime error: {:?}", e))?;

        Ok(())
    }

    /// Test helper to run WFL code and get a variable value
    async fn run_wfl_code_and_get_var(code: &str, var_name: &str) -> Result<Value, String> {
        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let ast = parser
            .parse()
            .map_err(|e| format!("Parse error: {:?}", e))?;

        let mut interpreter = Interpreter::new();
        interpreter
            .interpret(&ast)
            .await
            .map_err(|e| format!("Runtime error: {:?}", e))?;

        interpreter
            .global_env()
            .borrow()
            .get(var_name)
            .ok_or_else(|| format!("Variable '{}' not found", var_name))
    }

    #[tokio::test]
    async fn test_execute_simple_command() {
        #[cfg(windows)]
        let code = r#"
            wait for execute command "cmd /c echo Hello World" as result
            display "Command executed"
        "#;
        #[cfg(not(windows))]
        let code = r#"
            wait for execute command "echo Hello World" as result
            display "Command executed"
        "#;

        let result = run_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "Basic command execution should work: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_execute_command_stores_result() {
        #[cfg(windows)]
        let code = r#"
            wait for execute command "cmd /c echo test" as result
        "#;
        #[cfg(not(windows))]
        let code = r#"
            wait for execute command "echo test" as result
        "#;

        let result = run_wfl_code_and_get_var(code, "result").await;
        assert!(result.is_ok(), "Should store command result: {:?}", result);

        // Verify it's an Object
        if let Ok(Value::Object(_)) = result {
            // Success - result is an Object
        } else {
            panic!("Result should be an Object, got: {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_execute_command_completes() {
        #[cfg(windows)]
        let code = r#"
            wait for execute command "cmd /c echo test" as result
            display "Execution completed"
        "#;
        #[cfg(not(windows))]
        let code = r#"
            wait for execute command "echo test" as result
            display "Execution completed"
        "#;

        let result = run_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "Command execution should complete successfully"
        );
    }

    #[tokio::test]
    async fn test_spawn_and_wait_for_process() {
        #[cfg(windows)]
        let code = r#"
            wait for spawn command "timeout 1" as proc
            wait for 500 milliseconds
            wait for process proc to complete as exit_code
            display "Process completed"
        "#;

        #[cfg(not(windows))]
        let code = r#"
            wait for spawn command "sleep 1" as proc
            wait for 500 milliseconds
            wait for process proc to complete as exit_code
            display "Process completed"
        "#;

        let result = run_wfl_code(code).await;
        assert!(result.is_ok(), "Spawn and wait should work: {:?}", result);
    }

    #[tokio::test]
    async fn test_process_is_running() {
        #[cfg(windows)]
        let code = r#"
            wait for spawn command "timeout 2" as proc
            store is_active as process proc is running
            check if is_active:
                display "Process is running"
            end check
            kill process proc
        "#;

        #[cfg(not(windows))]
        let code = r#"
            wait for spawn command "sleep 2" as proc
            store is_active as process proc is running
            check if is_active:
                display "Process is running"
            end check
            kill process proc
        "#;

        let result = run_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "Process status check should work: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_kill_process() {
        #[cfg(windows)]
        let code = r#"
            wait for spawn command "timeout 10" as proc
            wait for 200 milliseconds
            kill process proc
            wait for 200 milliseconds
            store is_alive as process proc is running
        "#;

        #[cfg(not(windows))]
        let code = r#"
            wait for spawn command "sleep 10" as proc
            wait for 200 milliseconds
            kill process proc
            wait for 200 milliseconds
            store is_alive as process proc is running
        "#;

        let result = run_wfl_code_and_get_var(code, "is_alive").await;
        assert!(result.is_ok(), "Kill process should work");

        if let Ok(Value::Bool(alive)) = result {
            assert!(!alive, "Process should not be running after kill");
        } else {
            panic!("is_alive should be a boolean");
        }
    }

    #[tokio::test]
    async fn test_read_process_output() {
        #[cfg(windows)]
        let code = r#"
            wait for spawn command "cmd /c echo test data" as proc
            wait for 200 milliseconds
            wait for read output from process proc as proc_output
            display proc_output
        "#;
        #[cfg(not(windows))]
        let code = r#"
            wait for spawn command "echo test data" as proc
            wait for 200 milliseconds
            wait for read output from process proc as proc_output
            display proc_output
        "#;

        let result = run_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "Reading process output should work: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_error_handling_general() {
        // Test that errors don't crash - they should be caught or propagated
        let code = r#"
            try:
                wait for execute command "nonexistent_xyz_cmd_12345" as result
                display "Command executed"
            when error:
                display "Error caught successfully"
            end try
        "#;

        let result = run_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "Error handling should prevent crash: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_error_handling_invalid_process() {
        // Test that invalid process ID errors are handled
        let code = r#"
            try:
                kill process "invalid_id_xyz"
                display "Should not reach here"
            when error:
                display "Error caught successfully"
            end try
        "#;

        let result = run_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "Invalid process error should be catchable: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_execute_with_shell() {
        #[cfg(windows)]
        let code = r#"
            wait for execute command "cmd /c echo test args" as result
            display "Command with shell executed"
        "#;
        #[cfg(not(windows))]
        // Test that shell commands work correctly
        let code = r#"
            wait for execute command "echo test args" as result
            display "Command with shell executed"
        "#;

        let result = run_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "Execute with shell should work: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_execute_without_variable() {
        #[cfg(windows)]
        let code = r#"
            wait for execute command "cmd /c echo test"
            display "Done"
        "#;
        #[cfg(not(windows))]
        // Test executing without storing result
        let code = r#"
            wait for execute command "echo test"
            display "Done"
        "#;

        let result = run_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "Should be able to execute without storing result"
        );
    }

    #[tokio::test]
    async fn test_multiple_processes() {
        #[cfg(windows)]
        let code = r#"
            wait for spawn command "timeout 1" as proc1
            wait for spawn command "timeout 1" as proc2
            wait for 200 milliseconds
            store p1_running as process proc1 is running
            store p2_running as process proc2 is running
            wait for process proc1 to complete
            wait for process proc2 to complete
        "#;

        #[cfg(not(windows))]
        let code = r#"
            wait for spawn command "sleep 1" as proc1
            wait for spawn command "sleep 1" as proc2
            wait for 200 milliseconds
            store p1_running as process proc1 is running
            store p2_running as process proc2 is running
            wait for process proc1 to complete
            wait for process proc2 to complete
        "#;

        let result = run_wfl_code(code).await;
        assert!(
            result.is_ok(),
            "Multiple processes should work: {:?}",
            result
        );
    }
}
