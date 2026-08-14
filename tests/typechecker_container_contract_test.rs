use wfl::analyzer::Analyzer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{
    Argument, EventDefinition, Expression, Literal, Parameter, Program, PropertyDefinition,
    Statement, Type, Visibility,
};
use wfl::typechecker::{TypeCheckError, TypeChecker};

fn typecheck(source: &str) -> Result<(), TypeCheckError> {
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("test program should parse");
    TypeChecker::new().check_types(&program)
}

#[test]
fn action_local_container_instances_keep_their_instance_type() {
    let source = r#"
create container Widget:
    property amount: Number
end

define action called inspect:
    create new Widget as item:
        amount is 1
    end
    store invalid as item minus 1
end action
"#;
    let diagnostics = typecheck(source)
        .expect_err("a container instance is not a number")
        .into_diagnostics();
    assert!(
        diagnostics.iter().any(|error| {
            error.message.contains("Cannot perform Minus")
                && error.message.contains("Instance<Widget>")
        }),
        "expected the local binding to retain Widget's instance type: {diagnostics:?}"
    );
}

#[test]
fn container_initializers_check_property_names_and_types() {
    let diagnostics = typecheck(
        r#"
create container Widget:
    property amount: Number
end
create new Widget as item:
    amount is "wrong"
    extra_prop is 1
end
"#,
    )
    .expect_err("declared container properties are statically typed")
    .into_diagnostics();

    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("amount") && error.message.contains("Number")),
        "expected the property type mismatch: {diagnostics:?}"
    );
    assert!(
        diagnostics.iter().any(
            |error| error.message.contains("extra_prop") && error.message.contains("not found")
        ),
        "expected the unknown-property diagnostic: {diagnostics:?}"
    );
}

#[test]
fn inherited_properties_are_valid_initializers() {
    typecheck(
        r#"
create container Parent:
    property label: Text
end
create container Child extends Parent:
    property amount: Number
end
create new Child as item:
    label is "ok"
    amount is 1
end
"#,
    )
    .unwrap_or_else(|failure| {
        panic!(
            "initializers may target inherited properties: {:?}",
            failure.into_diagnostics()
        )
    });
}

#[test]
fn incompatible_inherited_property_overrides_emit_a_compatibility_warning() {
    let source = r#"
create container Parent:
    property value: Number defaults 1

    action reset:
        change value to 2
    end
end

create container Child extends Parent:
    property value: Text defaults "child"
end
"#;
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("test program should parse");
    let mut analyzer = Analyzer::new();
    analyzer
        .analyze(&program)
        .expect("the compatibility warning must remain non-fatal");

    assert!(
        analyzer.get_warnings().iter().any(|warning| {
            warning.message.contains("value")
                && warning.message.contains("Parent")
                && warning.message.contains("Number")
                && warning.message.contains("Text")
        }),
        "expected an incompatible inherited-property override warning: {:?}",
        analyzer.get_warnings()
    );
    typecheck(source).expect("the warning must not break an existing WFL program");
}

#[test]
fn inherited_property_overrides_accept_the_same_contract() {
    typecheck(
        r#"
create container Parent:
    property value: Number defaults 1
end

create container Child extends Parent:
    property value: Number defaults 2
end
"#,
    )
    .unwrap_or_else(|failure| {
        panic!(
            "an invariant same-type override should remain valid: {:?}",
            failure.into_diagnostics()
        )
    });
}

#[test]
fn cyclic_container_inheritance_is_rejected_without_walking_forever() {
    let diagnostics = typecheck(
        r#"
create container First extends Second:
end

create container Second extends First:
end
"#,
    )
    .expect_err("a cyclic parent chain has no valid container contract")
    .into_diagnostics();

    assert!(
        diagnostics.iter().any(|error| {
            error.message.to_lowercase().contains("cyclic")
                && error.message.contains("First")
                && error.message.contains("Second")
        }),
        "expected a cyclic-inheritance diagnostic: {diagnostics:?}"
    );
}

