use wfl::analyzer::Analyzer;
use wfl::interpreter::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

#[tokio::test]
async fn test_store_redefinition_error() {
    let source = r#"
        store x as 5
        store x as 10
    "#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().unwrap();

    // Analyzer should catch the redefinition
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);
    assert!(result.is_err());

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        let error = &errors[0];
        assert!(error.message.contains("already been defined"));
        assert!(error.message.contains("change"));
    }
}

#[tokio::test]
async fn test_change_without_define_error() {
    let source = r#"
        change x to 10
    "#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().unwrap();

    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&ast).await;

    assert!(result.is_err());
    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("Undefined variable"));
    }
}

#[tokio::test]
async fn test_store_then_change_success() {
    let source = r#"
        store x as 5
        change x to 10
        display x
    "#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().unwrap();

    // Analyzer should pass
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);
    assert!(result.is_ok());

    // Interpreter should execute successfully
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&ast).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_constant_redefinition_error() {
    let source = r#"
        store new constant PI as 3.14
        store new constant PI as 3.14159
    "#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().unwrap();

    // Analyzer should catch the constant redefinition
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);
    assert!(result.is_err());

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        let error = &errors[0];
        assert!(error.message.contains("already been defined"));
    }
}

#[tokio::test]
async fn test_constant_modification_error() {
    let source = r#"
        store new constant PI as 3.14
        change PI to 3.14159
    "#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().unwrap();

    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&ast).await;

    assert!(result.is_err());
    if let Err(errors) = result {
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("Cannot modify constant"));
    }
}

#[tokio::test]
async fn test_scope_redefinition_in_inner_scope() {
    let source = r#"
        store x as 5
        check if yes:
            store x as 10
        end check
    "#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().unwrap();

    // Analyzer should catch the redefinition in inner scope
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);
    assert!(result.is_err());

    if let Err(errors) = result {
        assert!(!errors.is_empty());
        let error = &errors[0];
        assert!(
            error
                .message
                .contains("already been defined in an outer scope")
        );
        assert!(error.message.contains("change"));
    }
}

#[tokio::test]
async fn test_scope_change_in_inner_scope() {
    let source = r#"
        store x as 5
        check if yes:
            change x to 10
            display x
        end check
        display x
    "#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().unwrap();

    // Analyzer should pass
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);
    assert!(result.is_ok());

    // Interpreter should execute successfully
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&ast).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_loop_variable_redefinition() {
    let source = r#"
        store counter as 0
        count from 1 to 5:
            display count
        end count
    "#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().unwrap();

    // The loop creates its own scope with 'count', which should work
    // even though there's a 'count' variable in the outer scope
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&ast).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_function_parameter_shadows_outer_variable() {
    let source = r#"
        store x as 5
        define action called show_value needs x:
            display x
        end action
        show_value with "hello"
    "#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().unwrap();

    // Function parameters can shadow outer variables - this is ok
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&ast).await;
    assert!(result.is_ok());
}
