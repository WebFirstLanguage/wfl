//! Regression tests for the remaining residuals of issue #560: an unannotated
//! action that clearly returns a value must not have its call result typed
//! `Nothing`.
//!
//! #575 added post-body return-type inference and #591 seeded recursion with
//! `Unknown`, but two shapes still slipped through:
//!
//! 1. `collect_return_types` did not descend into `try:` blocks (`try` body,
//!    `when error` clauses, `otherwise`, `finally`), so an action whose only
//!    `return`s live inside a `try:` block was inferred as `Nothing`, and
//!    indexing its result raised a false `Cannot index into Nothing`.
//! 2. Container methods were registered with `return_type: Nothing` when
//!    unannotated and never refined, so `instance.method()` results hit the
//!    same false diagnostic.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

/// Type-check `code` and assert it produces zero diagnostics (see
/// `recursive_action_return_type_test.rs` for why fully-clean is the tighter
/// regression guard).
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

/// An action whose only `return`s are inside a `try:` body / `when error`
/// clause must infer its return type from them, not default to `Nothing`.
#[test]
fn test_return_inside_try_block_infers_return_type() {
    assert_typechecks_clean(
        r#"
define action called load_data:
    try:
        return [1 and 2]
    when error:
        return [3 and 4]
    end try
end action

store xs as call load_data
store x0 as xs[0]
display x0
"#,
    );
}

/// Same, with `otherwise` (the success clause) also returning.
#[test]
fn test_return_inside_try_otherwise_infers_return_type() {
    assert_typechecks_clean(
        r#"
define action called risky:
    try:
        store vals as [1 and 2]
        return vals
    when error:
        return [9 and 9]
    otherwise:
        return [7 and 7]
    end try
end action

store xs as call risky
store x0 as xs[0]
display x0
"#,
    );
}

/// An unannotated container method that returns a value must not have its
/// call result typed `Nothing`.
#[test]
fn test_container_method_result_not_typed_nothing() {
    assert_typechecks_clean(
        r#"
create container Store:
    property label: Text

    action get_items:
        return [1 and 2]
    end
end

create new Store as s:
    label is "main"
end

store xs as s.get_items()
store x0 as xs[0]
display x0
"#,
    );
}

/// The same holds for a method inherited from a parent container.
#[test]
fn test_inherited_container_method_result_not_typed_nothing() {
    assert_typechecks_clean(
        r#"
create container Base:
    property label: Text

    action get_items:
        return [1 and 2]
    end
end

create container Child extends Base:
    property extra: Text
end

create new Child as c:
    label is "main"
    extra is "x"
end

store xs as c.get_items()
store x0 as xs[0]
display x0
"#,
    );
}
