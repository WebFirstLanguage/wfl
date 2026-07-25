//! Regression (P1 #5): a bracket index immediately after a `.property` (or
//! `.method(...)`) access must bind to that property value, not split off into a
//! separate bogus list-literal statement.
//!
//! Before the fix, `store ct as obj.headers["content-type"]` parsed as TWO
//! statements — `store ct as obj.headers` (a `PropertyAccess`) followed by a
//! standalone `["content-type"]` list literal — silently dropping the lookup so
//! `ct` was bound to the whole `headers` map. This proves both the AST shape
//! (one `IndexAccess` over the `PropertyAccess`) and the runtime value.

use std::fs;
use std::process::Command;
use tempfile::TempDir;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, Literal, Statement};

fn parse(src: &str) -> wfl::parser::ast::Program {
    let tokens = lex_wfl_with_positions(src);
    Parser::new(&tokens).parse().expect("parse should succeed")
}

fn wfl_exe() -> &'static str {
    env!("CARGO_BIN_EXE_wfl")
}

/// Run inline WFL source, returning (stdout+stderr, exit code).
fn run_src(src: &str) -> (String, Option<i32>) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("main.wfl");
    fs::write(&path, src).unwrap();
    let output = Command::new(wfl_exe())
        .arg(&path)
        .output()
        .expect("failed to execute WFL");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    drop(dir);
    (combined, output.status.code())
}

#[test]
fn property_then_index_is_one_index_access_over_property_access() {
    // A single statement: `store ct as obj.headers["content-type"]`.
    let program = parse("store ct as obj.headers[\"content-type\"]\n");
    assert_eq!(
        program.statements.len(),
        1,
        "the property-then-index expression must not split into extra statements; got {:#?}",
        program.statements
    );

    let value = match &program.statements[0] {
        Statement::VariableDeclaration { name, value, .. } => {
            assert_eq!(name, "ct");
            value
        }
        other => panic!("expected a VariableDeclaration, got {other:#?}"),
    };

    // Outer node is IndexAccess["content-type"] whose collection is the
    // PropertyAccess obj.headers.
    match value {
        Expression::IndexAccess {
            collection, index, ..
        } => {
            match index.as_ref() {
                Expression::Literal(Literal::String(key), ..) => {
                    assert_eq!(
                        key.as_ref(),
                        "content-type",
                        "index key must be the bracket string"
                    )
                }
                other => panic!("expected a string index, got {other:#?}"),
            }
            match collection.as_ref() {
                Expression::PropertyAccess {
                    object, property, ..
                } => {
                    assert_eq!(property, "headers");
                    assert!(
                        matches!(object.as_ref(), Expression::Variable(n, ..) if n == "obj"),
                        "property-access object must be the variable `obj`, got {object:#?}"
                    );
                }
                other => panic!(
                    "index collection must be the PropertyAccess obj.headers, got {other:#?}"
                ),
            }
        }
        other => panic!("expected IndexAccess over PropertyAccess, got {other:#?}"),
    }
}

#[test]
fn chained_property_index_nests_left_to_right() {
    // `grid.rows[0][1]` -> IndexAccess( IndexAccess( PropertyAccess(grid.rows), 0 ), 1 )
    let program = parse("store cell as grid.rows[0][1]\n");
    assert_eq!(program.statements.len(), 1, "must be one statement");
    let value = match &program.statements[0] {
        Statement::VariableDeclaration { value, .. } => value,
        other => panic!("expected VariableDeclaration, got {other:#?}"),
    };
    // Outermost index is [1].
    let inner = match value {
        Expression::IndexAccess { collection, .. } => collection,
        other => panic!("expected outer IndexAccess, got {other:#?}"),
    };
    // Next index is [0] over the property access.
    match inner.as_ref() {
        Expression::IndexAccess { collection, .. } => {
            assert!(
                matches!(collection.as_ref(), Expression::PropertyAccess { property, .. } if property == "rows"),
                "innermost collection must be grid.rows, got {collection:#?}"
            );
        }
        other => panic!("expected inner IndexAccess, got {other:#?}"),
    }
}

#[test]
fn property_index_runtime_value_is_the_indexed_field_not_the_whole_map() {
    // Unambiguous: the correct index result ("BBB") differs from the property
    // map, so a split (ct = the headers map) fails the equality check.
    let src = "create map inner:\n\
               \x20   \"a\" is \"AAA\"\n\
               \x20   \"b\" is \"BBB\"\n\
               end map\n\
               create map outer:\n\
               \x20   \"headers\" is inner\n\
               end map\n\
               store ct as outer.headers[\"b\"]\n\
               check if ct is equal to \"BBB\":\n\
               \x20   display \"INDEX_OK\"\n\
               otherwise:\n\
               \x20   display \"INDEX_WRONG\"\n\
               end check\n";
    let (out, code) = run_src(src);
    assert_eq!(code, Some(0), "program should exit 0: {out}");
    assert!(
        out.contains("INDEX_OK"),
        "`outer.headers[\"b\"]` must yield the indexed value \"BBB\", not the whole map: {out}"
    );
    assert!(
        !out.contains("INDEX_WRONG"),
        "the property-then-index lookup returned the wrong value: {out}"
    );
}

#[test]
fn outbound_stream_header_index_parses_as_single_statement() {
    // The canonical proxy pattern from the docs:
    // `store ct as upstream.headers["content-type"]`. It must be one statement
    // (an IndexAccess over the PropertyAccess), not a split that drops the key.
    let program = parse(
        "open url at \"http://example.com\" and stream response as upstream\n\
         store ct as upstream.headers[\"content-type\"]\n",
    );
    assert_eq!(
        program.statements.len(),
        2,
        "expected exactly the open + store statements, no split list literal; got {:#?}",
        program.statements
    );
    match &program.statements[1] {
        Statement::VariableDeclaration { value, .. } => assert!(
            matches!(value, Expression::IndexAccess { .. }),
            "the header lookup must be an IndexAccess, got {value:#?}"
        ),
        other => panic!("expected the store statement, got {other:#?}"),
    }
}
