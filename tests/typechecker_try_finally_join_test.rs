//! Regression coverage for type-state joins from `try` success/error endpoints
//! into `finally`, while keeping `when` error aliases clause-local.

use std::sync::Arc;
use wfl::analyzer::{Analyzer, Symbol, SymbolKind};
use wfl::parser::ast::{
    ErrorType, Expression, FileOpenMode, Literal, Operator, Program, Statement, Type, WhenClause,
};
use wfl::typechecker::TypeChecker;

fn text_literal(value: &str) -> Expression {
    Expression::Literal(Literal::String(Arc::from(value)), 1, 1)
}

fn stream_binding() -> Statement {
    Statement::StartStreamingResponseStatement {
        request: text_literal("request"),
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

    assert!(
        TypeChecker::new().check_types(&program).is_ok(),
        "finally must see the gradual join of the successful File path and the handler's \
         ResponseStream path; errors: {:?}",
        TypeChecker::new().check_types(&program).err()
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
fn handler_created_binding_is_semantically_visible_in_finally() {
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

    assert!(
        TypeChecker::new().check_types(&program).is_ok(),
        "the analyzer and checker must preserve an ordinary handler binding in the shared \
         runtime try scope until finally; errors: {:?}",
        TypeChecker::new().check_types(&program).err()
    );
}

#[test]
fn otherwise_created_binding_is_semantically_visible_in_finally() {
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

    assert!(
        TypeChecker::new().check_types(&program).is_ok(),
        "the analyzer and checker must preserve an ordinary otherwise binding in the shared \
         runtime try scope until finally; errors: {:?}",
        TypeChecker::new().check_types(&program).err()
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
            Statement::TryStatement {
                body: vec![display_text("success", 2)],
                when_clauses: vec![WhenClause {
                    error_type: ErrorType::General,
                    error_name: "caught".to_string(),
                    body: vec![display_variable("caught", 3)],
                }],
                otherwise_block: None,
                finally_block: Some(vec![subtract_one("caught", 5)]),
                line: 2,
                column: 1,
            },
        ],
    };

    assert!(
        TypeChecker::new().check_types(&program).is_ok(),
        "the implicit Text error alias must shadow only inside its clause, and finally must \
         resolve the outer Number; errors: {:?}",
        TypeChecker::new().check_types(&program).err()
    );
}
