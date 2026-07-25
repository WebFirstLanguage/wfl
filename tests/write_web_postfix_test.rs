//! Regression (maintainer re-review, P1): postfix accessors on `write line|chunk`
//! operands and on merged `content type` / `headers` clause operands must compose
//! onto the operand instead of dangling after the statement.
//!
//! The lexer merges the command word with the following identifier and leaves any
//! `[...]` index / `.field` property accessors as separate tokens, so
//! `write line chunks[0] to out`, `write line upstream.status to out`,
//! `headers upstream.headers`, and `content type upstream.headers["content-type"]`
//! previously left those accessors to dangle (a parse error or a wrong split).

use std::fs;
use std::process::Command;
use tempfile::TempDir;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, Statement};

fn parse(src: &str) -> wfl::parser::ast::Program {
    let tokens = lex_wfl_with_positions(src);
    Parser::new(&tokens).parse().expect("parse should succeed")
}

fn stream_write_value(stmt: &Statement) -> &Expression {
    match stmt {
        Statement::StreamWriteStatement { value, .. } => value,
        other => panic!("expected a StreamWriteStatement, got {other:#?}"),
    }
}

fn stream_write_fallback(stmt: &Statement) -> &Expression {
    match stmt {
        Statement::StreamWriteStatement {
            fallback_content: Some(fallback),
            ..
        } => fallback,
        other => panic!("expected an ambiguous StreamWriteStatement fallback, got {other:#?}"),
    }
}

fn expression_shape(expr: &Expression) -> &'static str {
    match expr {
        Expression::IndexAccess { .. } => "index",
        Expression::PropertyAccess { .. } => "property",
        Expression::MethodCall { .. } => "method",
        Expression::FunctionCall { .. } => "of-call",
        Expression::BinaryOperation { .. } => "operator",
        other => panic!("unexpected operand shape in parity matrix: {other:#?}"),
    }
}

fn streaming_clause_operand<'a>(stmt: &'a Statement, clause: &str) -> &'a Expression {
    match stmt {
        Statement::StartStreamingResponseStatement {
            content_type,
            headers,
            ..
        } => match clause {
            "content type" => content_type.as_ref().expect("content type operand"),
            "headers" => headers.as_ref().expect("headers operand"),
            other => panic!("unknown test clause {other}"),
        },
        other => panic!("expected StartStreamingResponseStatement, got {other:#?}"),
    }
}

fn streaming_status_and_headers(stmt: &Statement) -> (&Expression, &Expression) {
    match stmt {
        Statement::StartStreamingResponseStatement {
            status: Some(status),
            headers: Some(headers),
            ..
        } => (status, headers),
        other => {
            panic!("expected a streaming response with status and headers operands, got {other:#?}")
        }
    }
}

#[test]
fn streaming_status_clause_accepts_full_expressions_without_swallowing_headers() {
    let cases = [
        (
            "start streaming response to req with status base plus 1 and headers h as out\n",
            "operator",
        ),
        (
            "start streaming response to req with headers h and status codes at i as out\n",
            "index",
        ),
        (
            "start streaming response to req with status reply.code and headers h as out\n",
            "property",
        ),
        (
            "start streaming response to req with status choose of req and headers h as out\n",
            "of-call",
        ),
    ];

    for (source, expected_shape) in cases {
        let program = parse(source);
        assert_eq!(
            program.statements.len(),
            1,
            "`{source}` must remain one statement; got {:#?}",
            program.statements
        );
        let (status, headers) = streaming_status_and_headers(&program.statements[0]);
        assert_eq!(
            expression_shape(status),
            expected_shape,
            "status operand in `{source}`"
        );
        assert!(
            matches!(headers, Expression::Variable(name, ..) if name == "h"),
            "headers must remain a separate clause in `{source}`; got {headers:#?}"
        );
    }
}