#[test]
fn method_assignments_must_preserve_declared_property_types() {
    for source in [
        r#"
create container Counter:
    property total: Number defaults 1

    action reset:
        change total to nothing
    end
end
"#,
        r#"
create container Counter:
    property total: Number defaults 1

    action reset:
        change total to "wrong"
    end
end
"#,
        r#"
create container Counter:
    property total: Number defaults 1

    action reset:
        store total as "wrong"
    end
end
"#,
        r#"
create container Counter:
    static property total: Number defaults 1

    static action reset:
        change total to nothing
    end
end
"#,
        r#"
create container Counter:
    static property total: Number defaults 1

    static action set needs value: Any:
        change total to value
    end
end
"#,
    ] {
        let diagnostics = typecheck(source)
            .expect_err("a method must not invalidate a declared property type")
            .into_diagnostics();
        assert!(
            diagnostics.iter().any(|error| {
                error.message.contains("property")
                    && error.message.contains("total")
                    && error.message.contains("Number")
            }),
            "expected a declared-property assignment diagnostic: {diagnostics:?}"
        );
    }
}

#[test]
fn method_assignments_accept_values_that_preserve_property_contracts() {
    typecheck(
        r#"
create container Counter:
    property total: Number defaults 1

    action set needs value: Number:
        change total to value
    end

    static property shared_total: Number defaults 1

    static action set_shared needs value: Number:
        change shared_total to value
    end
end
"#,
    )
    .unwrap_or_else(|failure| {
        panic!(
            "compatible instance/static property assignments should type-check: {:?}",
            failure.into_diagnostics()
        )
    });
}

#[test]
fn method_cannot_redeclare_an_existing_property_as_a_constant() {
    let diagnostics = typecheck(
        r#"
create container Counter:
    property total: Number defaults 1

    action reset:
        store new constant total as 2
    end
end
"#,
    )
    .expect_err("runtime rejects constant shadowing of a property binding")
    .into_diagnostics();

    assert!(
        diagnostics.iter().any(|error| {
            error.message.contains("constant")
                && error.message.contains("property")
                && error.message.contains("total")
        }),
        "expected a constant/property redeclaration diagnostic: {diagnostics:?}"
    );
}

#[test]
fn static_methods_can_call_their_own_container_members() {
    typecheck(
        r#"
create container Counter:
    static property total: Number defaults 0

    static action increment: Number
        change total to total plus 1
        return total
    end

    static action increment_twice: Number
        store first as Counter.increment()
        return Counter.increment()
    end
end
"#,
    )
    .unwrap_or_else(|failure| {
        panic!(
            "the active container name must resolve inside its own static methods: {:?}",
            failure.into_diagnostics()
        )
    });
}

#[test]
fn method_list_mutations_preserve_declared_property_element_types() {
    let items_property = PropertyDefinition {
        name: "items".to_string(),
        property_type: Some(Type::List(Box::new(Type::Text))),
        default_value: Some(Expression::Literal(Literal::List(vec![]), 1, 1)),
        validation_rules: vec![],
        visibility: Visibility::Public,
        is_static: false,
        line: 1,
        column: 1,
    };
    let messages_container = |methods| Statement::ContainerDefinition {
        name: "Messages".to_string(),
        extends: None,
        implements: vec![],
        properties: vec![items_property.clone()],
        methods,
        events: vec![],
        static_properties: vec![],
        static_methods: vec![],
        line: 1,
        column: 1,
    };

    let diagnostics = check(Program {
        statements: vec![messages_container(vec![method(
            "corrupt",
            vec![],
            vec![Statement::PushStatement {
                list: Expression::Variable("items".to_string(), 2, 1),
                value: number(1),
                line: 2,
                column: 1,
            }],
        )])],
    })
    .expect_err("mutating a typed property list must preserve its element type")
    .into_diagnostics();
    assert!(
        diagnostics.iter().any(|error| {
            error.message.contains("property")
                && error.message.contains("items")
                && error.message.contains("Text")
        }),
        "expected a typed-property list mutation diagnostic: {diagnostics:?}"
    );

    check(Program {
        statements: vec![messages_container(vec![
            method(
                "append",
                vec![parameter("message", Type::Text)],
                vec![Statement::PushStatement {
                    list: Expression::Variable("items".to_string(), 2, 1),
                    value: Expression::Variable("message".to_string(), 2, 1),
                    line: 2,
                    column: 1,
                }],
            ),
            method(
                "reset",
                vec![],
                vec![Statement::Assignment {
                    name: "items".to_string(),
                    value: Expression::Literal(Literal::List(vec![]), 3, 1),
                    line: 3,
                    column: 1,
                }],
            ),
        ])],
    })
    .unwrap_or_else(|failure| {
        panic!(
            "a compatible typed-property list mutation should type-check: {:?}",
            failure.into_diagnostics()
        )
    });
}

