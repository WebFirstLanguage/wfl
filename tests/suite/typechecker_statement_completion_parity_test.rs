use wfl::parser::ast::{Expression, Literal, PatternExpression, Program, Statement, Type};
use wfl::typechecker::{TypeCheckError, TypeChecker};

fn number(value: i64) -> Expression {
    Expression::Literal(Literal::Integer(value), 1, 1)
}

fn action(name: &str, return_type: Type, body: Vec<Statement>) -> Statement {
    Statement::ActionDefinition {
        name: name.to_string(),
        parameters: vec![],
        body,
        return_type: Some(return_type),
        line: 1,
        column: 1,
    }
}

fn container(name: &str, extends: Option<&str>, methods: Vec<Statement>) -> Statement {
    Statement::ContainerDefinition {
        name: name.to_string(),
        extends: extends.map(str::to_string),
        implements: vec![],
        properties: vec![],
        methods,
        events: vec![],
        static_properties: vec![],
        static_methods: vec![],
        line: 1,
        column: 1,
    }
}

fn check(statements: Vec<Statement>) -> Result<(), TypeCheckError> {
    TypeChecker::new().check_types(&Program { statements })
}

fn assert_exact_completion_type(
    prerequisites: Vec<Statement>,
    completion: Statement,
    expected: Type,
) {
    let mut accepted = prerequisites.clone();
    accepted.push(action(
        "returns_expected_type",
        expected,
        vec![completion.clone()],
    ));
    check(accepted).expect("the implicit result must have the runtime statement's exact type");

    let mut rejected = prerequisites;
    rejected.push(action("rejects_wrong_type", Type::Text, vec![completion]));
    let diagnostics = check(rejected)
        .expect_err("an exact statement result must not degrade to Any")
        .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("implicit result")),
        "expected an implicit-result mismatch, got {diagnostics:?}"
    );
}

#[test]
fn container_instantiation_completes_with_the_instance() {
    let widget = container("Widget", None, vec![]);
    let instantiate = Statement::ContainerInstantiation {
        container_type: "Widget".to_string(),
        instance_name: "widget".to_string(),
        arguments: vec![],
        property_initializers: vec![],
        line: 1,
        column: 1,
    };

    assert_exact_completion_type(
        vec![widget],
        instantiate,
        Type::ContainerInstance("Widget".to_string()),
    );
}

#[test]
fn container_definition_completes_with_the_definition() {
    let definition = container("InnerContainer", None, vec![]);
    assert_exact_completion_type(
        vec![],
        definition,
        Type::Container("InnerContainer".to_string()),
    );
}

#[test]
fn interface_definition_completes_with_the_definition() {
    let definition = Statement::InterfaceDefinition {
        name: "Renderable".to_string(),
        extends: vec![],
        required_actions: vec![],
        line: 1,
        column: 1,
    };
    assert_exact_completion_type(
        vec![],
        definition,
        Type::Interface("Renderable".to_string()),
    );
}

#[test]
fn pattern_definition_completes_with_the_compiled_pattern() {
    let definition = Statement::PatternDefinition {
        name: "letter_a".to_string(),
        pattern: PatternExpression::Literal("a".to_string()),
        line: 1,
        column: 1,
    };
    assert_exact_completion_type(vec![], definition, Type::Pattern);
}

#[test]
fn parent_method_call_completes_with_the_parent_method_result() {
    let parent_method = action(
        "value",
        Type::Number,
        vec![Statement::ReturnStatement {
            value: Some(number(7)),
            line: 1,
            column: 1,
        }],
    );
    let parent = container("Parent", None, vec![parent_method]);
    let parent_call = Statement::ParentMethodCall {
        method_name: "value".to_string(),
        arguments: vec![],
        line: 1,
        column: 1,
    };
    let child_method = action("child_value", Type::Number, vec![parent_call.clone()]);
    let child = container("Child", Some("Parent"), vec![child_method]);

    check(vec![parent.clone(), child])
        .expect("a parent method statement must supply its method's runtime result");

    let wrong_child = container(
        "WrongChild",
        Some("Parent"),
        vec![action("child_value", Type::Text, vec![parent_call])],
    );
    let diagnostics = check(vec![parent, wrong_child])
        .expect_err("a parent method result must not degrade to Any")
        .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("implicit result")),
        "expected an implicit-result mismatch, got {diagnostics:?}"
    );
}

#[test]
fn event_definition_has_a_dynamic_completion_type() {
    let definition = Statement::EventDefinition {
        name: "updated".to_string(),
        parameters: vec![],
        line: 1,
        column: 1,
    };

    check(vec![action("define_event", Type::Any, vec![definition])])
        .expect("events have runtime values but no dedicated static event type");
}

#[test]
fn include_with_a_return_has_a_dynamic_completion_type() {
    let include = Statement::IncludeStatement {
        path: Expression::Literal(Literal::String("module.wfl".into()), 1, 1),
        line: 1,
        column: 1,
    };

    check(vec![action("include_value", Type::Any, vec![include])])
        .expect("an included file may return a value of a type unavailable to the caller");
}