#[test]
fn streaming_status_clause_accepts_a_builtin_call_without_swallowing_headers() {
    let program = parse(
        "start streaming response to req with status abs of requested_status and headers h as out\n",
    );
    assert_eq!(program.statements.len(), 1, "got {:#?}", program.statements);

    let (status, headers) = streaming_status_and_headers(&program.statements[0]);
    assert!(
        matches!(
            status,
            Expression::FunctionCall {
                function,
                arguments,
                ..
            } if matches!(function.as_ref(), Expression::Variable(name, ..) if name == "abs")
                && matches!(
                    arguments.as_slice(),
                    [wfl::parser::ast::Argument {
                        value: Expression::Variable(name, ..),
                        ..
                    }] if name == "requested_status"
                )
        ),
        "the builtin status operand must remain `abs of requested_status`; got {status:#?}"
    );
    assert!(
        matches!(headers, Expression::Variable(name, ..) if name == "h"),
        "headers must remain a separate response clause; got {headers:#?}"
    );
}

#[test]
fn write_and_streaming_clauses_share_the_full_expression_suffix_grammar() {
    let cases = [
        ("values[0]", "index"),
        ("object.field", "property"),
        ("object.method()", "method"),
        ("values at 0", "index"),
        ("values 0", "index"),
        ("convert of values", "of-call"),
        ("values plus 1", "operator"),
    ];

    for (operand, expected) in cases {
        let write = parse(&format!("write line {operand} to out\n"));
        assert_eq!(
            write.statements.len(),
            1,
            "write operand `{operand}` split into extra statements: {:#?}",
            write.statements
        );
        assert_eq!(
            expression_shape(stream_write_value(&write.statements[0])),
            expected,
            "write operand `{operand}`"
        );

        for clause in ["content type", "headers"] {
            let program = parse(&format!(
                "start streaming response to req with status 200 and {clause} {operand} as out\n"
            ));
            assert_eq!(
                program.statements.len(),
                1,
                "{clause} operand `{operand}` split into extra statements: {:#?}",
                program.statements
            );
            assert_eq!(
                expression_shape(streaming_clause_operand(&program.statements[0], clause)),
                expected,
                "{clause} operand `{operand}`"
            );
        }
    }
}

#[test]
fn unmerged_content_type_literal_keeps_operator_continuation() {
    let program = parse(
        "start streaming response to req with status 200 and content type \"text/\" with subtype as out\n",
    );
    assert_eq!(program.statements.len(), 1, "got {:#?}", program.statements);
    let content_type = streaming_clause_operand(&program.statements[0], "content type");
    assert!(
        matches!(content_type, Expression::Concatenation { .. }),
        "unmerged literal operand must retain `with subtype`, got {content_type:#?}"
    );
}

#[test]
fn write_line_indexed_operand_composes_into_one_index_access() {
    let program = parse("write line chunks[0] to out\n");
    assert_eq!(
        program.statements.len(),
        1,
        "the indexed write operand must not split into extra statements; got {:#?}",
        program.statements
    );
    assert!(
        matches!(
            stream_write_value(&program.statements[0]),
            Expression::IndexAccess { .. }
        ),
        "the write value must be an IndexAccess, got {:#?}",
        stream_write_value(&program.statements[0])
    );
}

#[test]
fn write_line_property_operand_composes_into_one_property_access() {
    let program = parse("write line upstream.status to out\n");
    assert_eq!(
        program.statements.len(),
        1,
        "the property write operand must not split into extra statements; got {:#?}",
        program.statements
    );
    assert!(
        matches!(
            stream_write_value(&program.statements[0]),
            Expression::PropertyAccess { .. }
        ),
        "the write value must be a PropertyAccess, got {:#?}",
        stream_write_value(&program.statements[0])
    );
}

#[test]
fn streaming_response_headers_clause_composes_postfix() {
    // `headers upstream.headers` — the operand must bind the `.headers` access.
    let program = parse(
        "start streaming response to req with status 200 and headers upstream.headers as out\n",
    );
    assert_eq!(program.statements.len(), 1, "got {:#?}", program.statements);
    match &program.statements[0] {
        Statement::StartStreamingResponseStatement { headers, .. } => {
            assert!(
                matches!(headers, Some(Expression::PropertyAccess { .. })),
                "the headers operand must be a PropertyAccess, got {headers:#?}"
            );
        }
        other => panic!("expected StartStreamingResponseStatement, got {other:#?}"),
    }
}

