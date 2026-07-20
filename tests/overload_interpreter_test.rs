//! Interpreter tests for user-defined action overloading: runtime dispatch by
//! arity, then by declared parameter types against the runtime argument
//! values, picking the most specific match (ties resolve to definition order).

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

async fn run(code: &str) -> Result<Interpreter, String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .unwrap_or_else(|e| panic!("Failed to parse {code:?}: {e:?}"));

    let mut interpreter = Interpreter::new();
    match interpreter.interpret(&program).await {
        Ok(_) => Ok(interpreter),
        Err(errors) => Err(errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ")),
    }
}

async fn run_err(code: &str, why: &str) -> String {
    match run(code).await {
        Ok(_) => panic!("{why}"),
        Err(e) => e,
    }
}

fn global_text(interpreter: &Interpreter, name: &str) -> String {
    match interpreter.global_env().borrow().get(name) {
        Some(Value::Text(t)) => t.to_string(),
        other => panic!("expected Text in '{name}', got {other:?}"),
    }
}

fn global_number(interpreter: &Interpreter, name: &str) -> f64 {
    match interpreter.global_env().borrow().get(name) {
        Some(Value::Number(n)) => n,
        other => panic!("expected Number in '{name}', got {other:?}"),
    }
}

#[tokio::test]
async fn arity_dispatch_runtime() {
    let interp = run(r#"
define action called f with parameters x:
    return "one"
end action

define action called f with parameters x and y:
    return "two"
end action

store r1 as f of 1
store r2 as f of 1 and 2
"#)
    .await
    .expect("program should run");
    assert_eq!(global_text(&interp, "r1"), "one");
    assert_eq!(global_text(&interp, "r2"), "two");
}

#[tokio::test]
async fn type_dispatch_runtime() {
    let interp = run(r#"
define action called f with parameters x as number:
    return "num"
end action

define action called f with parameters x as text:
    return "text"
end action

store r1 as f of 5
store r2 as f of "hello"

store v as 7
store r3 as f of v
"#)
    .await
    .expect("program should run");
    assert_eq!(global_text(&interp, "r1"), "num");
    assert_eq!(global_text(&interp, "r2"), "text");
    assert_eq!(global_text(&interp, "r3"), "num");
}

#[tokio::test]
async fn both_call_forms_dispatch() {
    let interp = run(r#"
define action called f with parameters x as number:
    return "num"
end action

define action called f with parameters x as text:
    return "text"
end action

store r1 as call f with 5
store r2 as call f with "hello"
"#)
    .await
    .expect("program should run");
    assert_eq!(global_text(&interp, "r1"), "num");
    assert_eq!(global_text(&interp, "r2"), "text");
}

#[tokio::test]
async fn partially_annotated_same_arity_rejected() {
    // No parameter position where BOTH overloads declare concrete, different
    // types — some calls (e.g. `f of 1 and "b"`) would match both, so the
    // pair is rejected at definition time, mirroring the analyzer's rule.
    let err = run_err(
        r#"
define action called f with parameters x as number and y:
    return "first"
end action

define action called f with parameters x and y as text:
    return "second"
end action
"#,
        "a partially-annotated indistinguishable pair must be rejected",
    )
    .await;
    assert!(
        err.contains('f') && err.contains("parameter"),
        "ambiguity error should explain the rule: {err}"
    );
}

#[tokio::test]
async fn cross_annotated_same_arity_dispatch() {
    let interp = run(r#"
define action called f with parameters x as number and y as text:
    return "first"
end action

define action called f with parameters x as text and y as number:
    return "second"
end action

store r1 as f of 1 and "b"
store r2 as f of "a" and 2
"#)
    .await
    .expect("program should run");
    assert_eq!(global_text(&interp, "r1"), "first");
    assert_eq!(global_text(&interp, "r2"), "second");
}

#[tokio::test]
async fn no_match_runtime_error_lists_candidates() {
    let err = run_err(
        r#"
define action called f with parameters x as number:
    return "num"
end action

define action called f with parameters x as text:
    return "text"
end action

store r as f of yes
"#,
        "boolean argument should not match any overload",
    )
    .await;
    assert!(
        err.contains("No version of 'f'"),
        "runtime no-match error should list candidates: {err}"
    );
}

#[tokio::test]
async fn wrong_arity_runtime_error() {
    let err = run_err(
        r#"
define action called f with parameters x:
    return "one"
end action

define action called f with parameters x and y:
    return "two"
end action

store r as f of 1 and 2 and 3
"#,
        "3 arguments should not match 1- or 2-parameter overloads",
    )
    .await;
    assert!(
        err.contains('f') && err.contains('3'),
        "wrong-arity error should mention the count: {err}"
    );
}

#[tokio::test]
async fn overloaded_name_as_first_class_value() {
    let interp = run(r#"
define action called f with parameters x as number:
    return "num"
end action

define action called f with parameters x as text:
    return "text"
end action

store h as f
store r as h of 5
"#)
    .await
    .expect("program should run");
    assert_eq!(global_text(&interp, "r"), "num");
}

#[tokio::test]
async fn zero_arg_overload_auto_calls_on_bare_reference() {
    let interp = run(r#"
define action called g:
    return 42
end action

define action called g with parameters x:
    return 0
end action

store r as g
"#)
    .await
    .expect("program should run");
    assert_eq!(global_number(&interp, "r"), 42.0);
}

#[tokio::test]
async fn overload_closures_capture_definitions() {
    let interp = run(r#"
store base as 10

define action called f with parameters x as number:
    return base plus x
end action

define action called f with parameters x as text:
    return x
end action

store r as f of 5
"#)
    .await
    .expect("program should run");
    assert_eq!(global_number(&interp, "r"), 15.0);
}

/// Interpreter call frames are very large in debug builds, so even shallow
/// WFL recursion can overflow the default test-thread stack; give this test a
/// generous one (the release binary used by TestPrograms has no such issue).
#[test]
fn recursion_across_sibling_overloads() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime")
                .block_on(async {
                    let interp = run(r#"
define action called fact with parameters n:
    return fact of n and 1
end action

define action called fact with parameters n and acc:
    check if n is less than 2:
        return acc
    end check
    return fact of (n minus 1) and (acc times n)
end action

store r as fact of 5
"#)
                    .await
                    .expect("program should run");
                    assert_eq!(global_number(&interp, "r"), 120.0);
                })
        })
        .expect("spawn test thread")
        .join()
        .expect("test thread panicked");
}

#[tokio::test]
async fn exact_duplicate_still_rejected_at_runtime() {
    let err = run_err(
        r#"
define action called f with parameters x:
    return 1
end action

define action called f with parameters y:
    return 2
end action
"#,
        "an exact duplicate definition must stay an error",
    )
    .await;
    assert!(
        err.contains('f'),
        "duplicate-definition error should name the action: {err}"
    );
}

#[tokio::test]
async fn nested_scope_shadowing_still_rejected() {
    let err = run_err(
        r#"
define action called outer:
    define action called outer:
        return 1
    end action
    return 2
end action

store r as outer
"#,
        "shadowing an outer action in a nested scope must stay an error",
    )
    .await;
    assert!(
        err.contains("already been defined"),
        "nested shadowing should keep today's error: {err}"
    );
}
