//! Regression coverage for response-stream symbols reconstructed in type-checker
//! child scopes after analyzer body scopes have been discarded.
//!
//! These tests cover type visibility and restoration during checking. Runtime
//! shadows response-stream names inside the corresponding child environments.

use std::sync::Arc;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, Literal, Operator, Program, Statement, WsHandlerEvent};
use wfl::typechecker::TypeChecker;

fn typecheck(code: &str) -> Result<(), String> {
    let tokens = lex_wfl_with_positions(code);
    let program = Parser::new(&tokens).parse().expect("parse");
    TypeChecker::new()
        .check_types(&program)
        .map_err(|errors| format!("{errors:?}"))
}

fn typecheck_program(program: &Program) -> Result<(), String> {
    TypeChecker::new()
        .check_types(program)
        .map_err(|errors| format!("{errors:?}"))
}

fn text_literal(value: &str) -> Expression {
    Expression::Literal(Literal::String(Arc::from(value)), 1, 1)
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

fn outer_number_binding() -> Statement {
    Statement::VariableDeclaration {
        name: "out".to_string(),
        value: Expression::Literal(Literal::Integer(10), 1, 1),
        is_constant: false,
        line: 1,
        column: 1,
    }
}

fn subtract_from_outer_out() -> Statement {
    Statement::DisplayStatement {
        value: Expression::BinaryOperation {
            left: Box::new(Expression::Variable("out".to_string(), 4, 1)),
            operator: Operator::Minus,
            right: Box::new(Expression::Literal(Literal::Integer(1), 4, 1)),
            line: 4,
            column: 1,
        },
        line: 4,
        column: 1,
    }
}

#[test]
fn response_stream_bindings_do_not_escape_typechecker_child_scopes() {
    let scoped_blocks = [
        "repeat while false:\n\
         \x20\x20\x20\x20start streaming response to \"request\" with status 200 as out\n\
         end repeat\n",
        "try:\n\
         \x20\x20\x20\x20start streaming response to \"request\" with status 200 as out\n\
         when error:\n\
         \x20\x20\x20\x20display \"ignored\"\n\
         end try\n",
        "count from 1 to 1:\n\
         \x20\x20\x20\x20start streaming response to \"request\" with status 200 as out\n\
         end count\n",
    ];

    for scoped_block in scoped_blocks {
        let source = format!(
            "open file at \"unused.txt\" for writing as out\n\
             {scoped_block}\
             store value as \"wrong stream type\"\n\
             store line value as 10\n\
             store n as 1\n\
             write line value minus n to out\n"
        );
        assert!(
            typecheck(&source).is_ok(),
            "a response stream reconstructed in a type-checker child scope must not replace \
             the outer File type; source:\n{source}\nerrors: {:?}",
            typecheck(&source).err()
        );
    }
}

#[test]
fn response_stream_bindings_are_local_while_outer_text_remains_visible_afterward() {
    let scoped_blocks = [
        "repeat while false:\n\
         \x20\x20\x20\x20start streaming response to \"request\" with status 200 as out\n\
         \x20\x20\x20\x20flush out\n\
         end repeat\n",
        "try:\n\
         \x20\x20\x20\x20start streaming response to \"request\" with status 200 as out\n\
         \x20\x20\x20\x20flush out\n\
         when error:\n\
         \x20\x20\x20\x20display \"ignored\"\n\
         end try\n",
        "count from 1 to 1:\n\
         \x20\x20\x20\x20start streaming response to \"request\" with status 200 as out\n\
         \x20\x20\x20\x20flush out\n\
         end count\n",
    ];

    for scoped_block in scoped_blocks {
        let source = format!(
            "store out as \"outer text\"\n\
             {scoped_block}\
             store invalid as out minus 1\n"
        );
        let errors = typecheck(&source)
            .expect_err("the outer Text binding must remain Text after the child scope");
        assert!(
            errors.contains("Cannot perform Minus operation"),
            "expected the restored outer Text subtraction error; source:\n{source}\nerrors: {errors}"
        );
        assert!(
            !errors.contains("`flush` requires a response-stream handle"),
            "the local binding must be visible as ResponseStream while checking its child scope; \
             source:\n{source}\nerrors: {errors}"
        );
    }
}

#[test]
fn default_count_binding_does_not_retype_an_outer_count_variable() {
    let source = "store count as \"outside\"\n\
                  count from 1 to 1:\n\
                  \x20\x20\x20\x20display count\n\
                  end count\n\
                  store invalid as count minus 1\n";
    let errors =
        typecheck(source).expect_err("the outer Text `count minus 1` must remain a type error");
    assert!(
        errors.contains("Cannot perform Minus operation"),
        "expected the outer Text/Number subtraction error, got: {errors}"
    );
}

#[test]
fn event_handler_body_types_do_not_leak_after_registration() {
    let program = Program {
        statements: vec![
            outer_number_binding(),
            Statement::EventHandler {
                event_source: text_literal("source"),
                event_name: "changed".to_string(),
                handler_body: vec![stream_binding()],
                line: 2,
                column: 1,
            },
            subtract_from_outer_out(),
        ],
    };

    assert!(
        typecheck_program(&program).is_ok(),
        "a deferred event body runs in a fresh runtime child and must not retype outer `out`; \
         errors: {:?}",
        typecheck_program(&program).err()
    );
}

#[test]
fn websocket_handler_body_types_do_not_leak_after_registration() {
    let program = Program {
        statements: vec![
            outer_number_binding(),
            Statement::WebSocketHandlerStatement {
                event: WsHandlerEvent::Connect,
                server: text_literal("WebSocketServer::127.0.0.1:0"),
                binding: "conn".to_string(),
                body: vec![stream_binding()],
                line: 2,
                column: 1,
            },
            subtract_from_outer_out(),
        ],
    };

    assert!(
        typecheck_program(&program).is_ok(),
        "a deferred WebSocket body runs in a fresh runtime child and must not retype outer \
         `out`; errors: {:?}",
        typecheck_program(&program).err()
    );
}
