//! Regression coverage for type-state joins from `try` success/error endpoints
//! into `finally`, while keeping `when` error aliases clause-local.

use std::sync::Arc;
use wfl::Interpreter;
use wfl::analyzer::{Analyzer, Symbol, SymbolKind};
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{
    ErrorType, Expression, FileOpenMode, Literal, Operator, Program, Statement, Type, WhenClause,
};
use wfl::typechecker::TypeChecker;

fn text_literal(value: &str) -> Expression {
    Expression::Literal(Literal::String(Arc::from(value)), 1, 1)
}

fn stream_binding() -> Statement {
    Statement::StartStreamingResponseStatement {
        request: Expression::Variable("request".to_string(), 3, 1),
        status: Some(Expression::Literal(Literal::Integer(200), 3, 1)),
        content_type: None,
        headers: None,
        variable_name: "out".to_string(),
        line: 3,
        column: 1,
    }
}

fn display_text(value: &str, line: usize) -> Statement {
    Statement::DisplayStatement {
        value: text_literal(value),
        line,
        column: 1,
    }
}

fn flush_out() -> Statement {
    Statement::FlushStreamStatement {
        target: Expression::Variable("out".to_string(), 5, 1),
        legacy_binding: None,
        action_fallback: None,
        line: 5,
        column: 1,
    }
}

fn subtract_one(name: &str, line: usize) -> Statement {
    Statement::DisplayStatement {
        value: Expression::BinaryOperation {
            left: Box::new(Expression::Variable(name.to_string(), line, 1)),
            operator: Operator::Minus,
            right: Box::new(Expression::Literal(Literal::Integer(1), line, 1)),
            line,
            column: 1,
        },
        line,
        column: 1,
    }
}

fn store_text(name: &str, value: &str, line: usize) -> Statement {
    Statement::VariableDeclaration {
        name: name.to_string(),
        value: text_literal(value),
        is_constant: false,
        line,
        column: 1,
    }
}

fn display_variable(name: &str, line: usize) -> Statement {
    Statement::DisplayStatement {
        value: Expression::Variable(name.to_string(), line, 1),
        line,
        column: 1,
    }
}

#[test]
fn handler_response_stream_state_is_joined_before_finally() {
    let mut analyzer = Analyzer::new();
    analyzer
        .define_symbol(Symbol {
            name: "request".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Custom("Request".to_string())),
            line: 1,
            column: 1,
        })
        .expect("define request binding");
    let program = Program {
        statements: vec![
            Statement::OpenFileStatement {
                path: text_literal("unused.txt"),
                variable_name: "out".to_string(),
                mode: FileOpenMode::Write,
                line: 1,
                column: 1,
            },
            Statement::TryStatement {
                body: vec![display_text("success", 2)],
                when_clauses: vec![WhenClause {
                    error_type: ErrorType::General,
                    error_name: "caught".to_string(),
                    body: vec![stream_binding()],
                }],
                otherwise_block: None,
                finally_block: Some(vec![flush_out()]),
                line: 2,
                column: 1,
            },
        ],
    };

    let result = TypeChecker::with_analyzer(analyzer).check_types(&program);
    assert!(
        result.is_ok(),
        "finally must see the gradual join of the successful File path and the handler's \
         ResponseStream path; errors: {:?}",
        result.err()
    );
}

