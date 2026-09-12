//! Builtin registration must not change `with` on an existing value binding.
mod common;

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, Program, Statement};
use wfl::repl::ReplState;

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
fn action_parameters_and_local_values_keep_concatenation() {
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
display session_cookie of "{token}"
display call local_label
"#
    );
    assert_output(
        &source,
        &format!(
            "prefixvalue\n__Host-wfl_session={token}; Path=/; Secure; HttpOnly; SameSite=Strict\nlocalvalue"
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
display call session_cookie with "{token}"
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
        "store touppercase as tolowercase\ndisplay touppercase with \"ABC\"\n",
        "abc",
    );
}

#[test]
fn legacy_shorthand_keeps_its_existing_parse_regardless_of_assignments() {
    let program = parse(
        r#"store substring as touppercase
display substring with "abc"
change substring to "prefix"
display substring with "value"
change substring to touppercase
display substring with "abc"
"#,
    );
    // The legacy grammar is unchanged, including errors for scalar callees.
    // New builtin registration must not introduce source-dependent inference.
    for index in [1, 3, 5] {
        let Statement::DisplayStatement { value, .. } = &program.statements[index] else {
            panic!("display")
        };
        assert!(matches!(value, Expression::ActionCall { .. }));
    }
}

#[tokio::test]
async fn existing_shorthand_is_unaffected_by_unexecuted_assignments() {
    let interpreter = common::run_wfl(
        r#"store touppercase as tolowercase
check if no:
    change touppercase to "prefix"
end check
store result as touppercase with "ABC"
"#,
    )
    .await
    .expect("the unexecuted branch cannot change the call");
    assert_eq!(common::get_text(&interpreter, "result"), "abc");
}

#[tokio::test]
async fn externally_injected_binding_keeps_concatenation() {
    let mut interpreter = Interpreter::new();
    interpreter
        .global_env()
        .borrow_mut()
        .define_or_replace("session_cookie", Value::Text("prefix".into()));
    interpreter
        .interpret(&parse("store result as session_cookie with \"value\"\n"))
        .await
        .expect("injected value binding must concatenate");
    assert_eq!(common::get_text(&interpreter, "result"), "prefixvalue");
}

#[tokio::test]
async fn repl_binding_keeps_concatenation_across_submissions() {
    let mut repl = ReplState::new();
    let stored = repl
        .process_line("store session_cookie as \"prefix\"")
        .await
        .unwrap();
    assert_eq!(stored, None);
    let output = repl
        .process_line("session_cookie with \"value\"")
        .await
        .unwrap();
    assert_eq!(output.as_deref(), Some("prefixvalue"));
}

#[test]
fn included_binding_keeps_concatenation() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("bindings.wfl"),
        "store session_cookie as \"prefix\"\n",
    )
    .unwrap();
    let main = dir.path().join("main.wfl");
    std::fs::write(
        &main,
        "include from \"bindings.wfl\"\ndisplay session_cookie with \"value\"\n",
    )
    .unwrap();
    let output = std::process::Command::new(common::wfl_exe())
        .current_dir(dir.path())
        .arg(main)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "prefixvalue"
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
