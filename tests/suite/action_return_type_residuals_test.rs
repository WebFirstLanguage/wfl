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

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
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

fn assert_type_error_contains(code: &str, needle: &str) {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    let errors = TypeChecker::new()
        .check_types(&program)
        .expect_err("Expected a type error")
        .into_diagnostics();
    assert!(
        errors.iter().any(|error| error.message.contains(needle)),
        "Expected a type error containing {needle:?}, got: {errors:?}"
    );
}

async fn interpret_result(code: &str) -> Value {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    TypeChecker::new()
        .check_types(&program)
        .expect("Program should type-check cleanly");

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .expect("Program should execute successfully");

    interpreter
        .global_env()
        .borrow()
        .get("result")
        .expect("Program should define result")
}

#[test]
fn literal_false_repeat_while_infers_nothing_completion() {
    assert_type_error_contains(
        r#"
define action called never_runs:
    repeat while no:
        42
    end repeat
end action

store result as call never_runs
close result
"#,
        "Expected a file or stream handle",
    );
}

#[tokio::test]
async fn repeat_while_returns_its_last_body_value_at_runtime() {
    let result = interpret_result(
        r#"
define action called run_once: Number:
    store should_continue as yes
    repeat while should_continue:
        change should_continue to no
        42
    end repeat
end action

store result as call run_once
"#,
    )
    .await;

    assert_eq!(result, Value::Number(42.0));
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

#[test]
fn exhaustive_return_makes_following_returns_unreachable() {
    assert_type_error_contains(
        r#"
define action called choose with parameters flag:
    check if flag:
        return 1
    otherwise:
        return 2
    end check
    return "dead"
end action

store result as call choose with yes
close result
"#,
        "Expected a file or stream handle",
    );
}

#[test]
fn definitely_returning_finally_overrides_primary_return_type() {
    assert_type_error_contains(
        r#"
define action called final_number:
    try:
        return "primary"
    finally:
        return 42
    end try
end action

create directory at call final_number
"#,
        "directory path",
    );

    assert_type_error_contains(
        r#"
define action called final_text:
    try:
        return 42
    finally:
        return "final"
    end try
end action

store invalid as (call final_text) minus 1
"#,
        "Cannot perform Minus",
    );
}

#[test]
fn partial_action_return_does_not_satisfy_a_text_builtin() {
    assert_type_error_contains(
        r#"
define action called maybe_label with parameters enabled:
    check if enabled:
        return "ready"
    end check
end action

store label as call maybe_label with no
store invalid as touppercase of label
"#,
        "expected Text",
    );
}

#[test]
fn partial_instance_method_return_does_not_satisfy_a_text_builtin() {
    assert_type_error_contains(
        r#"
create container Labeler:
    action maybe_label needs enabled: Boolean:
        check if enabled:
            return "ready"
        end check
    end
end

create new Labeler as labeler:
end
store label as labeler.maybe_label(no)
store invalid as touppercase of label
"#,
        "expected Text",
    );
}

#[test]
fn partial_static_method_return_does_not_satisfy_a_text_builtin() {
    assert_type_error_contains(
        r#"
create container Labels:
    static action maybe_label needs enabled: Boolean:
        check if enabled:
            return "ready"
        end check
    end
end

store label as Labels.maybe_label(no)
store invalid as touppercase of label
"#,
        "expected Text",
    );
}

#[test]
fn nested_partial_return_stays_optional_through_return_join() {
    assert_type_error_contains(
        r#"
define action called maybe_label with parameters enabled:
    check if enabled:
        return "ready"
    end check
end action

define action called wrapped_label with parameters use_partial:
    check if use_partial:
        return call maybe_label with yes
    otherwise:
        return "fallback"
    end check
end action

store label as call wrapped_label with yes
store invalid as touppercase of label
"#,
        "expected Text",
    );
}

#[test]
fn partial_return_stays_optional_in_list_element_join() {
    assert_type_error_contains(
        r#"
define action called maybe_label with parameters enabled:
    check if enabled:
        return "ready"
    end check
end action

store first_label as call maybe_label with no
store labels as [first_label and "fallback"]
store invalid as touppercase of labels[0]
"#,
        "expected Text",
    );
}

#[test]
fn partial_return_stays_optional_across_if_binding_join() {
    assert_type_error_contains(
        r#"
define action called maybe_label with parameters enabled:
    check if enabled:
        return "ready"
    end check
end action

store flag as yes
check if flag:
    store label as call maybe_label with no
otherwise:
    store label as "fallback"
end check
store invalid as touppercase of label
"#,
        "expected Text",
    );
}

#[test]
fn explicit_nothing_return_stays_optional() {
    assert_type_error_contains(
        r#"
define action called maybe_label with parameters enabled:
    check if enabled:
        return "ready"
    otherwise:
        return nothing
    end check
end action

store label as call maybe_label with no
store invalid as touppercase of label
"#,
        "expected Text",
    );
}

#[test]
fn nothing_checks_narrow_optional_values_inside_the_guarded_branch() {
    assert_typechecks_clean(
        r#"
define action called maybe_label with parameters enabled:
    check if enabled:
        return "ready"
    end check
end action

store first_label as call maybe_label with yes
check if first_label is not nothing:
    store upper_first as touppercase of first_label
end check

store second_label as call maybe_label with yes
check if isnothing of second_label:
    display "missing"
otherwise:
    store upper_second as touppercase of second_label
end check
"#,
    );
}

#[test]
fn terminating_nothing_guard_narrows_the_continuation() {
    assert_typechecks_clean(
        r#"
define action called maybe_label with parameters enabled:
    check if enabled:
        return "ready"
    end check
end action

define action called guarded_label with parameters enabled:
    store label as call maybe_label with enabled
    check if label is nothing:
        return "missing"
    end check
    return touppercase of label
end action

store result as call guarded_label with yes
display result
"#,
    );
}

#[test]
fn exit_path_does_not_make_an_action_return_optional() {
    assert_typechecks_clean(
        r#"
define action called label_or_exit with parameters enabled:
    check if enabled:
        return "ready"
    otherwise:
        exit
    end check
end action

store label as call label_or_exit with yes
store upper as touppercase of label
display upper
"#,
    );
}

#[test]
fn return_type_is_captured_at_the_return_program_point() {
    assert_type_error_contains(
        r#"
define action called early_number:
    store values as [1]
    return pop of values
    push with values and "dead"
end action

store result as call early_number
store invalid as touppercase of result
"#,
        "expected Text",
    );
}

#[test]
fn literal_true_return_path_does_not_join_an_unreachable_else_return() {
    assert_type_error_contains(
        r#"
define action called definite_number:
    check if yes:
        return 1
    otherwise:
        return "unreachable"
    end check
end action

store result as call definite_number
store invalid as touppercase of result
"#,
        "expected Text",
    );
}

#[test]
fn terminating_try_guard_narrows_the_continuation() {
    assert_typechecks_clean(
        r#"
define action called maybe_label with parameters enabled:
    check if enabled:
        return "ready"
    end check
end action

define action called guarded_label with parameters enabled:
    store label as call maybe_label with enabled
    check if label is nothing:
        try:
            return "missing"
        finally:
            display "cleanup"
        end try
    end check
    return touppercase of label
end action

store result as call guarded_label with yes
display result
"#,
    );
}

#[test]
fn ambiguous_overload_join_preserves_an_optional_return() {
    assert_type_error_contains(
        r#"
define action called choose_label with parameters value as number:
    check if value is greater than 0:
        return "positive"
    end check
end action

define action called choose_label with parameters value as text:
    return value
end action

store selector as nothing
store label as choose_label of selector
store invalid as touppercase of label
"#,
        "expected Text",
    );
}

#[test]
fn implicit_expression_fallthrough_is_the_action_result() {
    assert_typechecks_clean(
        r#"
define action called label:
    "ready"
end action

store upper as touppercase of call label
display upper
"#,
    );
}

#[test]
fn explicit_and_implicit_action_results_join_gradually() {
    assert_typechecks_clean(
        r#"
define action called mixed_result with parameters choose_number as boolean:
    check if choose_number:
        return 1
    end check
    "text"
end action

store result as call mixed_result with no
check if result is not nothing:
    store upper as touppercase of result
end check
"#,
    );
}

#[test]
fn annotated_action_checks_its_implicit_result() {
    assert_type_error_contains(
        r#"
define action called mislabeled: Number:
    "text"
end action
"#,
        "implicit result",
    );
}
