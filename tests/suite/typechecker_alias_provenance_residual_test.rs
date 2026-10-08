//! Regression coverage for action-return provenance visible through diagnostics.
//!
//! Flow-sensitive alias-state transitions are asserted directly by the
//! typechecker unit tests, where the inferred symbol types are observable.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

fn assert_type_error_contains(source: &str, expected: &str) {
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("program should parse");
    let diagnostics = TypeChecker::new()
        .check_types(&program)
        .expect_err("program should be rejected")
        .into_diagnostics();

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains(expected)),
        "expected a diagnostic containing {expected:?}, got {diagnostics:?}"
    );
}

fn assert_typechecks(source: &str) {
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("program should parse");
    TypeChecker::new()
        .check_types(&program)
        .unwrap_or_else(|failure| {
            panic!(
                "program should type-check, got {:?}",
                failure.into_diagnostics()
            )
        });
}

#[test]
fn explicit_fresh_list_return_preserves_its_element_type() {
    assert_type_error_contains(
        r#"
define action called make_numbers:
    store fresh_numbers as [1]
    return fresh_numbers
end action

store items as call make_numbers
create directory at items[0]
"#,
        "directory path",
    );
}

#[test]
fn implicit_fresh_list_return_preserves_its_element_type() {
    assert_type_error_contains(
        r#"
define action called make_numbers:
    [1]
end action

store items as call make_numbers
create directory at items[0]
"#,
        "directory path",
    );
}

#[test]
fn nested_shared_return_escapes_only_the_shared_list_path() {
    assert_type_error_contains(
        r#"
store leaf as [1]
define action called expose_nested:
    return [leaf]
end action

store exposed as call expose_nested
create directory at exposed[0]
"#,
        "directory path",
    );
}

#[test]
fn stored_user_action_alias_does_not_eagerly_escape_a_returned_captured_list() {
    assert_type_error_contains(
        r#"
store shared_values as [1]
define action called expose with parameters unused as number:
    return shared_values
end action

store saved_expose as expose
store exposed as saved_expose of 0
create directory at shared_values[0]
"#,
        "directory path",
    );
}

#[test]
fn mutating_a_stored_user_action_alias_return_updates_the_captured_list() {
    assert_typechecks(
        r#"
store shared_values as [1]
define action called expose with parameters unused as number:
    return shared_values
end action

store saved_expose as expose
store exposed as saved_expose of 0
push with exposed and "text"
create directory at shared_values[0]
"#,
    );
}

#[test]
fn stored_pop_alias_preserves_the_returned_nested_list_provenance() {
    assert_typechecks(
        r#"
store leaf as [1]
store nested as [leaf]
store take_last as pop
store exposed as take_last of nested
push with exposed and "text"
create directory at leaf[0]
"#,
    );
}

#[test]
fn bare_zero_argument_action_does_not_eagerly_escape_a_returned_captured_list() {
    assert_type_error_contains(
        r#"
store shared_values as [1]
define action called expose:
    return shared_values
end action

store exposed as expose
create directory at shared_values[0]
"#,
        "directory path",
    );
}

#[test]
fn mutating_a_bare_zero_argument_action_return_updates_the_captured_list() {
    assert_typechecks(
        r#"
store shared_values as [1]
define action called expose:
    return shared_values
end action

store exposed as expose
push with exposed and "text"
create directory at shared_values[0]
"#,
    );
}

#[test]
fn one_iteration_loop_completion_preserves_captured_list_return_provenance() {
    assert_typechecks(
        r#"
store shared_values as [1]
define action called expose:
    repeat until yes:
        shared_values
    end repeat
end action

store exposed as call expose
push with exposed and "text"
create directory at shared_values[0]
"#,
    );
}

#[test]
fn inner_binding_does_not_inherit_a_same_named_outer_action_alias() {
    assert_type_error_contains(
        r#"
define action called label with parameters value as number:
    return "outer"
end action

store selected as label
define action called increment with parameters selected as number:
    return selected of 1
end action

store inner_result as increment of 1
"#,
        "not a function",
    );
}

#[test]
fn calling_a_closure_that_rebinds_a_captured_action_alias_invalidates_it() {
    assert_typechecks(
        r#"
store values as [1]

define action called leave_values with parameters unused as number:
    display "unchanged"
end action

define action called widen_values with parameters unused as number:
    push with values and "text"
end action

store selected as leave_values
define action called select_widener:
    change selected to widen_values
end action

call select_widener
call selected with 0
store removed as pop of values
open file at removed for reading as input_file
"#,
    );
}

#[test]
fn closure_defined_before_alias_assignment_can_clear_the_later_alias() {
    assert_typechecks(
        r#"
store values as [1]

define action called leave_values with parameters unused as number:
    display "unchanged"
end action

store selected as nothing
define action called clear_selected:
    change selected to nothing
end action

change selected to leave_values
call clear_selected
call selected with 0
store removed as pop of values
open file at removed for reading as input_file
"#,
    );
}
