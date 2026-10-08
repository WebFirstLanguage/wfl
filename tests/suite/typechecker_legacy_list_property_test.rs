use std::sync::Arc;
use wfl::parser::ast::{
    Expression, Literal, Parameter, Program, PropertyDefinition, Statement, Type, Visibility,
};
use wfl::typechecker::{TypeCheckError, TypeChecker};

fn number(value: i64) -> Expression {
    Expression::Literal(Literal::Integer(value), 1, 1)
}

fn text(value: &str) -> Expression {
    Expression::Literal(Literal::String(Arc::from(value)), 1, 1)
}

fn empty_list() -> Expression {
    Expression::Literal(Literal::List(Vec::new()), 1, 1)
}

fn variable(name: &str) -> Expression {
    Expression::Variable(name.to_string(), 1, 1)
}

fn property(name: &str, property_type: Type) -> PropertyDefinition {
    PropertyDefinition {
        name: name.to_string(),
        property_type: Some(property_type),
        default_value: Some(empty_list()),
        validation_rules: Vec::new(),
        is_static: false,
        visibility: Visibility::Public,
        line: 1,
        column: 1,
    }
}

fn parameter(name: &str, parameter_type: Type) -> Parameter {
    Parameter {
        name: name.to_string(),
        param_type: Some(parameter_type),
        default_value: None,
        line: 1,
        column: 1,
    }
}

fn action(name: &str, parameters: Vec<Parameter>, body: Vec<Statement>) -> Statement {
    Statement::ActionDefinition {
        name: name.to_string(),
        parameters,
        body,
        return_type: None,
        line: 1,
        column: 1,
    }
}

fn container(properties: Vec<PropertyDefinition>, methods: Vec<Statement>) -> Statement {
    Statement::ContainerDefinition {
        name: "Messages".to_string(),
        extends: None,
        implements: Vec::new(),
        properties,
        methods,
        events: Vec::new(),
        static_properties: Vec::new(),
        static_methods: Vec::new(),
        line: 1,
        column: 1,
    }
}

fn outer_number(name: &str) -> Statement {
    Statement::VariableDeclaration {
        name: name.to_string(),
        value: number(1),
        is_constant: false,
        line: 1,
        column: 1,
    }
}

fn add(value: Expression, list_name: &str) -> Statement {
    Statement::AddToListStatement {
        value,
        list_name: list_name.to_string(),
        line: 1,
        column: 1,
    }
}

fn check(statements: Vec<Statement>) -> Result<(), TypeCheckError> {
    TypeChecker::new().check_types(&Program { statements })
}

fn assert_property_error(
    result: Result<(), TypeCheckError>,
    property_name: &str,
    expected_type: &str,
) {
    let diagnostics = result
        .expect_err("property element contract should be enforced")
        .into_diagnostics();
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.message.contains("property")
                && diagnostic.message.contains(property_name)
                && diagnostic.message.contains(expected_type)
        }),
        "expected a {expected_type} contract error for property {property_name:?}, got {diagnostics:?}"
    );
}

#[test]
fn legacy_add_resolves_property_before_outer_binding() {
    check(vec![
        outer_number("items"),
        container(
            vec![property("items", Type::List(Box::new(Type::Text)))],
            vec![action(
                "append",
                Vec::new(),
                vec![add(text("hello"), "items")],
            )],
        ),
    ])
    .unwrap_or_else(|failure| {
        panic!(
            "the current container property should shadow the outer binding: {:?}",
            failure.into_diagnostics()
        )
    });
}

#[test]
fn legacy_add_rejects_incompatible_property_element() {
    assert_property_error(
        check(vec![container(
            vec![property("items", Type::List(Box::new(Type::Text)))],
            vec![action("corrupt", Vec::new(), vec![add(number(1), "items")])],
        )]),
        "items",
        "Text",
    );
}

#[test]
fn legacy_add_rejects_gradual_value_for_concrete_property_element() {
    assert_property_error(
        check(vec![container(
            vec![property("items", Type::List(Box::new(Type::Text)))],
            vec![action(
                "append_dynamic",
                vec![parameter("value", Type::Any)],
                vec![add(variable("value"), "items")],
            )],
        )]),
        "items",
        "Text",
    );
}

#[test]
fn legacy_add_accepts_fresh_empty_list_for_nested_property_element() {
    check(vec![
        outer_number("groups"),
        container(
            vec![property(
                "groups",
                Type::List(Box::new(Type::List(Box::new(Type::Number)))),
            )],
            vec![action(
                "append_empty",
                Vec::new(),
                vec![add(empty_list(), "groups")],
            )],
        ),
    ])
    .unwrap_or_else(|failure| {
        panic!(
            "a fresh empty list satisfies any declared nested list element shape: {:?}",
            failure.into_diagnostics()
        )
    });
}

#[test]
fn legacy_remove_and_clear_resolve_property_before_outer_binding() {
    check(vec![
        outer_number("items"),
        container(
            vec![property("items", Type::List(Box::new(Type::Text)))],
            vec![action(
                "reset",
                Vec::new(),
                vec![
                    Statement::RemoveFromListStatement {
                        value: text("obsolete"),
                        list_name: "items".to_string(),
                        line: 1,
                        column: 1,
                    },
                    Statement::ClearListStatement {
                        list_name: "items".to_string(),
                        line: 1,
                        column: 1,
                    },
                ],
            )],
        ),
    ])
    .unwrap_or_else(|failure| {
        panic!(
            "remove/clear should resolve the current property before the outer binding: {:?}",
            failure.into_diagnostics()
        )
    });
}

#[test]
fn action_parameter_still_shadows_same_named_list_property() {
    check(vec![container(
        vec![property("items", Type::List(Box::new(Type::Text)))],
        vec![action(
            "append",
            vec![parameter("items", Type::Number)],
            vec![add(number(1), "items")],
        )],
    )])
    .expect("the local Number parameter should select arithmetic add and shadow the property");
}
