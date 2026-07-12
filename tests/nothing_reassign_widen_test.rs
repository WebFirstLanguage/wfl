//! Regression tests for issue #605: a variable initialized to `nothing` and
//! later reassigned must not stay pinned as `Nothing`.
//!
//! The idiomatic pattern is:
//!
//! ```wfl
//! store item as nothing
//! for each x in items:
//!     change item to x
//! end for
//! store value as item["key"]
//! ```
//!
//! Before the fix the type checker recorded `Nothing` from the initializer and
//! never widened it on `change`, so the index raised a false
//! `Cannot index into Nothing` diagnostic even though the program is correct
//! at runtime.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

/// Type-check `code` and assert it produces zero diagnostics.
fn assert_typechecks_clean(code: &str) {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    let mut type_checker = TypeChecker::new();
    let result = type_checker.check_types(&program);

    assert!(
        result.is_ok(),
        "Program should type-check clean; got: {:?}",
        result.err()
    );
}

/// Type-check `code` and assert at least one diagnostic mentions `needle`.
fn assert_type_error_contains(code: &str, needle: &str) {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    let mut type_checker = TypeChecker::new();
    let result = type_checker.check_types(&program);

    let errors = result
        .expect_err("Expected at least one type error")
        .into_diagnostics();
    assert!(
        errors.iter().any(|e| e.message.contains(needle)),
        "Expected a type error containing {needle:?}, got: {errors:?}"
    );
}

/// Minimal repro from issue #605: init to nothing, reassign in a for-each.
/// After the loop the variable must not stay pinned as Nothing.
/// (List literals are `List of Any`, so we assert via a use that would fail
/// on Nothing — length — rather than a map index on a text element.)
#[test]
fn test_nothing_then_reassign_in_loop_allows_indexing() {
    assert_typechecks_clean(
        r#"
store item as nothing
store items as ["a"]
for each x in items:
    change item to x
end for
display length of item
"#,
    );
}

/// Same pattern without a loop: `change` of a nothing-initialized variable
/// must widen the type so later indexing is not rejected.
#[test]
fn test_nothing_then_change_to_map_allows_indexing() {
    assert_typechecks_clean(
        r#"
store file_part as nothing
create map part:
    "name" is "file"
    "content_bytes" is "abc"
end map
change file_part to part
store file_bytes as file_part["content_bytes"]
display file_bytes
"#,
    );
}

/// Direct reassignment of a concrete non-Nothing value still works and does
/// not leave a false Nothing diagnostic on subsequent uses.
#[test]
fn test_nothing_then_change_to_text_is_allowed() {
    assert_typechecks_clean(
        r#"
store item as nothing
change item to "hello"
display item
"#,
    );
}

/// A variable that stays Nothing (never reassigned) must still reject indexing.
#[test]
fn test_nothing_without_reassign_still_rejects_indexing() {
    assert_type_error_contains(
        r#"
store item as nothing
display item["k"]
"#,
        "Cannot index into",
    );
}

/// Concrete type mismatches on reassignment remain errors (Number -> Text).
#[test]
fn test_incompatible_reassignment_still_errors() {
    assert_type_error_contains(
        r#"
store x as 10
change x to "hello"
"#,
        "incompatible type",
    );
}

/// Defining an action that would widen an outer Nothing binding must not
/// permanently refine that outer binding — the action may never be called
/// (PR #606 Codex review).
#[test]
fn test_action_body_does_not_permanently_widen_outer_nothing() {
    assert_type_error_contains(
        r#"
store item as nothing
define action called widen:
    change item to "x"
end action
display item["k"]
"#,
        "Cannot index into",
    );
}

/// Parent-walking `get_symbol_mut` must not let a `when` error binding
/// overwrite an outer variable of the same name (PR #606 review).
///
/// Full-pipeline analysis already rejects `when error as msg` when outer
/// `msg` exists (shadowing), so this builds the AST and uses a pre-seeded
/// analyzer to exercise the type checker path in isolation.
#[test]
fn test_when_error_name_does_not_clobber_outer_variable_type() {
    use wfl::analyzer::{Analyzer, Symbol, SymbolKind};
    use wfl::parser::ast::{ErrorType, Expression, Literal, Program, Statement, Type, WhenClause};

    let mut analyzer = Analyzer::new();
    analyzer
        .define_symbol(Symbol {
            name: "msg".to_string(),
            kind: SymbolKind::Variable { mutable: true },
            symbol_type: Some(Type::Number),
            line: 1,
            column: 1,
        })
        .expect("define outer msg");

    let program = Program {
        statements: vec![
            Statement::TryStatement {
                body: vec![Statement::DisplayStatement {
                    value: Expression::Literal(Literal::String("ok".into()), 2, 1),
                    line: 2,
                    column: 1,
                }],
                when_clauses: vec![WhenClause {
                    error_type: ErrorType::General,
                    error_name: "msg".to_string(),
                    body: vec![Statement::DisplayStatement {
                        value: Expression::Variable("msg".to_string(), 4, 1),
                        line: 4,
                        column: 1,
                    }],
                }],
                otherwise_block: None,
                finally_block: None,
                line: 1,
                column: 1,
            },
            // After the try, outer `msg` must still be Number.
            Statement::Assignment {
                name: "msg".to_string(),
                value: Expression::Literal(Literal::Integer(20), 6, 1),
                line: 6,
                column: 1,
            },
        ],
    };

    let mut type_checker = TypeChecker::with_analyzer(analyzer);
    let result = type_checker.check_types(&program);
    assert!(
        result.is_ok(),
        "when error binding must not clobber outer Number type; got: {:?}",
        result.err()
    );
}
