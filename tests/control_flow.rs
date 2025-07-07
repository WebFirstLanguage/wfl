use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

async fn execute_wfl(code: &str) -> Result<Value, String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .map_err(|e| format!("Parse error: {e:?}"))?;

    let mut interpreter = Interpreter::default();
    interpreter
        .interpret(&program)
        .await
        .map_err(|e| format!("Runtime error: {e:?}"))
}

#[tokio::test]
async fn test_break_in_forever_loop() {
    let code = r#"
    store counter as 0
    repeat forever:
        change counter to counter plus 1
        check if counter is greater than 5:
            break
        end check
    end repeat
    display counter
    "#;

    let result = execute_wfl(code).await.unwrap();
    assert_eq!(result, Value::Null); // The last statement is display, which returns Null
}

#[tokio::test]
async fn test_break_in_count_loop() {
    let code = r#"
    store result as 0
    count from 1 to 10:
        change result to result plus count
        check if count is greater than 5:
            break
        end check
    end count
    display result
    "#;

    let result = execute_wfl(code).await.unwrap();
    assert_eq!(result, Value::Null);
}

#[tokio::test]
async fn test_break_in_repeat_while_loop() {
    let code = r#"
    store counter as 0
    store result as 0
    repeat while counter is less than 10:
        change counter to counter plus 1
        change result to result plus counter
        check if counter is greater than 5:
            break
        end check
    end repeat
    display result
    "#;

    let result = execute_wfl(code).await.unwrap();
    assert_eq!(result, Value::Null);
}

#[tokio::test]
async fn test_break_in_repeat_until_loop() {
    let code = r#"
    store counter as 0
    store result as 0
    repeat until counter is greater than 9:
        change counter to counter plus 1
        change result to result plus counter
        check if counter is greater than 5:
            break
        end check
    end repeat
    display result
    "#;

    let result = execute_wfl(code).await.unwrap();
    assert_eq!(result, Value::Null);
}

#[tokio::test]
async fn test_continue_in_count_loop() {
    let code = r#"
    store result as 0
    count from 1 to 10:
        check if count is equal to 2:
            continue
        end check
        change result to result plus count
    end count
    display result
    "#;

    let result = execute_wfl(code).await.unwrap();
    assert_eq!(result, Value::Null);
}

#[tokio::test]
async fn test_exit_from_nested_loops() {
    let code = r#"
    store result as 0
    count from 1 to 5:
        count from 1 to 5:
            change result to result plus 1
            check if count is equal to 3:
                exit loop
            end check
        end count
    end count
    display result
    "#;

    let result = execute_wfl(code).await.unwrap();
    assert_eq!(result, Value::Null);
}

#[tokio::test]
async fn test_nested_loops_with_break() {
    let code = r#"
    store outer_count as 0
    store inner_count as 0
    store total as 0
    
    repeat while outer_count is less than 5:
        change outer_count to outer_count plus 1
        store inner_count as 0
        
        repeat while inner_count is less than 5:
            change inner_count to inner_count plus 1
            change total to total plus 1
            
            check if inner_count is equal to 3:
                break
            end check
        end repeat
    end repeat
    
    display total
    "#;

    let result = execute_wfl(code).await.unwrap();
    assert_eq!(result, Value::Null);
}

#[tokio::test]
async fn test_return_from_action() {
    let code = r#"
    define action called test_return needs x:
        check if x is greater than 5:
            give back x
        end check
        give back x times 2
    end action
    
    store result as test_return with 10
    display result
    "#;

    let result = execute_wfl(code).await.unwrap();
    assert_eq!(result, Value::Null);
}

#[tokio::test]
async fn test_return_from_loop_in_action() {
    let code = r#"
    define action called find_value needs target:
        count from 1 to 10:
            check if count is equal to target:
                give back count
            end check
        end count
        give back 0
    end action
    
    store result as find_value with 5
    display result
    "#;

    let result = execute_wfl(code).await.unwrap();
    assert_eq!(result, Value::Null);
}

#[tokio::test]
async fn test_break_from_foreach_loop() {
    let code = r#"
    create list as items
    push with items and 1
    push with items and 2
    push with items and 3
    push with items and 4
    push with items and 5
    push with items and 6
    push with items and 7
    push with items and 8
    push with items and 9
    push with items and 10
    store sum as 0
    
    for each item in items:
        change sum to sum plus item
        check if item is greater than 4:
            break
        end check
    end for
    
    display sum
    "#;

    let result = execute_wfl(code).await.unwrap();
    assert_eq!(result, Value::Null);
}

#[tokio::test]
async fn test_continue_from_foreach_loop() {
    let code = r#"
    create list as items
    push with items and 1
    push with items and 2
    push with items and 3
    push with items and 4
    push with items and 5
    push with items and 6
    push with items and 7
    push with items and 8
    push with items and 9
    push with items and 10
    store sum as 0
    
    for each item in items:
        check if (item divided by 2) times 2 is equal to item:
            continue
        end check
        change sum to sum plus item
    end for
    
    display sum
    "#;

    let result = execute_wfl(code).await.unwrap();
    assert_eq!(result, Value::Null);
}