#[test]
fn mutating_builtins_preserve_bare_list_property_contracts() {
    for mutation in [
        "store ignored as push of items and 1",
        "store ignored as unshift of items and 1",
        "store ignored as insert_at of items and 0 and 1",
        "store ignored as insertat of items and 0 and 1",
        "store ignored as fill of items and 1",
    ] {
        let source = format!(
            r#"
create container Messages:
    property items: List of Text defaults []

    action corrupt:
        {mutation}
    end
end
"#
        );
        let diagnostics = typecheck(&source)
            .expect_err("a mutating builtin must preserve a bare property's element type")
            .into_diagnostics();
        assert!(
            diagnostics.iter().any(|error| {
                error.message.contains("property")
                    && error.message.contains("items")
                    && error.message.contains("Text")
            }),
            "expected a declared-property mutation diagnostic for `{mutation}`: {diagnostics:?}"
        );
    }
}

#[test]
fn list_property_contracts_follow_instance_inherited_and_static_accesses() {
    for mutation in [
        "push with box.items and 1",
        "store ignored as push of box.items and 1",
        "store ignored as push of Messages.shared_items and 1",
    ] {
        let source = format!(
            r#"
create container BaseMessages:
    property items: List of Text defaults []
end

create container Messages extends BaseMessages:
    static property shared_items: List of Text defaults []
end

create new Messages as box:
end

{mutation}
"#
        );
        let diagnostics = typecheck(&source)
            .expect_err("mutating a property access must preserve its declared element type")
            .into_diagnostics();
        assert!(
            diagnostics.iter().any(|error| {
                error.message.contains("property")
                    && error.message.contains("Text")
                    && (error.message.contains("items") || error.message.contains("shared_items"))
            }),
            "expected a property-access mutation diagnostic for `{mutation}`: {diagnostics:?}"
        );
    }
}

#[test]
fn method_properties_shadow_outer_bindings_but_not_parameters() {
    typecheck(
        r#"
store total as "outer text"

create container Counter:
    property total: Number defaults 1

    action increment:
        try:
            change total to total plus 1
        when error:
            display "unexpected"
        end try
    end

    action echo needs total: Text: Text
        try:
            return touppercase of total
        when error:
            return "unexpected"
        end try
    end
end
"#,
    )
    .unwrap_or_else(|failure| {
        panic!(
            "runtime lookup is parameter, then property, then outer lexical binding: {:?}",
            failure.into_diagnostics()
        )
    });
}

#[test]
fn declared_nothing_property_does_not_widen_on_assignment() {
    let diagnostics = typecheck(
        r#"
create container State:
    property value: Nothing defaults nothing

    action corrupt:
        change value to 1
    end
end
"#,
    )
    .expect_err("a concrete Nothing property must retain its declared contract")
    .into_diagnostics();

    assert!(
        diagnostics.iter().any(|error| {
            error.message.contains("property")
                && error.message.contains("value")
                && error.message.contains("Nothing")
        }),
        "expected a declared-property assignment diagnostic: {diagnostics:?}"
    );
}

fn number(value: i64) -> Expression {
    Expression::Literal(Literal::Integer(value), 1, 1)
}

fn text(value: &str) -> Expression {
    Expression::Literal(Literal::String(value.into()), 1, 1)
}

fn parameter(name: &str, param_type: Type) -> Parameter {
    Parameter {
        name: name.to_string(),
        param_type: Some(param_type),
        default_value: None,
        line: 1,
        column: 1,
    }
}

fn method(name: &str, parameters: Vec<Parameter>, body: Vec<Statement>) -> Statement {
    Statement::ActionDefinition {
        name: name.to_string(),
        parameters,
        body,
        return_type: None,
        line: 1,
        column: 1,
    }
}

fn container(
    name: &str,
    extends: Option<&str>,
    methods: Vec<Statement>,
    static_methods: Vec<Statement>,
    events: Vec<EventDefinition>,
) -> Statement {
    Statement::ContainerDefinition {
        name: name.to_string(),
        extends: extends.map(str::to_string),
        implements: vec![],
        properties: vec![],
        methods,
        events,
        static_properties: vec![],
        static_methods,
        line: 1,
        column: 1,
    }
}

fn parent_call(method_name: &str, arguments: Vec<Expression>) -> Statement {
    Statement::ParentMethodCall {
        method_name: method_name.to_string(),
        arguments: arguments
            .into_iter()
            .map(|value| Argument { name: None, value })
            .collect(),
        line: 1,
        column: 1,
    }
}

