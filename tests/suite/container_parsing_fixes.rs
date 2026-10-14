// TDD tests for container parsing fixes
// These tests MUST fail first, then implementation will be written to make them pass

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Statement, Type};

#[test]
fn test_container_action_without_return_type_should_parse() {
    let source = r#"
create container Test:
    action greet:
        display "Hello"
    end
end
"#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    // This should pass once the bug is fixed
    assert!(
        result.is_ok(),
        "Parser should handle action without return type: {:?}",
        result.err()
    );
}

#[test]
fn test_container_action_with_parameters_should_parse() {
    let source = r#"
create container Test:
    action set_name needs new_name: Text:
        display "Setting name"
    end
end
"#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    // This should pass once the 'needs' keyword bug is fixed
    assert!(
        result.is_ok(),
        "Parser should handle 'needs' parameters: {:?}",
        result.err()
    );
}

#[test]
fn test_nested_end_tokens_should_parse() {
    let source = r#"
create container Test:
    action greet:
        display "Hello"
    end
end
"#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    // This should pass once the nested end token handling is fixed
    assert!(
        result.is_ok(),
        "Parser should handle nested end tokens: {:?}",
        result.err()
    );
}

#[test]
fn test_all_container_parsing_issues_combined() {
    let source = r#"
create container Person:
    property name: Text
    
    action greet:
        display "Hello, I am " with name
    end
    
    action set_name needs new_name: Text:
        store name as new_name
    end
end
"#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    // This comprehensive test should pass once all bugs are fixed
    assert!(
        result.is_ok(),
        "Parser should handle complete container with all features: {:?}",
        result.err()
    );
}

#[test]
fn lowercase_date_and_time_types_parse_in_action_parameters() {
    let source = r#"
define action called inspect with parameters day as date and clock as time and instant as datetime:
    display day
end action
"#;
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("temporal annotations should parse");
    let Statement::ActionDefinition { parameters, .. } = &program.statements[0] else {
        panic!("expected an action definition");
    };
    assert_eq!(
        parameters
            .iter()
            .map(|parameter| parameter.param_type.clone())
            .collect::<Vec<_>>(),
        vec![
            Some(Type::Date),
            Some(Type::Time),
            Some(Type::Custom("datetime".to_string())),
        ]
    );
}

#[test]
fn lowercase_date_and_time_types_parse_in_container_methods() {
    let source = r#"
create container Clock:
    action set needs day: date, clock: time:
        display day
    end
end
"#;
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("temporal annotations should parse");
    let Statement::ContainerDefinition { methods, .. } = &program.statements[0] else {
        panic!("expected a container definition");
    };
    let Statement::ActionDefinition { parameters, .. } = &methods[0] else {
        panic!("expected a container method");
    };
    assert_eq!(
        parameters
            .iter()
            .map(|parameter| parameter.param_type.clone())
            .collect::<Vec<_>>(),
        vec![Some(Type::Date), Some(Type::Time),]
    );
}

#[test]
fn lowercase_temporal_types_parse_for_container_properties_and_returns() {
    let source = r#"
create container Clock:
    property day: date
    property clock: time

    action get_day: date
        return today
    end

    action get_clock: time
        return now
    end
end
"#;
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .expect("temporal property and return annotations should parse");
    let Statement::ContainerDefinition {
        properties,
        methods,
        ..
    } = &program.statements[0]
    else {
        panic!("expected a container definition");
    };
    assert_eq!(
        properties
            .iter()
            .map(|property| property.property_type.clone())
            .collect::<Vec<_>>(),
        vec![Some(Type::Date), Some(Type::Time),]
    );
    assert_eq!(
        methods
            .iter()
            .map(|method| match method {
                Statement::ActionDefinition { return_type, .. } => return_type.clone(),
                other => panic!("expected action definition, got {other:?}"),
            })
            .collect::<Vec<_>>(),
        vec![Some(Type::Date), Some(Type::Time),]
    );
}

#[test]
fn colon_style_lowercase_primitive_names_remain_custom_types() {
    let source = r#"
create container Holder:
    property numeric: number
    property logical: bOoLeAn

    action accept needs numeric_value: number, logical_value: bOoLeAn:
        display numeric_value
    end
end
"#;
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .expect("historical lowercase custom annotations should parse");
    let Statement::ContainerDefinition {
        properties,
        methods,
        ..
    } = &program.statements[0]
    else {
        panic!("expected a container definition");
    };
    assert_eq!(
        properties
            .iter()
            .map(|property| property.property_type.clone())
            .collect::<Vec<_>>(),
        vec![
            Some(Type::Custom("number".to_string())),
            Some(Type::Custom("bOoLeAn".to_string())),
        ]
    );
    let Statement::ActionDefinition { parameters, .. } = &methods[0] else {
        panic!("expected a container action");
    };
    assert_eq!(
        parameters
            .iter()
            .map(|parameter| parameter.param_type.clone())
            .collect::<Vec<_>>(),
        vec![
            Some(Type::Custom("number".to_string())),
            Some(Type::Custom("bOoLeAn".to_string())),
        ]
    );
}

#[test]
fn list_property_annotations_produce_real_list_types() {
    let source = r#"
create container Collections:
    property anything: List
    property labels: List of Text
    property groups: List of List of Number
end
"#;
    let program = Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .expect("documented colon-style list property annotations should parse");
    let Statement::ContainerDefinition { properties, .. } = &program.statements[0] else {
        panic!("expected a container definition");
    };
    assert_eq!(
        properties
            .iter()
            .map(|property| property.property_type.clone())
            .collect::<Vec<_>>(),
        vec![
            Some(Type::List(Box::new(Type::Any))),
            Some(Type::List(Box::new(Type::Text))),
            Some(Type::List(Box::new(Type::List(Box::new(Type::Number))))),
        ]
    );
}

#[test]
fn typed_list_property_contract_is_reachable_from_source() {
    let source = r#"
create container Messages:
    property items: List of Text defaults []

    action corrupt:
        push with items and 1
    end
end
"#;
    let program = Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .expect("typed list property should parse");
    let diagnostics = wfl::typechecker::TypeChecker::new()
        .check_types(&program)
        .expect_err("the parsed List of Text contract must reject a Number insertion")
        .into_diagnostics();
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.message.contains("items") && diagnostic.message.contains("Text")
        }),
        "expected the source-level property contract diagnostic: {diagnostics:?}"
    );
}
