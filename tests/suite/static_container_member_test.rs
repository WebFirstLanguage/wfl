use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::{
    Parser,
    ast::{Argument, Expression, Literal, Program, Statement},
};
use wfl::typechecker::TypeChecker;

fn parse(source: &str) -> Program {
    let tokens = lex_wfl_with_positions(source);
    Parser::new(&tokens)
        .parse()
        .unwrap_or_else(|error| panic!("parse failed: {error:?}"))
}

fn typecheck(source: &str) -> Result<Program, String> {
    let program = parse(source);
    TypeChecker::new()
        .check_types(&program)
        .map_err(|error| {
            error
                .into_diagnostics()
                .into_iter()
                .map(|diagnostic| diagnostic.message)
                .collect::<Vec<_>>()
                .join("; ")
        })
        .map(|()| program)
}

#[tokio::test]
async fn parsed_static_properties_and_methods_work_end_to_end() {
    let program = typecheck(
        r#"
create container Counter:
    static property total: Number defaults 41

    static action answer: Number
        return total plus 1
    end

    static action increment: Number
        change total to total plus 1
        return total
    end
end

store property_value as Counter.total
store method_value as Counter.answer()
store incremented_value as Counter.increment()
store persisted_value as Counter.total
"#,
    )
    .unwrap_or_else(|error| panic!("typecheck failed: {error}"));

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .unwrap_or_else(|errors| panic!("runtime failed: {errors:?}"));

    for name in [
        "property_value",
        "method_value",
        "incremented_value",
        "persisted_value",
    ] {
        assert_eq!(
            interpreter.global_env().borrow().get(name),
            Some(Value::Number(if name == "property_value" {
                41.0
            } else {
                42.0
            }))
        );
    }
}

#[tokio::test]
async fn inherited_static_members_match_the_static_checker() {
    let program = typecheck(
        r#"
create container Parent:
    static property base_value: Number defaults 7

    static action base_answer: Number
        return base_value
    end
end

create container Child extends Parent:
end

store inherited_property as Child.base_value
store inherited_method as Child.base_answer()
"#,
    )
    .unwrap_or_else(|error| panic!("typecheck failed: {error}"));

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .unwrap_or_else(|errors| panic!("runtime failed: {errors:?}"));

    for name in ["inherited_property", "inherited_method"] {
        assert_eq!(
            interpreter.global_env().borrow().get(name),
            Some(Value::Number(7.0))
        );
    }
}

#[tokio::test]
async fn legacy_static_member_ast_uses_the_same_inheritance_rules() {
    let mut program = parse(
        r#"
create container Parent:
    static property base_value: Number defaults 7
end

create container Child extends Parent:
end
"#,
    );
    program.statements.push(Statement::VariableDeclaration {
        name: "legacy_value".to_string(),
        value: Expression::StaticMemberAccess {
            container: "Child".to_string(),
            member: "base_value".to_string(),
            line: 8,
            column: 1,
        },
        is_constant: false,
        line: 8,
        column: 1,
    });
    TypeChecker::new()
        .check_types(&program)
        .expect("legacy static-member AST should follow inheritance");

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .unwrap_or_else(|errors| panic!("runtime failed: {errors:?}"));
    assert_eq!(
        interpreter.global_env().borrow().get("legacy_value"),
        Some(Value::Number(7.0))
    );
}

#[test]
fn static_property_defaults_follow_their_declared_types() {
    let error = typecheck(
        r#"
create container Counter:
    static property total: Number defaults "not a number"
end
"#,
    )
    .expect_err("a mismatched static default must be rejected");
    assert!(
        error.contains("Default value type"),
        "expected a static-property default diagnostic, got: {error}"
    );
}

#[test]
fn static_methods_cannot_read_instance_properties_as_bare_names() {
    let error = typecheck(
        r#"
create container Counter:
    property instance_total: Number defaults 1

    static action invalid: Number
        return instance_total
    end
end
"#,
    )
    .expect_err("a static method has no instance property environment");
    assert!(
        error.contains("instance_total"),
        "expected an undefined instance-property diagnostic, got: {error}"
    );
}

