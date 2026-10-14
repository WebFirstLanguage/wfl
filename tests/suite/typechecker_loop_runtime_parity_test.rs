use wfl::parser::ast::{Expression, Literal, Program, Statement};
use wfl::typechecker::{TypeCheckError, TypeChecker};

fn boolean(value: bool) -> Expression {
    Expression::Literal(Literal::Boolean(value), 1, 1)
}

fn number(value: i64) -> Expression {
    Expression::Literal(Literal::Integer(value), 1, 1)
}

fn variable(name: &str) -> Expression {
    Expression::Variable(name.to_string(), 1, 1)
}

fn declaration(name: &str, value: Expression, is_constant: bool) -> Statement {
    Statement::VariableDeclaration {
        name: name.to_string(),
        value,
        is_constant,
        line: 1,
        column: 1,
    }
}

fn assignment(name: &str, value: Expression) -> Statement {
    Statement::Assignment {
        name: name.to_string(),
        value,
        line: 1,
        column: 1,
    }
}

fn messages(statements: Vec<Statement>) -> Vec<String> {
    TypeChecker::new()
        .check_types(&Program { statements })
        .expect_err("program should be rejected")
        .into_diagnostics()
        .into_iter()
        .map(|error| error.message)
        .collect()
}

fn typecheck(statements: Vec<Statement>) -> Result<(), TypeCheckError> {
    TypeChecker::new().check_types(&Program { statements })
}

fn invalid_first_iteration_body() -> Vec<Statement> {
    vec![
        Statement::WaitForDurationStatement {
            duration: variable("changing"),
            unit: "milliseconds".to_string(),
            line: 1,
            column: 1,
        },
        assignment("changing", number(1)),
    ]
}

#[test]
fn persistent_loops_preserve_diagnostics_from_the_first_iteration() {
    let loops = [
        Statement::WhileLoop {
            condition: boolean(true),
            body: invalid_first_iteration_body(),
            line: 1,
            column: 1,
        },
        Statement::RepeatWhileLoop {
            condition: boolean(true),
            body: invalid_first_iteration_body(),
            line: 1,
            column: 1,
        },
        Statement::RepeatUntilLoop {
            condition: boolean(false),
            body: invalid_first_iteration_body(),
            line: 1,
            column: 1,
        },
    ];

    for loop_statement in loops {
        let diagnostics = messages(vec![
            declaration(
                "changing",
                Expression::Literal(Literal::Nothing, 1, 1),
                false,
            ),
            loop_statement,
        ]);
        assert!(
            diagnostics
                .iter()
                .any(|message| message.contains("number for wait duration")),
            "the deterministic first-iteration Number mismatch must survive fixed-point checking: \
             {diagnostics:?}"
        );
    }
}

#[test]
fn constants_in_persistent_loop_environments_cannot_be_redeclared() {
    let constant_body = || vec![declaration("per_iteration", number(1), true)];
    let loops = [
        Statement::WhileLoop {
            condition: boolean(true),
            body: constant_body(),
            line: 1,
            column: 1,
        },
        Statement::RepeatWhileLoop {
            condition: boolean(true),
            body: constant_body(),
            line: 1,
            column: 1,
        },
        Statement::RepeatUntilLoop {
            condition: boolean(false),
            body: constant_body(),
            line: 1,
            column: 1,
        },
    ];

    for loop_statement in loops {
        let diagnostics = messages(vec![loop_statement]);
        assert!(
            diagnostics.iter().any(|message| {
                message.to_lowercase().contains("constant") && message.contains("per_iteration")
            }),
            "a second iteration would redeclare the same runtime constant: {diagnostics:?}"
        );
    }
}

#[test]
fn nested_persistent_loops_inherit_outer_reentry_state() {
    let nested_loop = Statement::WhileLoop {
        condition: boolean(true),
        body: vec![
            declaration("nested_constant", number(1), true),
            Statement::BreakStatement { line: 1, column: 1 },
        ],
        line: 1,
        column: 1,
    };
    let outer_loop = Statement::WhileLoop {
        condition: boolean(true),
        body: vec![nested_loop],
        line: 1,
        column: 1,
    };

    let diagnostics = messages(vec![outer_loop]);
    assert!(
        diagnostics.iter().any(|message| {
            message.to_lowercase().contains("constant") && message.contains("nested_constant")
        }),
        "a nested persistent scope survives outer-loop re-entry: {diagnostics:?}"
    );
}

