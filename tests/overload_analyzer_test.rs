//! Analyzer tests for user-defined action overloading.
//!
//! Overloading rules (definition time):
//! - Two same-scope actions with the same name are legal when they differ in
//!   parameter count, or in at least one position where BOTH declare concrete,
//!   different parameter types (`as number` vs `as text`).
//! - Exact duplicates (same arity, same normalized type vector) are rejected.
//! - Indistinguishable same-arity pairs (e.g. `f(x)` vs `f(x as number)`) are
//!   rejected, because an untyped parameter accepts numbers too.
//!
//! Call-site rules:
//! - Candidates are filtered by arity, then by static argument types.
//! - No arity match / no type match are errors listing the candidates.
//! - Statically ambiguous calls (Unknown-typed arguments) defer to runtime
//!   dispatch with no analyzer error.

use wfl::analyzer::Analyzer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

fn analyze_errors(code: &str) -> Vec<String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .unwrap_or_else(|e| panic!("Failed to parse {code:?}: {e:?}"));

    let mut analyzer = Analyzer::new();
    match analyzer.analyze(&program) {
        Ok(()) => Vec::new(),
        Err(errors) => errors.into_iter().map(|e| e.message).collect(),
    }
}

// --- Definition-time rules --------------------------------------------------

#[test]
fn overload_by_arity_accepted() {
    let errors = analyze_errors(
        r#"
define action called greet with parameters name:
    display name
end action

define action called greet with parameters first and last:
    display first with last
end action

call greet with "a"
call greet with "a" and "b"
"#,
    );
    assert!(
        errors.is_empty(),
        "arity-distinct overloads should be accepted: {errors:?}"
    );
}

#[test]
fn typed_same_arity_accepted() {
    let errors = analyze_errors(
        r#"
define action called depict with parameters value as number:
    display value
end action

define action called depict with parameters value as text:
    display value
end action

call depict with 5
call depict with "hello"
"#,
    );
    assert!(
        errors.is_empty(),
        "type-distinct same-arity overloads should be accepted: {errors:?}"
    );
}

#[test]
fn exact_duplicate_rejected() {
    let errors = analyze_errors(
        r#"
define action called greet with parameters name as text:
    display name
end action

define action called greet with parameters other as text:
    display other
end action
"#,
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("greet") && e.contains("same parameters")),
        "exact duplicate signature must be rejected: {errors:?}"
    );
}

#[test]
fn untyped_duplicate_rejected() {
    let errors = analyze_errors(
        r#"
define action called greet with parameters name:
    display name
end action

define action called greet with parameters other:
    display other
end action
"#,
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("greet") && e.contains("same parameters")),
        "two untyped same-arity definitions must be rejected: {errors:?}"
    );
}

#[test]
fn ambiguous_same_arity_rejected() {
    let errors = analyze_errors(
        r#"
define action called f with parameters x:
    display x
end action

define action called f with parameters x as number:
    display x
end action
"#,
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("f") && e.contains("cannot be told apart")),
        "untyped vs typed same-arity pair must be rejected as ambiguous: {errors:?}"
    );
}

#[test]
fn variable_function_collision_still_rejected() {
    let errors = analyze_errors(
        r#"
store greet as 5

define action called greet with parameters name:
    display name
end action
"#,
    );
    assert!(
        errors.iter().any(|e| e.contains("already been defined")),
        "variable/function name collision must remain an error: {errors:?}"
    );
}

// --- Call-site resolution ---------------------------------------------------

#[test]
fn call_no_arity_match_lists_arities() {
    let errors = analyze_errors(
        r#"
define action called f with parameters x:
    display x
end action

define action called f with parameters x and y:
    display x with y
end action

call f with 1 and 2 and 3
"#,
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains('f') && e.contains('3') && e.contains('1') && e.contains('2')),
        "wrong-arity call must list available arities: {errors:?}"
    );
}

#[test]
fn call_no_type_match_lists_candidates() {
    let errors = analyze_errors(
        r#"
define action called f with parameters x as text:
    display x
end action

define action called f with parameters x as boolean:
    display x
end action

call f with 5
"#,
    );
    assert!(
        errors.iter().any(|e| e.contains("No version of 'f'")),
        "no-type-match call must produce a candidate-listing error: {errors:?}"
    );
}

#[test]
fn call_dynamic_arg_deferred_without_error() {
    let errors = analyze_errors(
        r#"
define action called f with parameters x as number:
    display x
end action

define action called f with parameters x as text:
    display x
end action

define action called g with parameters v:
    call f with v
end action
"#,
    );
    assert!(
        errors.is_empty(),
        "a dynamically-typed argument must defer to runtime, not error: {errors:?}"
    );
}

#[test]
fn of_form_and_call_form_agree() {
    let of_form = analyze_errors(
        r#"
define action called f with parameters x as text:
    display x
end action

define action called f with parameters x as boolean:
    display x
end action

store r as f of 5
"#,
    );
    let call_form = analyze_errors(
        r#"
define action called f with parameters x as text:
    display x
end action

define action called f with parameters x as boolean:
    display x
end action

store r as call f with 5
"#,
    );
    assert!(
        of_form.iter().any(|e| e.contains("No version of 'f'")),
        "of-form must reject a no-type-match call: {of_form:?}"
    );
    assert!(
        call_form.iter().any(|e| e.contains("No version of 'f'")),
        "call-form must reject a no-type-match call: {call_form:?}"
    );
}

#[test]
fn nothing_and_pattern_type_annotations_parse() {
    // `nothing` lexes as NothingLiteral and `pattern`/`text` as keywords, so
    // type positions must accept those tokens, not just identifiers
    // (PR #639 review).
    let errors = analyze_errors(
        r#"
define action called f with parameters x as nothing:
    display "nothing"
end action

define action called f with parameters x as number:
    display x
end action
"#,
    );
    assert!(
        errors.is_empty(),
        "'as nothing' must parse and overload against 'as number': {errors:?}"
    );
}

#[test]
fn single_signature_behavior_unchanged() {
    // The classic single-definition path must keep its existing diagnostics.
    let errors = analyze_errors(
        r#"
define action called greet with parameters name:
    display name
end action

call greet with "a" and "b"
"#,
    );
    assert!(
        !errors.is_empty(),
        "wrong arity against a single signature must still error"
    );
}