#[test]
fn handler_error_aliases_remain_clause_local_before_finally() {
    let mut analyzer = Analyzer::new();
    for name in ["caught", "error_message"] {
        analyzer
            .define_symbol(Symbol {
                name: name.to_string(),
                kind: SymbolKind::Variable { mutable: true },
                symbol_type: Some(Type::Number),
                line: 1,
                column: 1,
            })
            .expect("define outer Number binding");
    }

    let program = Program {
        statements: vec![Statement::TryStatement {
            body: vec![display_text("success", 2)],
            when_clauses: vec![WhenClause {
                error_type: ErrorType::General,
                error_name: "caught".to_string(),
                body: vec![
                    Statement::DisplayStatement {
                        value: Expression::Variable("caught".to_string(), 3, 1),
                        line: 3,
                        column: 1,
                    },
                    Statement::DisplayStatement {
                        value: Expression::Variable("error_message".to_string(), 3, 1),
                        line: 3,
                        column: 1,
                    },
                ],
            }],
            otherwise_block: None,
            finally_block: Some(vec![
                subtract_one("caught", 5),
                subtract_one("error_message", 6),
            ]),
            line: 2,
            column: 1,
        }],
    };

    let result = TypeChecker::with_analyzer(analyzer).check_types(&program);
    assert!(
        result.is_ok(),
        "finally must resolve the outer Number bindings, not clause-local Text aliases; \
         got: {:?}",
        result.err()
    );
}

#[test]
fn handler_only_binding_is_not_definitely_available_in_finally() {
    let program = Program {
        statements: vec![Statement::TryStatement {
            body: vec![display_text("success", 2)],
            when_clauses: vec![WhenClause {
                error_type: ErrorType::FileNotFound,
                error_name: "caught".to_string(),
                body: vec![store_text("cleanup_message", "handled", 3)],
            }],
            otherwise_block: None,
            finally_block: Some(vec![display_variable("cleanup_message", 5)]),
            line: 1,
            column: 1,
        }],
    };

    let outcome = TypeChecker::new().check_types(&program);
    assert!(
        outcome.is_err(),
        "a handler-only binding is absent on the successful and unmatched-error paths, so \
         finally must reject it as potentially unbound"
    );
}

#[test]
fn otherwise_only_binding_is_not_definitely_available_in_finally() {
    let program = Program {
        statements: vec![Statement::TryStatement {
            body: vec![display_text("success", 2)],
            when_clauses: vec![],
            otherwise_block: Some(vec![store_text("cleanup_message", "otherwise", 3)]),
            finally_block: Some(vec![display_variable("cleanup_message", 5)]),
            line: 1,
            column: 1,
        }],
    };

    let outcome = TypeChecker::new().check_types(&program);
    assert!(
        outcome.is_err(),
        "an otherwise-only binding is absent on the successful path, so finally must reject \
         it as potentially unbound"
    );
}

#[test]
fn partially_initialized_body_binding_is_not_available_in_handler() {
    let program = parse(
        r#"
store divisor as 0
try:
    store maybe_value as 1 divided by divisor
when error:
    display maybe_value
end try
"#,
    );

    let outcome = TypeChecker::new().check_types(&program);
    assert!(
        outcome.is_err(),
        "a handler can run when a body declaration's initializer fails, so that binding must \
         remain unavailable in the handler"
    );
}

#[test]
fn full_pipeline_error_alias_is_clause_local() {
    let program = Program {
        statements: vec![
            Statement::VariableDeclaration {
                name: "caught".to_string(),
                value: Expression::Literal(Literal::Integer(10), 1, 1),
                is_constant: false,
                line: 1,
                column: 1,
            },
            Statement::VariableDeclaration {
                name: "error_message".to_string(),
                value: Expression::Literal(Literal::Integer(20), 1, 1),
                is_constant: false,
                line: 1,
                column: 1,
            },
            Statement::TryStatement {
                body: vec![display_text("success", 2)],
                when_clauses: vec![WhenClause {
                    error_type: ErrorType::General,
                    error_name: "caught".to_string(),
                    body: vec![
                        display_variable("caught", 3),
                        display_variable("error_message", 3),
                    ],
                }],
                otherwise_block: None,
                finally_block: Some(vec![
                    subtract_one("caught", 5),
                    subtract_one("error_message", 6),
                ]),
                line: 2,
                column: 1,
            },
        ],
    };

    assert!(
        TypeChecker::new().check_types(&program).is_ok(),
        "the implicit Text error aliases must shadow only inside their clause, and finally \
         must resolve the outer Numbers; errors: {:?}",
        TypeChecker::new().check_types(&program).err()
    );
}

fn parse(source: &str) -> Program {
    Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .expect("test program should parse")
}

