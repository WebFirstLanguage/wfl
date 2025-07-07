use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, Literal, Statement};

#[test]
fn test_action_def_parses() {
    let source = "define action called log_message needs message_text:
        display message_text
    end action";

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse program");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::ActionDefinition {
            name, parameters, ..
        } => {
            assert_eq!(name, "log_message");
            assert_eq!(parameters.len(), 1);
            assert_eq!(parameters[0].name, "message_text");
        }
        _ => panic!("Expected ActionDefinition, got {:?}", program.statements[0]),
    }
}

#[test]
fn test_action_call_parses() {
    // Define the action first, then call it
    let source = "define action called log_message needs message_text:
        display message_text
    end action

    log_message with \"Hello, world!\"";

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse program");

    assert_eq!(program.statements.len(), 2);
    match &program.statements[1] {
        Statement::ExpressionStatement { expression, .. } => match expression {
            Expression::ActionCall {
                name, arguments, ..
            } => {
                assert_eq!(name, "log_message");
                assert_eq!(arguments.len(), 1);
                match &arguments[0].value {
                    Expression::Literal(Literal::String(value), ..) => {
                        assert_eq!(value, "Hello, world!");
                    }
                    _ => panic!("Expected string literal, got {:?}", arguments[0].value),
                }
            }
            _ => panic!("Expected ActionCall, got {expression:?}"),
        },
        _ => panic!(
            "Expected ExpressionStatement, got {:?}",
            program.statements[1]
        ),
    }
}

#[test]
fn test_parser_token_consumption() {
    let source = "define action called log_message needs message_text:
        display message_text
    end action
    
    log_message with \"test\"";

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(result.is_ok(), "Parser should not go into an infinite loop");
    let program = result.unwrap();
    assert_eq!(program.statements.len(), 2);
}

#[tokio::test]
async fn test_action_call_executes() {
    let source = "
    define action called test_action needs param:
        param
    end action
    
    store result as test_action with 42
    ";

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse program");

    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&program).await;
    assert!(result.is_ok(), "Failed to execute program: {result:?}");

    let env = interpreter.global_env();
    let result_value = env.borrow().get("result").expect("Result not found");

    match result_value {
        Value::Number(n) => assert_eq!(n, 42.0),
        _ => panic!("Expected number, got {result_value:?}"),
    }
}
