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