#[test]
fn streaming_response_content_type_clause_composes_property_then_index() {
    // `content type upstream.headers["content-type"]` — property then index.
    let program = parse(
        "start streaming response to req with status 200 and content type upstream.headers[\"content-type\"] as out\n",
    );
    assert_eq!(program.statements.len(), 1, "got {:#?}", program.statements);
    match &program.statements[0] {
        Statement::StartStreamingResponseStatement { content_type, .. } => {
            assert!(
                matches!(content_type, Some(Expression::IndexAccess { .. })),
                "the content type operand must be an IndexAccess (over a PropertyAccess), \
                 got {content_type:#?}"
            );
        }
        other => panic!("expected StartStreamingResponseStatement, got {other:#?}"),
    }
}

#[test]
fn write_line_at_indexing_parses() {
    // Classic `write line values at 0 to "/tmp/out"` must parse (issue #642).
    let program = parse(
        "store line values as [\"first\" and \"second\"]\n\
         write line values at 0 to \"/tmp/out\"\n",
    );
    assert!(
        program.statements.len() >= 2,
        "expected store + write, got {:#?}",
        program.statements
    );
    let write = program
        .statements
        .iter()
        .find(|s| matches!(s, Statement::StreamWriteStatement { .. }))
        .expect("write statement");
    // Stream reading value is IndexAccess over Variable("values").
    assert!(
        matches!(stream_write_value(write), Expression::IndexAccess { .. }),
        "write value must be IndexAccess for `at` indexing, got {:#?}",
        stream_write_value(write)
    );
    assert!(
        matches!(
            stream_write_fallback(write),
            Expression::IndexAccess { collection, index, .. }
                if matches!(collection.as_ref(), Expression::Variable(name, ..) if name == "line values")
                    && matches!(index.as_ref(), Expression::Literal(wfl::parser::ast::Literal::Integer(0), ..))
        ),
        "classic fallback must index the full `line values` binding, got {:#?}",
        stream_write_fallback(write)
    );
}

#[test]
fn write_line_direct_integer_indexing_parses() {
    // Classic `write line values 0 to "/tmp/out"` must parse (issue #642).
    let program = parse("write line values 0 to \"/tmp/out\"\n");
    assert_eq!(program.statements.len(), 1, "got {:#?}", program.statements);
    assert!(
        matches!(
            stream_write_value(&program.statements[0]),
            Expression::IndexAccess { .. }
        ),
        "write value must be IndexAccess for direct integer indexing, got {:#?}",
        stream_write_value(&program.statements[0])
    );
    assert!(
        matches!(
            stream_write_fallback(&program.statements[0]),
            Expression::IndexAccess { collection, index, .. }
                if matches!(collection.as_ref(), Expression::Variable(name, ..) if name == "line values")
                    && matches!(index.as_ref(), Expression::Literal(wfl::parser::ast::Literal::Integer(0), ..))
        ),
        "classic fallback must retain the full merged binding for direct indexing, got {:#?}",
        stream_write_fallback(&program.statements[0])
    );
}

#[test]
fn streaming_response_content_type_of_call_parses() {
    // `content type mime_type of path` — `of` continuation on the merged clause
    // operand must compose like an ordinary expression (issue #642).
    let program = parse(
        "start streaming response to req with status 200 and content type mime_type of path as out\n",
    );
    assert_eq!(program.statements.len(), 1, "got {:#?}", program.statements);
    match &program.statements[0] {
        Statement::StartStreamingResponseStatement { content_type, .. } => {
            assert!(
                matches!(content_type, Some(Expression::FunctionCall { .. })),
                "content type operand must be a FunctionCall (`mime_type of path`), got {content_type:#?}"
            );
        }
        other => panic!("expected StartStreamingResponseStatement, got {other:#?}"),
    }
}

