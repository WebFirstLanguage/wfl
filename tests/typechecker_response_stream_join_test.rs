//! Regression coverage for conservative type-state joins across conditional
//! control flow involving response-stream and file-handle bindings.

use std::sync::Arc;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, FileOpenMode, Literal, Operator, Program, Statement};
use wfl::typechecker::TypeChecker;

fn parse(source: &str) -> Program {
    Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .expect("parse")
}

fn typecheck(program: &Program) -> Result<(), String> {
    TypeChecker::new()
        .check_types(program)
        .map_err(|errors| format!("{errors:?}"))
}

fn bool_literal(value: bool) -> Expression {
    Expression::Literal(Literal::Boolean(value), 2, 1)
}

fn text_literal(value: &str) -> Expression {
    Expression::Literal(Literal::String(Arc::from(value)), 2, 1)
}

fn stream_binding() -> Statement {
    Statement::StartStreamingResponseStatement {
        request: text_literal("request"),
        status: Some(Expression::Literal(Literal::Integer(200), 2, 1)),
        content_type: None,
        headers: None,
        variable_name: "out".to_string(),
        line: 2,
        column: 1,
    }
}

fn invalid_stream_lead_with_valid_file_fallback() -> Statement {
    Statement::StreamWriteStatement {
        value: Expression::BinaryOperation {
            left: Box::new(Expression::Literal(Literal::Integer(10), 2, 1)),
            operator: Operator::Minus,
            right: Box::new(text_literal("not a number")),
            line: 2,
            column: 1,
        },
        target: Expression::Variable("out".to_string(), 2, 1),
        is_line: true,
        fallback_content: Some(Box::new(text_literal("valid file text"))),
        line: 2,
        column: 1,
    }
}

fn open_out_file() -> Statement {
    Statement::OpenFileStatement {
        path: text_literal("unused.txt"),
        variable_name: "out".to_string(),
        mode: FileOpenMode::Write,
        line: 1,
        column: 1,
    }
}

fn ambiguous_file_write_program(control: Statement) -> Program {
    let mut program = parse(
        "open file at \"unused.txt\" for writing as out\n\
         store value as 10\n\
         store line value as \"text\"\n\
         store n as 1\n\
         write line value minus n to out\n",
    );
    program.statements.insert(1, control);
    program
}

#[test]
fn maybe_skipped_stream_bindings_require_both_write_readings_to_be_valid() {
    let controls = [
        (
            "if",
            Statement::IfStatement {
                condition: bool_literal(false),
                then_block: vec![stream_binding()],
                else_block: None,
                line: 2,
                column: 1,
            },
        ),
        (
            "single-line if",
            Statement::SingleLineIf {
                condition: bool_literal(false),
                then_stmt: Box::new(stream_binding()),
                else_stmt: None,
                line: 2,
                column: 1,
            },
        ),
        (
            "while",
            Statement::WhileLoop {
                condition: bool_literal(false),
                body: vec![stream_binding()],
                line: 2,
                column: 1,
            },
        ),
    ];

    for (label, control) in controls {
        let errors = typecheck(&ambiguous_file_write_program(control))
            .expect_err("File or ResponseStream must conservatively validate both write readings");
        assert!(
            errors.contains("Cannot perform Minus operation"),
            "{label} must retain the possible outer File path and reject the \
             Text/Number classic fallback; got: {errors}"
        );
    }
}

#[test]
fn while_loop_rechecks_stream_lead_after_tail_response_stream_rebind() {
    let program = Program {
        statements: vec![
            open_out_file(),
            Statement::WhileLoop {
                condition: bool_literal(true),
                body: vec![
                    invalid_stream_lead_with_valid_file_fallback(),
                    stream_binding(),
                ],
                line: 2,
                column: 1,
            },
        ],
    };

    let errors = typecheck(&program)
        .expect_err("the loop backedge must recheck the body under ResponseStream or File");
    assert!(
        errors.contains("Cannot perform Minus operation"),
        "the first iteration has a valid File fallback, but a later iteration must reject \
         the Number/Text stream lead after the tail ResponseStream rebind; got: {errors}"
    );
}

#[test]
fn repeat_while_loop_rechecks_stream_lead_after_tail_response_stream_rebind() {
    let program = Program {
        statements: vec![
            open_out_file(),
            Statement::RepeatWhileLoop {
                condition: bool_literal(true),
                body: vec![
                    invalid_stream_lead_with_valid_file_fallback(),
                    stream_binding(),
                ],
                line: 2,
                column: 1,
            },
        ],
    };

    let errors = typecheck(&program)
        .expect_err("the repeat-loop backedge must recheck the body under ResponseStream or File");
    assert!(
        errors.contains("Cannot perform Minus operation"),
        "the first iteration has a valid File fallback, but a later iteration must reject \
         the Number/Text stream lead after the tail ResponseStream rebind; got: {errors}"
    );
}

#[test]
fn two_concrete_branch_types_join_instead_of_taking_the_last_checked_branch() {
    let program = Program {
        statements: vec![
            Statement::IfStatement {
                condition: bool_literal(true),
                then_block: vec![stream_binding()],
                else_block: Some(vec![Statement::OpenFileStatement {
                    path: text_literal("unused.txt"),
                    variable_name: "out".to_string(),
                    mode: FileOpenMode::Write,
                    line: 3,
                    column: 1,
                }]),
                line: 1,
                column: 1,
            },
            Statement::FlushStreamStatement {
                target: Expression::Variable("out".to_string(), 5, 1),
                legacy_binding: None,
                action_fallback: None,
                line: 5,
                column: 1,
            },
        ],
    };

    assert!(
        typecheck(&program).is_ok(),
        "ResponseStream or File must join to a gradual type instead of treating \
         the last checked File branch as definite; errors: {:?}",
        typecheck(&program).err()
    );
}
