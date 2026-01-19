use std::time::Duration;
use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

async fn execute_wfl_code(code: &str) -> Result<Value, String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().map_err(|e| format!("{:?}", e))?;

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .map_err(|e| format!("{:?}", e))
}

#[tokio::test]
async fn test_literal_optimization_performance() {
    let code = r#"
        // Test literal optimization with tight loop
        store start_time as current time in milliseconds
        count from 1 to 10000:
            store x as 42
            store y as "hello"
            store z as true
        end count
        store end_time as current time in milliseconds
        give back end_time - start_time
    "#;

    // Warmup run
    let _ = execute_wfl_code(code).await;

    let start = std::time::Instant::now();
    let result = execute_wfl_code(code).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Execution failed: {:?}", result.err());

    // On a reasonable machine, this should be well under 1 second with optimization
    // Without optimization it might be slower, but 2s is a safe upper bound
    // The main point is to ensure it runs correctly and reasonably fast
    assert!(
        elapsed < Duration::from_millis(2000),
        "Literal optimization not effective, took {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_literal_correctness() {
    let code = r#"
        store i as 123
        store f as 123.456
        store s as "test string"
        store b as true
        store n as nothing

        if i = 123 then
            // ok
        otherwise
            give back "integer failed"
        end if

        if s = "test string" then
            // ok
        otherwise
            give back "string failed"
        end if

        if b = true then
            // ok
        otherwise
            give back "boolean failed"
        end if

        if n = nothing then
            // ok
        otherwise
            give back "nothing failed"
        end if

        give back "success"
    "#;

    let result = execute_wfl_code(code).await;
    assert_eq!(result.unwrap(), Value::Text("success".into()));
}