#[test]
fn streaming_response_content_type_then_headers_both_orders() {
    // Clause connectives must not be swallowed as Boolean AND (both orders).
    let a = parse(
        "start streaming response to req with status 200 and content type ct and headers h as out\n",
    );
    match &a.statements[0] {
        Statement::StartStreamingResponseStatement {
            content_type,
            headers,
            ..
        } => {
            assert!(content_type.is_some(), "content type must bind");
            assert!(
                headers.is_some(),
                "headers must bind (not swallowed by content type)"
            );
        }
        other => panic!("expected StartStreamingResponseStatement, got {other:#?}"),
    }
    let b = parse(
        "start streaming response to req with status 200 and headers h and content type ct as out\n",
    );
    match &b.statements[0] {
        Statement::StartStreamingResponseStatement {
            content_type,
            headers,
            ..
        } => {
            assert!(headers.is_some(), "headers must bind");
            assert!(
                content_type.is_some(),
                "content type must bind (not swallowed by headers)"
            );
        }
        other => panic!("expected StartStreamingResponseStatement, got {other:#?}"),
    }
}

#[test]
fn streaming_clause_boundary_survives_nested_concatenation_rhs_in_both_orders() {
    let cases = [
        (
            "start streaming response to req with content type \"text/\" with subtype and headers h as out\n",
            "content type",
        ),
        (
            "start streaming response to req with headers base_headers with extra and content type ct as out\n",
            "headers",
        ),
    ];

    for (source, nested_clause) in cases {
        let program = parse(source);
        assert_eq!(
            program.statements.len(),
            1,
            "`{source}` must remain one statement; got {:#?}",
            program.statements
        );
        let statement = &program.statements[0];
        assert!(
            matches!(
                streaming_clause_operand(statement, nested_clause),
                Expression::Concatenation { .. }
            ),
            "{nested_clause} must retain its complete concatenation operand; got {statement:#?}"
        );
        assert!(
            matches!(
                statement,
                Statement::StartStreamingResponseStatement {
                    content_type: Some(_),
                    headers: Some(_),
                    ..
                }
            ),
            "the following response clause must not be swallowed into the concatenation RHS; \
             got {statement:#?}"
        );
    }
}

#[test]
fn type_prefixed_identifier_is_content_not_a_response_clause() {
    let source = "start streaming response to req with content type \"application/\" with type suffix and headers h as out\n";
    let program = parse(source);
    assert_eq!(
        program.statements.len(),
        1,
        "`{source}` must remain one statement; got {:#?}",
        program.statements
    );
    match &program.statements[0] {
        Statement::StartStreamingResponseStatement {
            content_type: Some(Expression::Concatenation { right, .. }),
            headers: Some(Expression::Variable(headers, ..)),
            ..
        } => {
            assert!(
                matches!(right.as_ref(), Expression::Variable(name, ..) if name == "type suffix"),
                "`type suffix` must remain the concatenation RHS, got {right:#?}"
            );
            assert_eq!(
                headers, "h",
                "the following headers clause must remain separate"
            );
        }
        other => panic!(
            "expected concatenated content type plus a separate headers clause, got {other:#?}"
        ),
    }
}

#[test]
fn streaming_clause_boundary_survives_at_index_expression_in_both_orders() {
    let cases = [
        (
            "start streaming response to req with content type media_types at kind and headers h as out\n",
            "content type",
        ),
        (
            "start streaming response to req with headers header_sets at kind and content type ct as out\n",
            "headers",
        ),
    ];

    for (source, indexed_clause) in cases {
        let program = parse(source);
        assert_eq!(
            program.statements.len(),
            1,
            "`{source}` must remain one statement; got {:#?}",
            program.statements
        );
        let statement = &program.statements[0];
        assert!(
            matches!(
                streaming_clause_operand(statement, indexed_clause),
                Expression::IndexAccess { .. }
            ),
            "{indexed_clause} must retain its complete `at` index operand; got {statement:#?}"
        );
        assert!(
            matches!(
                statement,
                Statement::StartStreamingResponseStatement {
                    content_type: Some(_),
                    headers: Some(_),
                    ..
                }
            ),
            "the following response clause must not be swallowed into the `at` index; \
             got {statement:#?}"
        );
    }
}