#[tokio::test]
async fn stored_static_method_reference_keeps_environment_and_persists_mutation() {
    let mut program = parse(
        r#"
create container Counter:
    static property total: Number defaults 0

    static action increment_by needs amount: Number: Number
        change total to total plus amount
        return total
    end
end
"#,
    );
    program.statements.extend([
        Statement::VariableDeclaration {
            name: "increment_counter".into(),
            value: Expression::StaticMemberAccess {
                container: "Counter".into(),
                member: "increment_by".into(),
                line: 1,
                column: 1,
            },
            is_constant: false,
            line: 1,
            column: 1,
        },
        Statement::VariableDeclaration {
            name: "direct_value".into(),
            value: Expression::MethodCall {
                object: Box::new(Expression::Variable("Counter".into(), 1, 1)),
                method: "increment_by".into(),
                arguments: vec![Argument {
                    name: None,
                    value: Expression::Literal(Literal::Integer(1), 1, 1),
                }],
                line: 1,
                column: 1,
            },
            is_constant: false,
            line: 1,
            column: 1,
        },
        Statement::VariableDeclaration {
            name: "incremented_value".into(),
            value: Expression::FunctionCall {
                function: Box::new(Expression::Variable("increment_counter".into(), 1, 1)),
                arguments: vec![Argument {
                    name: None,
                    value: Expression::Literal(Literal::Integer(1), 1, 1),
                }],
                line: 1,
                column: 1,
            },
            is_constant: false,
            line: 1,
            column: 1,
        },
        Statement::VariableDeclaration {
            name: "persisted_value".into(),
            value: Expression::PropertyAccess {
                object: Box::new(Expression::Variable("Counter".into(), 1, 1)),
                property: "total".into(),
                line: 1,
                column: 1,
            },
            is_constant: false,
            line: 1,
            column: 1,
        },
    ]);
    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .unwrap_or_else(|errors| panic!("runtime failed: {errors:?}"));

    assert_eq!(
        interpreter.global_env().borrow().get("direct_value"),
        Some(Value::Number(1.0))
    );
    for name in ["incremented_value", "persisted_value"] {
        assert_eq!(
            interpreter.global_env().borrow().get(name),
            Some(Value::Number(2.0)),
            "{name} should observe both the direct and stored-reference mutations"
        );
    }
}

#[tokio::test]
async fn reentrant_static_method_calls_share_the_latest_property_state() {
    let program = parse(
        r#"
create container Counter:
    static property total: Number defaults 0

    static action increment: Number
        change total to total plus 1
        return total
    end

    static action set_then_increment: Number
        change total to 5
        store nested_value as Counter.increment()
        return total
    end
end

store returned_value as Counter.set_then_increment()
store persisted_value as Counter.total
"#,
    );

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .unwrap_or_else(|errors| panic!("runtime failed: {errors:?}"));

    for name in ["returned_value", "persisted_value"] {
        assert_eq!(
            interpreter.global_env().borrow().get(name),
            Some(Value::Number(6.0)),
            "{name} should include the nested static method's increment"
        );
    }
}

#[tokio::test]
async fn static_property_mutations_persist_when_a_direct_call_errors() {
    let program = parse(
        r#"
create container Counter:
    static property total: Number defaults 0

    static action mutate_then_fail:
        change total to 7
        store failure as 1 divided by 0
    end
end

Counter.mutate_then_fail()
"#,
    );
    let mut interpreter = Interpreter::new();
    assert!(
        interpreter.interpret(&program).await.is_err(),
        "the method's deliberate divide-by-zero should escape"
    );

    let Some(Value::ContainerDefinition(counter)) =
        interpreter.global_env().borrow().get("Counter")
    else {
        panic!("Counter definition should remain available after the error");
    };
    assert_eq!(
        counter.static_properties.borrow().get("total"),
        Some(&Value::Number(7.0)),
        "mutations completed before an error must not be rolled back"
    );
}

#[tokio::test]
async fn instance_property_mutations_persist_when_a_method_errors() {
    let program = parse(
        r#"
create container Counter:
    property total: Number defaults 0

    action mutate_then_fail:
        change total to 5
        store failure as 1 divided by 0
    end
end

create new Counter as counter:
end
counter.mutate_then_fail()
"#,
    );
    let mut interpreter = Interpreter::new();
    assert!(
        interpreter.interpret(&program).await.is_err(),
        "the method's deliberate divide-by-zero should escape"
    );

    let Some(Value::ContainerInstance(counter)) = interpreter.global_env().borrow().get("counter")
    else {
        panic!("Counter instance should remain available after the error");
    };
    assert_eq!(
        counter.borrow().properties.get("total"),
        Some(&Value::Number(5.0)),
        "mutations completed before an error must not be rolled back"
    );
}

#[tokio::test]
async fn static_property_mutations_persist_when_a_stored_call_errors() {
    let program = parse(
        r#"
create container Counter:
    static property total: Number defaults 0

    static action mutate_then_fail:
        change total to 9
        store failure as 1 divided by 0
    end
end

store failer as Counter.mutate_then_fail
store ignored as failer
"#,
    );
    let mut interpreter = Interpreter::new();
    assert!(
        interpreter.interpret(&program).await.is_err(),
        "the stored method's deliberate divide-by-zero should escape"
    );

    let Some(Value::ContainerDefinition(counter)) =
        interpreter.global_env().borrow().get("Counter")
    else {
        panic!("Counter definition should remain available after the error");
    };
    assert_eq!(
        counter.static_properties.borrow().get("total"),
        Some(&Value::Number(9.0)),
        "first-class static methods must persist pre-error mutations"
    );
}

#[test]
fn stored_zero_argument_static_method_auto_calls_to_its_result_type() {
    typecheck(
        r#"
create container Counter:
    static action answer: Number
        return 42
    end
end

store getter as Counter.answer
store value as getter
store incremented as value plus 1
"#,
    )
    .unwrap_or_else(|error| {
        panic!("stored zero-argument static method should infer Number: {error}")
    });
}