#[tokio::test(flavor = "current_thread")]
async fn runtime_error_aliases_are_removed_before_finally() {
    let program = parse(
        r#"
store caught as 10
store error_message as 20
try:
    store bad as 1 divided by 0
when error as caught:
    display caught
finally:
    store caught_result as caught minus 1
    store message_result as error_message minus 1
end try
"#,
    );

    Interpreter::new()
        .interpret(&program)
        .await
        .expect("finally must resolve the outer numeric bindings");
}

#[tokio::test(flavor = "current_thread")]
async fn runtime_finally_return_overrides_the_primary_result() {
    let program = parse(
        r#"
define action called value_from_finally:
    try:
        display "primary"
    finally:
        return "from finally"
    end try
end action

store result as call value_from_finally
check if result is not equal to "from finally":
    store bad as 1 divided by 0
end check
"#,
    );

    Interpreter::new()
        .interpret(&program)
        .await
        .expect("return from finally must propagate out of the action");
}

#[test]
fn nested_try_error_path_keeps_intermediate_mutation_state() {
    let program = parse(
        r#"
store values as [1]
store divisor as 0
try:
    check if yes:
        store first_fill as fill of values and "text"
        store failure as 1 divided by divisor
        store second_fill as fill of values and 2
    end check
when error:
    store removed as pop of values
    store upper as touppercase of removed
end try
"#,
    );

    TypeChecker::new()
        .check_types(&program)
        .expect("the handler must see a gradual Number/Text element type");
}

#[test]
fn nested_try_error_path_rejects_use_that_ignores_intermediate_scalar_mutation() {
    let program = parse(
        r#"
store value as "start"
store divisor as 0
try:
    check if yes:
        change value to nothing
        store failure as 1 divided by divisor
        change value to "end"
    end check
when error:
    store upper as touppercase of value
end try
"#,
    );

    let outcome = TypeChecker::new().check_types(&program);
    assert!(
        outcome.is_err(),
        "the handler must retain the intermediate Nothing state instead of treating value as \
         definitely Text from the body's endpoint"
    );
}

#[test]
fn try_flow_snapshot_traversal_is_charged_to_the_operation_budget() {
    use wfl::exec::budget::{BudgetLimits, ExecutionBudget};

    let declarations = || {
        (0..160)
            .map(|index| Statement::VariableDeclaration {
                name: format!("value_{index}"),
                value: Expression::Literal(Literal::Integer(index), 1, 1),
                is_constant: false,
                line: 1,
                column: 1,
            })
            .collect::<Vec<_>>()
    };
    let limits = || BudgetLimits {
        max_operations: Some(400),
        ..Default::default()
    };

    let mut control_statements = declarations();
    control_statements.push(display_text("no try capture", 2));
    let control_budget = std::sync::Arc::new(ExecutionBudget::new(limits()));
    {
        let _guard = ExecutionBudget::enter(std::sync::Arc::clone(&control_budget));
        TypeChecker::with_analyzer(Analyzer::new())
            .check_types(&Program {
                statements: control_statements,
            })
            .expect("ordinary checking must fit beneath the cap used to isolate try traversal");
    }

    let mut try_statements = declarations();
    try_statements.push(Statement::TryStatement {
        body: vec![display_text("capture", 2)],
        when_clauses: vec![],
        otherwise_block: None,
        finally_block: None,
        line: 2,
        column: 1,
    });
    let program = Program {
        statements: try_statements,
    };
    let budget = std::sync::Arc::new(ExecutionBudget::new(limits()));
    let _guard = ExecutionBudget::enter(std::sync::Arc::clone(&budget));

    let outcome = TypeChecker::with_analyzer(Analyzer::new()).check_types(&program);
    assert!(
        matches!(outcome, Err(wfl::typechecker::TypeCheckError::Budget(_))),
        "walking live binding/alias state for a try-flow capture must be charged \
         proportionally instead of bypassing the run budget; charged {}, got: {outcome:?}",
        budget.operations_charged()
    );
}