#[test]
fn streaming_clause_boundary_propagates_through_recursive_operand_forms() {
    let cases = [
        (
            "start streaming response to req with content type lookup of media_types at kind and headers h as out\n",
            "of-call",
        ),
        (
            "start streaming response to req with content type touppercase with media_types at kind and headers h as out\n",
            "builtin-call",
        ),
        (
            "start streaming response to req with content type not media_types at kind and headers h as out\n",
            "unary",
        ),
    ];

    for (source, operand_kind) in cases {
        let program = parse(source);
        assert_eq!(
            program.statements.len(),
            1,
            "`{source}` must remain one statement; got {:#?}",
            program.statements
        );
        let statement = &program.statements[0];
        let content_type = streaming_clause_operand(statement, "content type");
        let has_expected_shape = match operand_kind {
            "of-call" => matches!(content_type, Expression::FunctionCall { .. }),
            "builtin-call" => matches!(content_type, Expression::ActionCall { .. }),
            "unary" => matches!(content_type, Expression::UnaryOperation { .. }),
            _ => unreachable!("unknown recursive operand kind"),
        };
        assert!(
            has_expected_shape,
            "{operand_kind} operand must retain its complete AST; got {content_type:#?}"
        );
        assert!(
            matches!(
                statement,
                Statement::StartStreamingResponseStatement {
                    content_type: Some(_),
                    headers: Some(_),
                    ..
                }
            ),
            "the following headers clause must survive {operand_kind} recursion; got {statement:#?}"
        );
    }
}

#[test]
fn streaming_clause_boundary_propagates_through_explicit_call_arguments() {
    let program = parse(
        "start streaming response to req with content type call render with value and headers h as out\n",
    );
    assert_eq!(program.statements.len(), 1, "got {:#?}", program.statements);

    let statement = &program.statements[0];
    let content_type = streaming_clause_operand(statement, "content type");
    assert!(
        matches!(
            content_type,
            Expression::ActionCall { arguments, .. }
                if arguments.len() == 1
                    && matches!(&arguments[0].value, Expression::Variable(name, ..) if name == "value")
        ),
        "the following headers clause must not become another explicit-call argument; \
         got {content_type:#?}"
    );
    assert!(
        matches!(
            statement,
            Statement::StartStreamingResponseStatement {
                headers: Some(Expression::Variable(name, ..)),
                ..
            } if name == "h"
        ),
        "the headers clause must remain outside the explicit call; got {statement:#?}"
    );
}

#[test]
fn streaming_clause_boundary_propagates_through_at_index_under_file_exists() {
    let program = parse(
        "start streaming response to req with content type file exists at paths at kind and headers h as out\n",
    );
    assert_eq!(program.statements.len(), 1, "got {:#?}", program.statements);

    let statement = &program.statements[0];
    let content_type = streaming_clause_operand(statement, "content type");
    assert!(
        matches!(
            content_type,
            Expression::FileExists { path, .. }
                if matches!(
                    path.as_ref(),
                    Expression::IndexAccess { collection, index, .. }
                        if matches!(collection.as_ref(), Expression::Variable(name, ..) if name == "paths")
                            && matches!(index.as_ref(), Expression::Variable(name, ..) if name == "kind")
                )
        ),
        "the wrapper's nested `at` index must stop before the next response clause; \
         got {content_type:#?}"
    );
    assert!(
        matches!(
            statement,
            Statement::StartStreamingResponseStatement {
                headers: Some(Expression::Variable(name, ..)),
                ..
            } if name == "h"
        ),
        "the headers clause must survive recursion through `file exists at`; got {statement:#?}"
    );
}

