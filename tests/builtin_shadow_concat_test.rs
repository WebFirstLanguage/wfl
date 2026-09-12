//! Builtin registration must not change `with` on an existing value binding.
mod common;

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, Program, Statement};

const NEW_BUILTINS: &[&str] = &[
    "password_hash_policy",
    "hash_password_with_policy",
    "password_needs_rehash",
    "create_session_store",
    "session_create",
    "session_lookup",
    "session_rotate",
    "session_revoke",
    "session_revoke_account",
    "session_csrf_guard",
    "session_cookie",
    "create_account_rate_limiter",
    "account_rate_limit_allow",
];

fn parse(source: &str) -> Program {
    Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .expect("valid source")
}

fn assert_concatenation(expression: &Expression) {
    assert!(
        matches!(expression, Expression::Concatenation { .. }),
        "value binding must concatenate: {expression:?}"
    );
}

fn assert_output(source: &str, expected: &str) {
    let (output, status) = common::run_src(source);
    assert_eq!(status, Some(0), "{output}");
    assert_eq!(output.trim(), expected, "{source}");
}

#[test]
fn stored_and_constant_builtin_names_keep_concatenation() {
    for name in NEW_BUILTINS {
        for declaration in ["store", "store new constant"] {
            let program = parse(&format!(
                "{declaration} {name} as \"prefix\"\ndisplay {name} with \"value\"\n"
            ));
            let Statement::DisplayStatement { value, .. } = &program.statements[1] else {
                panic!("display")
            };
            assert_concatenation(value);
        }
    }
    assert_output(
        "store session_cookie as \"prefix\"\ndisplay session_cookie with \"value\"\n",
        "prefixvalue",
    );
}

#[test]
fn shadowed_names_concatenate_in_nested_and_clause_expressions() {
    let program = parse(
        r#"store session_cookie as "prefix"
display "start:" with session_cookie with "value"
store wrapped as (session_cookie with "value")
start streaming response to req with content type session_cookie with "value" and status 200 as out
"#,
    );
    let Statement::DisplayStatement {
        value: Expression::Concatenation { right, .. },
        ..
    } = &program.statements[1]
    else {
        panic!("outer concatenation")
    };
    assert_concatenation(right);
    let Statement::VariableDeclaration { value, .. } = &program.statements[2] else {
        panic!("store")
    };
    assert_concatenation(value);
    let Statement::StartStreamingResponseStatement {
        content_type: Some(content),
        ..
    } = &program.statements[3]
    else {
        panic!("streaming response")
    };
    assert_concatenation(content);
}

#[test]
fn action_parameters_and_locals_do_not_leak_into_outer_builtin_calls() {
    let token = "a".repeat(64);
    let source = format!(
        r#"define action called append_value with parameters session_cookie:
    return session_cookie with "value"
end action
define action called local_label:
    store session_cookie as "local"
    return session_cookie with "value"
end action
display append_value of "prefix"
display local_label
display session_cookie with "{token}"
"#
    );
    assert_output(
        &source,
        &format!(
            "prefixvalue\nlocalvalue\n__Host-wfl_session={token}; Path=/; Secure; HttpOnly; SameSite=Strict"
        ),
    );
}

#[test]
fn foreach_binding_is_local_and_callable_aliases_keep_working() {
    let token = "a".repeat(64);
    let source = format!(
        r#"for each session_cookie in ["prefix"]:
    display session_cookie with "value"
end for
store cookie_builder as session_cookie
display cookie_builder of "{token}"
store explicit_cookie as call cookie_builder with "{token}"
display explicit_cookie
display session_cookie with "{token}"
"#
    );
    let cookie = format!("__Host-wfl_session={token}; Path=/; Secure; HttpOnly; SameSite=Strict");
    assert_output(
        &source,
        &format!("prefixvalue\n{cookie}\n{cookie}\n{cookie}"),
    );
}

#[test]
fn callable_shadow_of_an_existing_builtin_preserves_shorthand() {
    assert_output(
        "store substring as touppercase\ndisplay substring with \"abc\"\n",
        "ABC",
    );
}

#[test]
fn request_and_caught_error_bindings_are_values_for_with() {
    let program = parse(
        r#"wait for request comes in on webserver as session_cookie
display session_cookie with "value"
"#,
    );
    let Statement::DisplayStatement { value, .. } = &program.statements[1] else {
        panic!("display")
    };
    assert_concatenation(value);
    let program = parse(
        r#"try:
    store invalid as parse_json of "bad"
when error as session_cookie:
    display session_cookie with "value"
end try
"#,
    );
    let Statement::TryStatement { when_clauses, .. } = &program.statements[0] else {
        panic!("try")
    };
    let Statement::DisplayStatement { value, .. } = &when_clauses[0].body[0] else {
        panic!("display")
    };
    assert_concatenation(value);
}