#[test]
fn constants_in_fresh_iteration_environments_remain_valid() {
    let constant_body = || vec![declaration("per_iteration", number(1), true)];
    let loops = [
        Statement::ForEachLoop {
            item_name: "item".to_string(),
            collection: Expression::Literal(Literal::List(vec![number(1), number(2)]), 1, 1),
            reversed: false,
            body: constant_body(),
            line: 1,
            column: 1,
        },
        Statement::CountLoop {
            start: number(1),
            end: number(2),
            step: None,
            downward: false,
            variable_name: None,
            body: constant_body(),
            line: 1,
            column: 1,
        },
        Statement::ForeverLoop {
            body: constant_body(),
            line: 1,
            column: 1,
        },
        Statement::MainLoop {
            body: constant_body(),
            concurrent: false,
            line: 1,
            column: 1,
        },
    ];

    for loop_statement in loops {
        typecheck(vec![loop_statement])
            .expect("the runtime creates or clears the loop child environment every iteration");
    }
}

#[test]
fn constants_are_valid_when_a_persistent_loop_cannot_reach_a_second_iteration() {
    let constant_body = || vec![declaration("single_iteration", number(1), true)];
    let loops = [
        Statement::WhileLoop {
            condition: boolean(false),
            body: constant_body(),
            line: 1,
            column: 1,
        },
        Statement::RepeatWhileLoop {
            condition: boolean(false),
            body: constant_body(),
            line: 1,
            column: 1,
        },
        Statement::RepeatUntilLoop {
            condition: boolean(true),
            body: constant_body(),
            line: 1,
            column: 1,
        },
    ];

    for loop_statement in loops {
        typecheck(vec![loop_statement])
            .expect("a statically zero/one-iteration loop cannot redeclare its constant");
    }
}

#[test]
fn pre_test_loops_preserve_diagnostics_from_the_first_condition_check() {
    let loops = [
        Statement::WhileLoop {
            condition: variable("condition_value"),
            body: vec![assignment("condition_value", boolean(true))],
            line: 1,
            column: 1,
        },
        Statement::RepeatWhileLoop {
            condition: variable("condition_value"),
            body: vec![assignment("condition_value", boolean(true))],
            line: 1,
            column: 1,
        },
    ];

    for loop_statement in loops {
        let diagnostics = messages(vec![
            declaration(
                "condition_value",
                Expression::Literal(Literal::Nothing, 1, 1),
                false,
            ),
            loop_statement,
        ]);
        assert!(
            diagnostics
                .iter()
                .any(|message| message.to_lowercase().contains("boolean")),
            "the first condition is evaluated before the body can widen its type: \
             {diagnostics:?}"
        );
    }
}

#[test]
fn statically_false_pre_test_loops_do_not_apply_unreachable_body_types() {
    let loops = [
        Statement::WhileLoop {
            condition: boolean(false),
            body: vec![assignment("duration", number(1))],
            line: 1,
            column: 1,
        },
        Statement::RepeatWhileLoop {
            condition: boolean(false),
            body: vec![assignment("duration", number(1))],
            line: 1,
            column: 1,
        },
    ];

    for loop_statement in loops {
        let diagnostics = messages(vec![
            declaration(
                "duration",
                Expression::Literal(Literal::Nothing, 1, 1),
                false,
            ),
            loop_statement,
            Statement::WaitForDurationStatement {
                duration: variable("duration"),
                unit: "milliseconds".to_string(),
                line: 1,
                column: 1,
            },
        ]);
        assert!(
            diagnostics
                .iter()
                .any(|message| message.contains("number for wait duration")),
            "a statically unreachable body must not widen the post-loop binding: \
             {diagnostics:?}"
        );
    }
}