fn assert_post_of_index(expr: &Expression, expected_function: &str, expected_argument: &str) {
    match expr {
        Expression::IndexAccess {
            collection, index, ..
        } => {
            assert!(
                matches!(
                    index.as_ref(),
                    Expression::Literal(wfl::parser::ast::Literal::Integer(0), ..)
                ),
                "post-call index must be integer zero, got {index:#?}"
            );
            match collection.as_ref() {
                Expression::FunctionCall {
                    function,
                    arguments,
                    ..
                } => {
                    assert_eq!(
                        leftmost_variable(function),
                        Some(expected_function),
                        "unexpected function root in {expr:#?}"
                    );
                    assert!(
                        matches!(
                            arguments.as_slice(),
                            [wfl::parser::ast::Argument {
                                value: Expression::Variable(name, ..),
                                ..
                            }] if name == expected_argument
                        ),
                        "the parenthesized argument must stay inside the call, got {arguments:#?}"
                    );
                }
                other => panic!("index must wrap an `of` FunctionCall, got {other:#?}"),
            }
        }
        other => panic!("expected postfix index after an `of` call, got {other:#?}"),
    }
}

fn post_of_index_shape(expr: &Expression) -> (String, String, i64) {
    match expr {
        Expression::IndexAccess {
            collection, index, ..
        } => {
            let index = match index.as_ref() {
                Expression::Literal(wfl::parser::ast::Literal::Integer(index), ..) => *index,
                other => panic!("expected an integer post-call index, got {other:#?}"),
            };
            match collection.as_ref() {
                Expression::FunctionCall {
                    function,
                    arguments,
                    ..
                } => {
                    let function = leftmost_variable(function).unwrap_or_else(|| {
                        panic!("expected a variable call root, got {function:#?}")
                    });
                    let argument = match arguments.as_slice() {
                        [
                            wfl::parser::ast::Argument {
                                value: Expression::Variable(name, ..),
                                ..
                            },
                        ] => name,
                        other => panic!("expected one variable call argument, got {other:#?}"),
                    };
                    (function.to_string(), argument.to_string(), index)
                }
                other => panic!("expected the index to wrap an `of` call, got {other:#?}"),
            }
        }
        other => panic!("expected a post-`of` index expression, got {other:#?}"),
    }
}

fn initializer_expression(source: &str) -> Expression {
    let program = parse(&format!("store parity result as {source}\n"));
    match &program.statements[0] {
        Statement::VariableDeclaration { value, .. } => value.clone(),
        other => panic!("expected a variable initializer, got {other:#?}"),
    }
}

#[test]
fn seeded_operands_resume_postfix_parsing_after_of_calls() {
    let write = parse("write line choose of (chunks)[0] to out\n");
    assert_eq!(write.statements.len(), 1, "got {:#?}", write.statements);
    assert_post_of_index(stream_write_value(&write.statements[0]), "choose", "chunks");
    assert_post_of_index(
        stream_write_fallback(&write.statements[0]),
        "line choose",
        "chunks",
    );

    let streaming = parse(
        "start streaming response to req with content type choose of (types)[0] and headers h as out\n",
    );
    assert_eq!(
        streaming.statements.len(),
        1,
        "got {:#?}",
        streaming.statements
    );
    assert_post_of_index(
        streaming_clause_operand(&streaming.statements[0], "content type"),
        "choose",
        "types",
    );
    assert!(
        matches!(
            &streaming.statements[0],
            Statement::StartStreamingResponseStatement {
                headers: Some(Expression::Variable(name, ..)),
                ..
            } if name == "h"
        ),
        "headers must remain outside the indexed call; got {:#?}",
        streaming.statements[0]
    );

    let flush = parse("flush cache of (items)[0]\n");
    assert_eq!(flush.statements.len(), 1, "got {:#?}", flush.statements);
    match &flush.statements[0] {
        Statement::FlushStreamStatement {
            target,
            action_fallback: Some(fallback),
            ..
        } => {
            assert_post_of_index(target, "cache", "items");
            assert_post_of_index(fallback, "flush cache", "items");
        }
        other => panic!("expected ambiguous FlushStreamStatement, got {other:#?}"),
    }
}

