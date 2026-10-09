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
        request: Expression::Variable("req".to_string(), 2, 1),
        status: Some(Expression::Literal(Literal::Integer(200), 2, 1)),
        content_type: None,
        headers: None,
        variable_name: "out".to_string(),
        line: 2,
        column: 1,
    }
}

fn with_request_setup(statements: Vec<Statement>) -> Program {
    let mut program = parse(
        "listen on port 8080 as srv\n\
         wait for request comes in on srv as req\n",
    );
    program.statements.extend(statements);
    program
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

fn valid_stream_lead_with_invalid_file_fallback() -> Statement {
    Statement::StreamWriteStatement {
        value: text_literal("valid stream text"),
        target: Expression::Variable("out".to_string(), 2, 1),
        is_line: true,
        fallback_content: Some(Box::new(Expression::BinaryOperation {
            left: Box::new(Expression::Literal(Literal::Integer(10), 2, 1)),
            operator: Operator::Minus,
            right: Box::new(text_literal("not a number")),
            line: 2,
            column: 1,
        })),
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
        "listen on port 8080 as srv\n\
         wait for request comes in on srv as req\n\
         open file at \"unused.txt\" for writing as out\n\
         store value as 10\n\
         store line value as \"text\"\n\
         store n as 1\n\
         write line value minus n to out\n",
    );
    program.statements.insert(3, control);
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
    let program = with_request_setup(vec![
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
    ]);

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
    let program = with_request_setup(vec![
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
    ]);

    let errors = typecheck(&program)
        .expect_err("the repeat-loop backedge must recheck the body under ResponseStream or File");
    assert!(
        errors.contains("Cannot perform Minus operation"),
        "the first iteration has a valid File fallback, but a later iteration must reject \
         the Number/Text stream lead after the tail ResponseStream rebind; got: {errors}"
    );
}

#[test]
fn repeat_until_checks_the_condition_after_the_first_body_execution() {
    let program = parse(
        "store done as nothing\n\
         repeat until done:\n\
             change done to yes\n\
         end repeat\n",
    );

    assert!(
        typecheck(&program).is_ok(),
        "repeat-until is a post-test loop, so its body may establish the condition type first: {:?}",
        typecheck(&program).err()
    );
}

#[test]
fn repeat_until_body_bindings_are_visible_to_its_condition() {
    let program = parse(
        "repeat until done:\n\
             store done as yes\n\
         end repeat\n",
    );

    assert!(
        typecheck(&program).is_ok(),
        "a post-test loop condition may reference a binding established by its body: {:?}",
        typecheck(&program).err()
    );
}

#[test]
fn repeat_until_rechecks_stream_lead_after_tail_response_stream_rebind() {
    let program = with_request_setup(vec![
        open_out_file(),
        Statement::RepeatUntilLoop {
            condition: bool_literal(false),
            body: vec![
                invalid_stream_lead_with_valid_file_fallback(),
                stream_binding(),
            ],
            line: 2,
            column: 1,
        },
    ]);

    let errors = typecheck(&program)
        .expect_err("the repeat-until backedge must recheck the body under joined state");
    assert!(
        errors.contains("Cannot perform Minus operation"),
        "the first iteration has a valid File fallback, but a later iteration must reject \
         the Number/Text stream lead after the tail ResponseStream rebind; got: {errors}"
    );
}

#[test]
fn two_concrete_branch_types_join_instead_of_taking_the_last_checked_branch() {
    let program = with_request_setup(vec![
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
    ]);

    assert!(
        typecheck(&program).is_ok(),
        "ResponseStream or File must join to a gradual type instead of treating \
         the last checked File branch as definite; errors: {:?}",
        typecheck(&program).err()
    );
}

#[test]
fn maybe_empty_collection_loops_preserve_the_zero_iteration_type_path() {
    let controls = [
        Statement::ForEachLoop {
            item_name: "item".to_string(),
            collection: Expression::Literal(Literal::List(vec![]), 2, 1),
            reversed: false,
            body: vec![stream_binding()],
            line: 2,
            column: 1,
        },
        Statement::CountLoop {
            start: Expression::Literal(Literal::Integer(2), 2, 1),
            end: Expression::Literal(Literal::Integer(1), 2, 1),
            step: None,
            downward: false,
            variable_name: None,
            body: vec![stream_binding()],
            line: 2,
            column: 1,
        },
    ];

    for control in controls {
        let program = with_request_setup(vec![
            open_out_file(),
            control,
            valid_stream_lead_with_invalid_file_fallback(),
        ]);
        let errors = typecheck(&program)
            .expect_err("a maybe-empty loop must retain the possible outer File state");
        assert!(
            errors.contains("Cannot perform Minus operation"),
            "the possible zero-iteration File path must validate the classic write fallback: \
             {errors}"
        );
    }
}

#[test]
fn maybe_empty_loops_do_not_apply_outer_assignments_as_definite() {
    for source in [
        "store value as nothing\n\
         count from 2 to 1:\n\
             change value to today\n\
         end count\n\
         store result as value minus 1\n",
        "store value as nothing\n\
         store items as []\n\
         for each item in items:\n\
             change value to today\n\
         end for\n\
         store result as value minus 1\n",
    ] {
        let program = parse(source);
        let errors =
            typecheck(&program).expect_err("a maybe-empty loop leaves a Date-or-Nothing value");
        assert!(
            errors.contains("Date or Nothing"),
            "the zero-iteration and assignment paths must retain Optional<Date>: {errors}"
        );
    }
}

#[test]
fn collection_and_infinite_loops_reset_iteration_local_bindings() {
    let loops = [
        Statement::ForEachLoop {
            item_name: "item".to_string(),
            collection: Expression::Literal(
                Literal::List(vec![
                    Expression::Literal(Literal::Integer(1), 2, 1),
                    Expression::Literal(Literal::Integer(2), 2, 1),
                ]),
                2,
                1,
            ),
            reversed: false,
            body: vec![
                invalid_stream_lead_with_valid_file_fallback(),
                stream_binding(),
            ],
            line: 2,
            column: 1,
        },
        Statement::CountLoop {
            start: Expression::Literal(Literal::Integer(1), 2, 1),
            end: Expression::Literal(Literal::Integer(2), 2, 1),
            step: None,
            downward: false,
            variable_name: None,
            body: vec![
                invalid_stream_lead_with_valid_file_fallback(),
                stream_binding(),
            ],
            line: 2,
            column: 1,
        },
        Statement::ForeverLoop {
            body: vec![
                invalid_stream_lead_with_valid_file_fallback(),
                stream_binding(),
            ],
            line: 2,
            column: 1,
        },
        Statement::MainLoop {
            body: vec![
                invalid_stream_lead_with_valid_file_fallback(),
                stream_binding(),
            ],
            concurrent: false,
            line: 2,
            column: 1,
        },
    ];

    for loop_statement in loops {
        let program = with_request_setup(vec![open_out_file(), loop_statement]);
        assert!(
            typecheck(&program).is_ok(),
            "the runtime clears each iteration's local ResponseStream before the next iteration; \
             errors: {:?}",
            typecheck(&program).err()
        );
    }
}
