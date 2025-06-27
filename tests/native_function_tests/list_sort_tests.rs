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
async fn test_sort_function_exists() {
    let code = r#"
    create list as numbers
    push with numbers and 3
    push with numbers and 1
    push with numbers and 2
    store result as sort(numbers)
    "#;

    let result = execute_wfl(code).await;
    assert!(result.is_ok(), "sort() function should be available");
}

#[tokio::test]
async fn test_sort_numbers_ascending() {
    let code = r#"
    create list as numbers
    push with numbers and 3
    push with numbers and 1
    push with numbers and 2
    store result as sort(numbers)
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
        Value::List(list) => {
            let items = list.borrow();
            assert_eq!(items.len(), 3);
            match (&items[0], &items[1], &items[2]) {
                (Value::Number(a), Value::Number(b), Value::Number(c)) => {
                    assert_eq!(*a, 1.0);
                    assert_eq!(*b, 2.0);
                    assert_eq!(*c, 3.0);
                },
                _ => panic!("Expected numbers in sorted list"),
            }
        },
        _ => panic!("Expected list, got {:?}", result_value),
    }
}

#[tokio::test]
async fn test_sort_numbers_descending() {
    let code = r#"
    create list as numbers
    push with numbers and 1
    push with numbers and 3
    push with numbers and 2
    store result as sort(numbers, null, true)
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
        Value::List(list) => {
            let items = list.borrow();
            assert_eq!(items.len(), 3);
            match (&items[0], &items[1], &items[2]) {
                (Value::Number(a), Value::Number(b), Value::Number(c)) => {
                    assert_eq!(*a, 3.0);
                    assert_eq!(*b, 2.0);
                    assert_eq!(*c, 1.0);
                },
                _ => panic!("Expected numbers in sorted list"),
            }
        },
        _ => panic!("Expected list, got {:?}", result_value),
    }
}

#[tokio::test]
async fn test_sort_text_ascending() {
    let code = r#"
    create list as words
    push with words and "zebra"
    push with words and "apple"
    push with words and "banana"
    store result as sort(words)
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
        Value::List(list) => {
            let items = list.borrow();
            assert_eq!(items.len(), 3);
            match (&items[0], &items[1], &items[2]) {
                (Value::Text(a), Value::Text(b), Value::Text(c)) => {
                    assert_eq!(a.as_ref(), "apple");
                    assert_eq!(b.as_ref(), "banana");
                    assert_eq!(c.as_ref(), "zebra");
                },
                _ => panic!("Expected text in sorted list"),
            }
        },
        _ => panic!("Expected list, got {:?}", result_value),
    }
}

#[tokio::test]
async fn test_sort_empty_list() {
    let code = r#"
    create list as empty
    store result as sort(empty)
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
        Value::Number(n) => assert_eq!(n, 0.0, "Expected empty list"),
        _ => panic!("Expected number, got {:?}", len_value),
    }
}

#[tokio::test]
async fn test_sort_wrong_argument_count() {
    let code = r#"
    store result as sort()
    "#;

    let result = execute_wfl(code).await;
    assert!(result.is_err(), "sort() should require at least 1 argument");
}

#[tokio::test]
async fn test_sort_non_list_argument() {
    let code = r#"
    store result as sort("not a list")
    "#;

    let result = execute_wfl(code).await;
    assert!(result.is_err(), "sort() should require a list argument");
}