#[test]
fn post_of_operands_match_the_ordinary_expression_ast_shape() {
    let write = parse("write line choose of (chunks)[0] to out\n");
    let ordinary_write = initializer_expression("choose of (chunks)[0]");
    assert_eq!(
        post_of_index_shape(stream_write_value(&write.statements[0])),
        post_of_index_shape(&ordinary_write),
        "the seeded write operand must compose exactly like an ordinary expression"
    );
    let ordinary_classic_write = initializer_expression("line choose of (chunks)[0]");
    assert_eq!(
        post_of_index_shape(stream_write_fallback(&write.statements[0])),
        post_of_index_shape(&ordinary_classic_write),
        "the classic write fallback must preserve the ordinary post-`of` AST"
    );

    let streaming =
        parse("start streaming response to req with content type choose of (types)[0] as out\n");
    let ordinary_content_type = initializer_expression("choose of (types)[0]");
    assert_eq!(
        post_of_index_shape(streaming_clause_operand(
            &streaming.statements[0],
            "content type",
        )),
        post_of_index_shape(&ordinary_content_type),
        "the content-type operand must compose exactly like an ordinary expression"
    );

    let flush = parse("flush cache of (items)[0]\n");
    let (target, fallback) = match &flush.statements[0] {
        Statement::FlushStreamStatement {
            target,
            action_fallback: Some(fallback),
            ..
        } => (target, fallback),
        other => panic!("expected an ambiguous FlushStreamStatement, got {other:#?}"),
    };
    let ordinary_flush_target = initializer_expression("cache of (items)[0]");
    assert_eq!(
        post_of_index_shape(target),
        post_of_index_shape(&ordinary_flush_target),
        "the streaming flush target must preserve the ordinary post-`of` AST"
    );
    let ordinary_legacy_flush = initializer_expression("flush cache of (items)[0]");
    assert_eq!(
        post_of_index_shape(fallback),
        post_of_index_shape(&ordinary_legacy_flush),
        "the legacy flush fallback must preserve the ordinary post-`of` AST"
    );
}

#[test]
fn write_line_of_call_argument_absorbs_arithmetic() {
    // `double of n minus 1` must parse as `double of (n minus 1)` — the same
    // precedence as an ordinary expression — not `(double of n) minus 1`.
    let program = parse("write line double of n minus 1 to out\n");
    assert_eq!(program.statements.len(), 1, "got {:#?}", program.statements);
    match stream_write_value(&program.statements[0]) {
        Expression::FunctionCall { arguments, .. } => {
            assert_eq!(
                arguments.len(),
                1,
                "the `of` call should take one argument, got {arguments:#?}"
            );
            assert!(
                matches!(arguments[0].value, Expression::BinaryOperation { .. }),
                "the `of` argument must absorb `minus 1` (double of (n minus 1)), got {:#?}",
                arguments[0].value
            );
        }
        other => panic!("expected the value to be an `of` FunctionCall, got {other:#?}"),
    }
}

#[test]
fn write_line_method_call_operand_composes() {
    // `obj.method()` must compose into a MethodCall, not leave `()` dangling.
    let program = parse("write line obj.method() to out\n");
    assert_eq!(program.statements.len(), 1, "got {:#?}", program.statements);
    assert!(
        matches!(
            stream_write_value(&program.statements[0]),
            Expression::MethodCall { .. }
        ),
        "the write value must be a MethodCall, got {:#?}",
        stream_write_value(&program.statements[0])
    );
}

#[test]
fn flush_method_call_operand_composes() {
    // `flush obj.method()` must compose the method call onto the operand.
    let program = parse("flush obj.method()\n");
    assert_eq!(program.statements.len(), 1, "got {:#?}", program.statements);
    match &program.statements[0] {
        Statement::FlushStreamStatement { target, .. } => {
            assert!(
                matches!(target, Expression::MethodCall { .. }),
                "the flush operand must be a MethodCall, got {target:#?}"
            );
        }
        other => panic!("expected FlushStreamStatement, got {other:#?}"),
    }
}