fn instantiate(container_type: &str, arguments: Vec<Expression>) -> Statement {
    Statement::ContainerInstantiation {
        container_type: container_type.to_string(),
        instance_name: "instance".to_string(),
        arguments: arguments
            .into_iter()
            .map(|value| Argument { name: None, value })
            .collect(),
        property_initializers: vec![],
        line: 1,
        column: 1,
    }
}

fn check(program: Program) -> Result<(), TypeCheckError> {
    TypeChecker::new().check_types(&program)
}

#[test]
fn constructor_arguments_match_the_direct_initialize_method() {
    let widget = container(
        "Widget",
        None,
        vec![method(
            "initialize",
            vec![parameter("amount", Type::Number)],
            vec![],
        )],
        vec![],
        vec![],
    );

    check(Program {
        statements: vec![widget.clone(), instantiate("Widget", vec![number(1)])],
    })
    .expect("a matching direct initialize method accepts constructor arguments");

    let diagnostics = check(Program {
        statements: vec![widget, instantiate("Widget", vec![text("wrong")])],
    })
    .expect_err("constructor argument types must match initialize")
    .into_diagnostics();
    assert!(
        diagnostics.iter().any(|error| {
            error.message.contains("initialize")
                && error.message.contains("Number")
                && error.message.contains("Text")
        }),
        "expected an initialize argument diagnostic: {diagnostics:?}"
    );
}

#[test]
fn constructor_arguments_require_a_direct_initialize_method() {
    let diagnostics = check(Program {
        statements: vec![
            container("Widget", None, vec![], vec![], vec![]),
            instantiate("Widget", vec![number(1)]),
        ],
    })
    .expect_err("runtime rejects constructor arguments without initialize")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("initialize") && error.message.contains("Widget")),
        "expected a missing initialize diagnostic: {diagnostics:?}"
    );

    let parent = container(
        "Parent",
        None,
        vec![method(
            "initialize",
            vec![parameter("amount", Type::Number)],
            vec![],
        )],
        vec![],
        vec![],
    );
    let child = container("Child", Some("Parent"), vec![], vec![], vec![]);
    assert!(
        check(Program {
            statements: vec![parent, child, instantiate("Child", vec![number(1)])],
        })
        .is_err(),
        "runtime does not inherit initialize methods"
    );
}

#[test]
fn parent_calls_require_an_instance_method_and_direct_parent_contract() {
    let diagnostics = check(Program {
        statements: vec![parent_call("run", vec![])],
    })
    .expect_err("parent calls are invalid outside container instance methods")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("instance method")),
        "expected an instance-method-context diagnostic: {diagnostics:?}"
    );

    let parent = container(
        "Parent",
        None,
        vec![method(
            "receive",
            vec![parameter("amount", Type::Number)],
            vec![],
        )],
        vec![],
        vec![],
    );
    let child = container(
        "Child",
        Some("Parent"),
        vec![method("run", vec![], vec![parent_call("receive", vec![])])],
        vec![],
        vec![],
    );
    let diagnostics = check(Program {
        statements: vec![parent, child],
    })
    .expect_err("parent calls must match direct-parent arity")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("receive") && error.message.contains("1")),
        "expected a parent method arity diagnostic: {diagnostics:?}"
    );
}

#[test]
fn parent_calls_validate_arguments_and_reject_static_contexts() {
    let parent = container(
        "Parent",
        None,
        vec![method(
            "receive",
            vec![parameter("amount", Type::Number)],
            vec![],
        )],
        vec![],
        vec![],
    );
    let child_with_bad_type = container(
        "Child",
        Some("Parent"),
        vec![method(
            "run",
            vec![],
            vec![parent_call("receive", vec![text("wrong")])],
        )],
        vec![],
        vec![],
    );
    let diagnostics = check(Program {
        statements: vec![parent.clone(), child_with_bad_type],
    })
    .expect_err("parent call arguments must match the parent method")
    .into_diagnostics();
    assert!(
        diagnostics.iter().any(|error| {
            error.message.contains("receive")
                && error.message.contains("Number")
                && error.message.contains("Text")
        }),
        "expected a parent method argument diagnostic: {diagnostics:?}"
    );

    let child_with_static_call = container(
        "StaticChild",
        Some("Parent"),
        vec![],
        vec![method(
            "run",
            vec![],
            vec![parent_call("receive", vec![number(1)])],
        )],
        vec![],
    );
    let diagnostics = check(Program {
        statements: vec![parent, child_with_static_call],
    })
    .expect_err("runtime has no `this` in a static method")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("static method")),
        "expected a static-method parent-call diagnostic: {diagnostics:?}"
    );
}

