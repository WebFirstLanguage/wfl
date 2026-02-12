use super::{Interpreter, Value};
use crate::lexer::lex_wfl_with_positions;
use crate::parser::Parser;
use std::sync::Arc;

#[tokio::test]
async fn test_binary_ops() {
    let mut interpreter = Interpreter::new();

    // Addition
    let result = run_expr(&mut interpreter, "2 plus 3").await;
    assert_eq!(result, Value::Number(5.0));

    // Subtraction
    let result = run_expr(&mut interpreter, "5 minus 2").await;
    assert_eq!(result, Value::Number(3.0));

    // Multiplication
    let result = run_expr(&mut interpreter, "4 times 3").await;
    assert_eq!(result, Value::Number(12.0));

    // Division
    let result = run_expr(&mut interpreter, "10 divided by 2").await;
    assert_eq!(result, Value::Number(5.0));

    // Modulo (using % as modulo keyword is not supported)
    let result = run_expr(&mut interpreter, "10 % 3").await;
    assert_eq!(result, Value::Number(1.0));

    // Equality (using 'is equal to' or '=')
    let result = run_expr(&mut interpreter, "5 is equal to 5").await;
    assert_eq!(result, Value::Bool(true));

    let result = run_expr(&mut interpreter, "5 = 5").await;
    assert_eq!(result, Value::Bool(true));

    let result = run_expr(&mut interpreter, "5 is equal to 6").await;
    assert_eq!(result, Value::Bool(false));

    // Inequality
    let result = run_expr(&mut interpreter, "5 is not 6").await;
    assert_eq!(result, Value::Bool(true));

    // Greater than
    let result = run_expr(&mut interpreter, "5 is greater than 3").await;
    assert_eq!(result, Value::Bool(true));

    // Less than
    let result = run_expr(&mut interpreter, "2 is less than 4").await;
    assert_eq!(result, Value::Bool(true));

    // Greater than or equal
    let result = run_expr(&mut interpreter, "5 is greater than or equal to 5").await;
    assert_eq!(result, Value::Bool(true));

    // Less than or equal
    let result = run_expr(&mut interpreter, "3 is less than or equal to 3").await;
    assert_eq!(result, Value::Bool(true));

    // And
    let result = run_expr(&mut interpreter, "true and true").await;
    assert_eq!(result, Value::Bool(true));

    let result = run_expr(&mut interpreter, "true and false").await;
    assert_eq!(result, Value::Bool(false));

    // Or
    let result = run_expr(&mut interpreter, "true or false").await;
    assert_eq!(result, Value::Bool(true));

    let result = run_expr(&mut interpreter, "false or false").await;
    assert_eq!(result, Value::Bool(false));

    // Contains (List)
    let result = run_expr(&mut interpreter, "contains 2 in [1, 2, 3]").await;
    assert_eq!(result, Value::Bool(true));

    // Contains (Text)
    let result = run_expr(&mut interpreter, "contains \"bar\" in \"foobar\"").await;
    assert_eq!(result, Value::Bool(true));
}

#[tokio::test]
async fn test_unary_ops() {
    let mut interpreter = Interpreter::new();

    // Not
    let result = run_expr(&mut interpreter, "not true").await;
    assert_eq!(result, Value::Bool(false));

    let result = run_expr(&mut interpreter, "not false").await;
    assert_eq!(result, Value::Bool(true));

    // Minus (Negation)
    let result = run_expr(&mut interpreter, "-5").await;
    assert_eq!(result, Value::Number(-5.0));
}

#[tokio::test]
async fn test_concatenation() {
    let mut interpreter = Interpreter::new();

    // Using 'with' for concatenation
    let result = run_expr(&mut interpreter, "\"hello\" with \" \" with \"world\"").await;
    assert_eq!(result, Value::Text(Arc::from("hello world")));

    // Concatenation with non-text types (implicit string conversion)
    let result = run_expr(&mut interpreter, "\"count: \" with 5").await;
    assert_eq!(result, Value::Text(Arc::from("count: 5")));
}

// Helper function to run expression
async fn run_expr(interpreter: &mut Interpreter, source: &str) -> Value {
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .unwrap_or_else(|_| panic!("Failed to parse: {}", source));
    interpreter
        .interpret(&program)
        .await
        .unwrap_or_else(|_| panic!("Failed to interpret: {}", source))
}