fn leftmost_variable(expr: &Expression) -> Option<&str> {
    match expr {
        Expression::Variable(name, ..) => Some(name),
        Expression::IndexAccess { collection, .. } => leftmost_variable(collection),
        Expression::PropertyAccess { object, .. } | Expression::MethodCall { object, .. } => {
            leftmost_variable(object)
        }
        Expression::BinaryOperation { left, .. } => leftmost_variable(left),
        Expression::FunctionCall { function, .. } => leftmost_variable(function),
        _ => None,
    }
}

#[test]
fn flush_preserves_binary_continuation_for_both_interpretations() {
    let program = parse("flush cache plus 1\n");
    assert_eq!(program.statements.len(), 1, "got {:#?}", program.statements);
    match &program.statements[0] {
        Statement::FlushStreamStatement {
            target,
            action_fallback,
            ..
        } => {
            assert!(
                matches!(target, Expression::BinaryOperation { .. }),
                "stream target must keep `plus 1`, got {target:#?}"
            );
            assert_eq!(leftmost_variable(target), Some("cache"));
            let fallback = action_fallback.as_ref().expect("legacy fallback");
            assert!(
                matches!(fallback, Expression::BinaryOperation { .. }),
                "legacy expression must keep `plus 1`, got {fallback:#?}"
            );
            assert_eq!(leftmost_variable(fallback), Some("flush cache"));
        }
        other => panic!("expected FlushStreamStatement, got {other:#?}"),
    }
}

#[test]
fn flush_preserves_arbitrarily_nested_postfix_for_both_interpretations() {
    let program = parse("flush cache[0][0]\n");
    assert_eq!(program.statements.len(), 1, "got {:#?}", program.statements);
    match &program.statements[0] {
        Statement::FlushStreamStatement {
            target,
            action_fallback,
            ..
        } => {
            assert!(
                matches!(
                    target,
                    Expression::IndexAccess { collection, .. }
                        if matches!(collection.as_ref(), Expression::IndexAccess { .. })
                ),
                "stream target must retain both indexes, got {target:#?}"
            );
            assert_eq!(leftmost_variable(target), Some("cache"));
            let fallback = action_fallback.as_ref().expect("legacy fallback");
            assert!(
                matches!(
                    fallback,
                    Expression::IndexAccess { collection, .. }
                        if matches!(collection.as_ref(), Expression::IndexAccess { .. })
                ),
                "legacy expression must retain both indexes, got {fallback:#?}"
            );
            assert_eq!(leftmost_variable(fallback), Some("flush cache"));
        }
        other => panic!("expected FlushStreamStatement, got {other:#?}"),
    }
}

#[test]
fn classic_indexed_file_write_still_works_at_runtime() {
    // The ambiguous merged form's classic file-write reading must keep working with
    // an indexed operand: `write line values[0] to <file>`. The target is a text
    // path, so the runtime takes the classic reading whose content is the merged
    // lead `line values` indexed at 0 — it must write the first element, not fail on
    // a dangling `[0]`.
    let dir = TempDir::new().expect("tempdir");
    let out = dir.path().join("out.txt");
    let out_str = out.to_string_lossy().replace('\\', "/");
    let src = format!(
        "store line values as [\"first\" and \"second\"]\n\
         write line values[0] to \"{out_str}\"\n"
    );
    let program_file = dir.path().join("main.wfl");
    fs::write(&program_file, &src).unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .arg(&program_file)
        .status()
        .expect("run wfl");
    assert!(
        status.success(),
        "classic indexed file write should succeed"
    );
    let written = fs::read_to_string(&out).expect("output file written");
    assert_eq!(
        written.trim_end(),
        "first",
        "the indexed element must be written, not the whole list"
    );
}