fn event(name: &str, parameters: Vec<Parameter>) -> EventDefinition {
    EventDefinition {
        name: name.to_string(),
        parameters,
        line: 1,
        column: 1,
    }
}

#[test]
fn event_handlers_validate_sources_events_and_parameter_scope() {
    let source_container = container(
        "Emitter",
        None,
        vec![],
        vec![],
        vec![event("changed", vec![parameter("amount", Type::Number)])],
    );
    let instance = instantiate("Emitter", vec![]);
    let valid_handler = Statement::EventHandler {
        event_name: "changed".to_string(),
        event_source: Some(Expression::Variable("instance".to_string(), 2, 1)),
        handler_body: vec![Statement::DisplayStatement {
            value: Expression::Variable("amount".to_string(), 3, 1),
            line: 3,
            column: 1,
        }],
        line: 2,
        column: 1,
    };
    check(Program {
        statements: vec![source_container.clone(), instance.clone(), valid_handler],
    })
    .expect("event parameters should be in scope in a valid handler");

    let diagnostics = check(Program {
        statements: vec![
            source_container,
            instance,
            Statement::EventHandler {
                event_name: "missing".to_string(),
                event_source: Some(Expression::Variable("instance".to_string(), 2, 1)),
                handler_body: vec![],
                line: 2,
                column: 1,
            },
        ],
    })
    .expect_err("runtime rejects events absent from the direct container")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("missing") && error.message.contains("Emitter")),
        "expected an unknown-event diagnostic: {diagnostics:?}"
    );

    let diagnostics = check(Program {
        statements: vec![Statement::EventHandler {
            event_name: "changed".to_string(),
            event_source: Some(number(1)),
            handler_body: vec![],
            line: 1,
            column: 1,
        }],
    })
    .expect_err("runtime rejects non-container handler sources")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("non-container")),
        "expected a handler-source diagnostic: {diagnostics:?}"
    );
}

#[test]
fn event_triggers_visit_arguments_and_check_overlapping_parameter_types() {
    let definition = Statement::EventDefinition {
        name: "changed".to_string(),
        parameters: vec![parameter("amount", Type::Number)],
        line: 1,
        column: 1,
    };

    check(Program {
        statements: vec![
            definition.clone(),
            Statement::EventTrigger {
                name: "changed".to_string(),
                arguments: vec![],
                line: 2,
                column: 1,
            },
        ],
    })
    .expect("runtime fills missing event parameters with Nothing");

    let diagnostics = check(Program {
        statements: vec![
            definition.clone(),
            Statement::EventTrigger {
                name: "changed".to_string(),
                arguments: vec![Argument {
                    name: None,
                    value: text("wrong"),
                }],
                line: 2,
                column: 1,
            },
        ],
    })
    .expect_err("provided event arguments should match overlapping parameters")
    .into_diagnostics();
    assert!(
        diagnostics.iter().any(|error| {
            error.message.contains("changed")
                && error.message.contains("Number")
                && error.message.contains("Text")
        }),
        "expected an event argument diagnostic: {diagnostics:?}"
    );

    let invalid_extra = Expression::BinaryOperation {
        left: Box::new(number(1)),
        operator: wfl::parser::ast::Operator::Minus,
        right: Box::new(text("wrong")),
        line: 2,
        column: 1,
    };
    let diagnostics = check(Program {
        statements: vec![
            definition,
            Statement::EventTrigger {
                name: "changed".to_string(),
                arguments: vec![
                    Argument {
                        name: None,
                        value: number(1),
                    },
                    Argument {
                        name: None,
                        value: invalid_extra,
                    },
                ],
                line: 2,
                column: 1,
            },
        ],
    })
    .expect_err("extra event arguments are ignored only after evaluation")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("Cannot perform Minus")),
        "every trigger argument must be traversed: {diagnostics:?}"
    );
}

#[test]
fn container_methods_resolve_their_own_declared_events() {
    let valid = container(
        "Emitter",
        None,
        vec![method(
            "emit",
            vec![],
            vec![Statement::EventTrigger {
                name: "changed".to_string(),
                arguments: vec![Argument {
                    name: None,
                    value: number(1),
                }],
                line: 1,
                column: 1,
            }],
        )],
        vec![],
        vec![event("changed", vec![parameter("amount", Type::Number)])],
    );
    check(Program {
        statements: vec![valid],
    })
    .expect("container methods receive their direct container events at runtime");
}
