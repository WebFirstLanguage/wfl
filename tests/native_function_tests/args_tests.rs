use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use std::cell::RefCell;
use std::rc::Rc;

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
async fn test_args_function_exists() {
    let code = r#"
    store result as args()
    "#;

    let result = execute_wfl(code).await;
    assert!(result.is_ok(), "args() function should be available");
}

#[tokio::test]
async fn test_args_returns_list() {
    let code = r#"
    store result as args()
    "#;

    let tokens = lex_wfl_with_positions(code);
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
async fn test_args_with_no_arguments() {
    let code = r#"
    store result as args()
    store len as length(result)
    "#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse program");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Failed to execute program: {:?}", result);

    let env = interpreter.global_env();
    let len_value = env.borrow().get("len").expect("Length not found");

    match len_value {
        Value::Number(n) => assert_eq!(n, 0.0, "Expected empty args list"),
        _ => panic!("Expected number, got {:?}", len_value),
    }
}

#[tokio::test]
async fn test_args_function_call_with_wrong_arguments() {
    let code = r#"
    store result as args("unexpected")
    "#;

    let result = execute_wfl(code).await;
    assert!(result.is_err(), "args() should not accept arguments");
}
