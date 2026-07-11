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

    let errors = result.expect_err("Expected at least one type error");
    assert!(
        errors.iter().any(|e| e.message.contains(needle)),
        "Expected a type error containing {needle:?}, got: {errors:?}"
    );
}

/// Minimal repro from issue #605: init to nothing, reassign in a for-each,
/// then index. Must not report "Cannot index into Nothing".
#[test]
fn test_nothing_then_reassign_in_loop_allows_indexing() {
    assert_typechecks_clean(
        r#"
store item as nothing
store items as ["a"]
for each x in items:
    change item to x
end for
display item["k"]
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
