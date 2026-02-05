// Test for AST corruption bug in container action parsing
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, Literal, Statement};

#[test]
fn test_container_action_ast_structure() {
    let source = r#"
create container Test:
    property name: Text
    
    action greet:
        display "Hello"
    end
end
"#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

    let program = result.unwrap();
    assert_eq!(
        program.statements.len(),
        1,
        "Should have exactly one statement (container definition)"
    );

    // Verify the container structure is correct
    if let Statement::ContainerDefinition { methods, .. } = &program.statements[0] {
        assert_eq!(methods.len(), 1, "Should have exactly one method");

        if let Statement::ActionDefinition { name, body, .. } = &methods[0] {
            assert_eq!(name, "greet", "Method name should be 'greet'");
            assert_eq!(
                body.len(),
                1,
                "Method should have exactly one statement in body"
            );

            // Verify the display statement structure
            if let Statement::DisplayStatement { value, .. } = &body[0] {
                if let Expression::Literal(Literal::String(s), line, column) = value {
                    assert_eq!(s.as_ref(), "Hello", "Display text should be 'Hello'");
                    // These coordinates should point to the string literal in the action body,
                    // not to some corrupted position from earlier parsing
                    assert_eq!(*line, 6, "String literal should be on line 6 (action body)");
                    assert!(
                        *column > 10,
                        "String literal should be at reasonable column position in action body, got column {}",
                        column
                    );
                } else {
                    panic!("Display value should be a string literal, got: {:?}", value);
                }
            } else {
                panic!(
                    "Method body should contain a display statement, got: {:?}",
                    body[0]
                );
            }
        } else {
            panic!(
                "Container method should be an ActionDefinition, got: {:?}",
                methods[0]
            );
        }
    } else {
        panic!(
            "Statement should be a ContainerDefinition, got: {:?}",
            program.statements[0]
        );
    }
}

#[test]
fn test_action_with_parameters_ast_structure() {
    let source = r#"
create container Test:
    property name: Text
    
    action set_name needs new_name: Text:
        store name as new_name
    end
end
"#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

    let program = result.unwrap();

    if let Statement::ContainerDefinition { methods, .. } = &program.statements[0]
        && let Statement::ActionDefinition {
            name,
            parameters,
            body,
            ..
        } = &methods[0]
    {
        assert_eq!(name, "set_name");
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].name, "new_name");
        assert_eq!(body.len(), 1);

        // Check that the store statement has correct structure
        if let Statement::VariableDeclaration {
            name: var_name,
            value,
            ..
        } = &body[0]
        {
            assert_eq!(var_name, "name");
            if let Expression::Variable(param_name, line, column) = value {
                assert_eq!(param_name, "new_name");
                // This should point to the parameter usage in the action body
                assert_eq!(*line, 6, "Parameter usage should be on line 6");
                assert!(
                    *column > 20,
                    "Parameter should be at reasonable column in store statement, got column {}",
                    column
                );
            } else {
                panic!(
                    "Store value should be a variable reference to new_name, got: {:?}",
                    value
                );
            }
        } else {
            panic!(
                "Action body should contain a store statement, got: {:?}",
                body[0]
            );
        }
    }
}
