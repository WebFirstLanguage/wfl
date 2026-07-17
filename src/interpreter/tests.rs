use super::{Environment, Interpreter, Value};
use crate::lexer::lex_wfl_with_positions;
use crate::parser::Parser;
use crate::typechecker::TypeChecker;
// use std::io::Write;

#[tokio::test]
async fn test_literal_evaluation() {
    let interpreter = Interpreter::new();
    let env = Environment::new_global();

    let source = "42";
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();

    if let Some(stmt) = program.statements.first() {
        if let crate::parser::ast::Statement::ExpressionStatement { expression, .. } = stmt {
            let result = interpreter
                .evaluate_expression(expression, env)
                .await
                .unwrap();
            match result {
                Value::Number(n) => assert_eq!(n, 42.0),
                _ => panic!("Expected number, got {result:?}"),
            }
        } else {
            panic!("Expected expression statement");
        }
    } else {
        panic!("No statements in program");
    }
}

#[tokio::test]
async fn test_variable_declaration_and_access() {
    let mut interpreter = Interpreter::new();

    let source = "store x as 42\nx";
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();

    let result = interpreter.interpret(&program).await.unwrap();

    match result {
        Value::Number(n) => assert_eq!(n, 42.0),
        _ => panic!("Expected number, got {result:?}"),
    }
}

#[tokio::test]
async fn test_binary_operations() {
    let mut interpreter = Interpreter::new();

    let source = "2 plus 3";
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    let result = interpreter.interpret(&program).await.unwrap();
    match result {
        Value::Number(n) => assert_eq!(n, 5.0),
        _ => panic!("Expected number, got {result:?}"),
    }

    let source = "2 is less than 3";
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    let result = interpreter.interpret(&program).await.unwrap();
    match result {
        Value::Bool(b) => assert!(b),
        _ => panic!("Expected boolean, got {result:?}"),
    }
}

#[tokio::test]
async fn test_if_statement() {
    let mut interpreter = Interpreter::new();

    let source = "check if yes: display \"true\" otherwise: display \"false\" end check";
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    let result = interpreter.interpret(&program).await.unwrap();

    match result {
        Value::Null => {}
        _ => panic!("Expected null, got {result:?}"),
    }
}

/*
#[tokio::test]
async fn test_function_definition_and_call() {
    let mut interpreter = Interpreter::new();

    let source = "define action called add: give back 2 plus 3 end action\nadd";
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    let result = interpreter.interpret(&program).await.unwrap();

    match result {
        Value::Number(n) => assert_eq!(n, 5.0),
        _ => panic!("Expected number, got {:?}", result),
    }
}
*/

#[tokio::test]
async fn test_count_loop_with_direct_access() {
    let mut interpreter = Interpreter::new();

    let source = "
    count from 1 to 5:
        display \"Count: \" with count
    end count
    ";
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();

    let result = interpreter.interpret(&program).await.unwrap();

    match result {
        Value::Null => {}
        _ => panic!("Expected null, got {result:?}"),
    }
}
#[tokio::test]
async fn test_timeout_happy_path() {
    // Use a longer timeout (5 seconds) to avoid flakiness under heavy test load
    let mut interpreter = Interpreter::with_timeout(5);

    let source = "store x as 42\nx"; // A quick script
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();

    let result = interpreter.interpret(&program);
    assert!(
        result.await.is_ok(),
        "Simple script should complete within timeout"
    );
}

#[tokio::test]
async fn test_timeout_forever_loop() {
    let mut interpreter = Interpreter::with_timeout(1); // 1 second timeout

    let source = "
    count from 1 to 1000000000:
        store x as 1 plus 1
    end count
    ";
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();

    let start = std::time::Instant::now();
    let result = interpreter.interpret(&program);
    let elapsed = start.elapsed();

    let result_value = result.await;
    assert!(result_value.is_err());
    if let Err(errors) = result_value {
        assert!(!errors.is_empty());
        println!("Actual error message: {}", errors[0].message);
        assert!(errors[0].message.contains("Execution exceeded timeout"));
    }

    assert!(
        elapsed.as_millis() <= 1100,
        "Timeout took too long: {elapsed:?}"
    );
}

#[tokio::test]
async fn test_type_error_blocked_by_default() {
    let input = "store x as 1\nstore x as \"oops\"";
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    assert!(!program.statements.is_empty());

    let mut tc = TypeChecker::new();
    assert!(tc.check_types(&program).is_err());
}

#[tokio::test]
async fn test_count_in_binary_operations() {
    let input = r#"
        store sum as 0
        count from 1 to 5:
            change sum to sum plus count
        end count
        sum
    "#;

    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await.unwrap();
    assert_eq!(result, Value::Number(15.0)); // Sum of numbers 1 to 5 = 15
}

#[tokio::test]
async fn test_nested_count_loops() {
    let input = r#"
        store total as 0
        count from 1 to 3:
            store outer as count
            count from 1 to 2:
                store inner as count
                change total to total plus outer times inner
            end count
        end count
        total
    "#;

    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await.unwrap();
    assert_eq!(result, Value::Number(18.0)); // (1×1 + 1×2) + (2×1 + 2×2) + (3×1 + 3×2) = 18
}

#[tokio::test]
async fn test_list_no_double_execution_of_side_effects() {
    // Verify the fix for list literal double execution: a zero-arg action placed
    // inside a list literal should only execute once, not twice.
    let input = r#"
        store counter as 0

        define action called bump:
            change counter to counter plus 1
            give back counter
        end action

        store my_list as [bump]
        counter
    "#;

    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await.unwrap();
    assert_eq!(
        result,
        Value::Number(1.0),
        "Action in list literal should execute exactly once, not twice"
    );
}

#[tokio::test]
async fn test_pattern_literal_evaluation() {
    let mut interpreter = Interpreter::new();
    let env = Environment::new_global();

    // 1. Test synchronous evaluation
    let source = "pattern \"hello\"";
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();

    if let Some(crate::parser::ast::Statement::ExpressionStatement { expression, .. }) =
        program.statements.first()
    {
        let result = interpreter
            .evaluate_expression(expression, std::rc::Rc::clone(&env))
            .await
            .unwrap();

        if let Value::Pattern(p) = result {
            assert!(p.matches("hello world"));
            assert!(!p.matches("goodbye"));
        } else {
            panic!("Expected Value::Pattern, got {:?}", result);
        }
    }

    // 2. Test async path by nesting in a list (forcing async evaluation path for elements)
    // Wait, patterns themselves are sync but lists are scanned. If a list contains async, it will evaluate elements async.
    // Let's create an async function to force it.
    let async_source = "
define action called do_async:
    wait for 1 milliseconds
end action

[pattern \"async_test\", do_async]
";
    let tokens = lex_wfl_with_positions(async_source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    let result = interpreter.interpret(&program).await.unwrap();

    if let Value::List(list_ref) = result {
        let list = list_ref.borrow();
        if let Some(Value::Pattern(p)) = list.first() {
            assert!(p.matches("hello async_test"));
        } else {
            panic!("Expected Value::Pattern at index 0");
        }
    } else {
        panic!("Expected Value::List, got {:?}", result);
    }
}

#[cfg(unix)]
#[test]
fn test_lexical_abspath() {
    use super::lexical_abspath;
    use std::path::Path;

    // Absolute paths pass through unchanged
    assert_eq!(
        lexical_abspath(Path::new("/a/b/c.wfl"), Path::new("/cwd")),
        "/a/b/c.wfl"
    );
    // Relative paths join to the cwd
    assert_eq!(
        lexical_abspath(Path::new("Tools/foo.wfl"), Path::new("/home/user/wfl")),
        "/home/user/wfl/Tools/foo.wfl"
    );
    // "." components are dropped, ".." collapses lexically (like Python os.path.abspath)
    assert_eq!(
        lexical_abspath(Path::new("./Tools/../src/x.rs"), Path::new("/base")),
        "/base/src/x.rs"
    );
    // ".." at the root is dropped, matching os.path.normpath("/../x") == "/x"
    assert_eq!(lexical_abspath(Path::new("/../x"), Path::new("/")), "/x");
    assert_eq!(
        lexical_abspath(Path::new("a/b.wfl"), Path::new("/w/./z/..")),
        "/w/a/b.wfl"
    );
}

#[cfg(windows)]
#[test]
fn test_lexical_abspath_windows() {
    use super::lexical_abspath;
    use std::path::Path;

    // Absolute paths pass through unchanged
    assert_eq!(
        lexical_abspath(Path::new(r"C:\a\b\c.wfl"), Path::new(r"C:\cwd")),
        r"C:\a\b\c.wfl"
    );
    // Relative paths join to the cwd
    assert_eq!(
        lexical_abspath(Path::new(r"Tools\foo.wfl"), Path::new(r"C:\repo")),
        r"C:\repo\Tools\foo.wfl"
    );
    // "." components are dropped, ".." collapses lexically; forward slashes
    // are accepted as separators and normalized
    assert_eq!(
        lexical_abspath(Path::new("./Tools/../src/x.rs"), Path::new(r"C:\base")),
        r"C:\base\src\x.rs"
    );
    // ".." at the root is dropped
    assert_eq!(
        lexical_abspath(Path::new(r"C:\..\x"), Path::new(r"C:\")),
        r"C:\x"
    );
}

#[tokio::test]
async fn test_script_path_and_directory_globals() {
    let mut interpreter = Interpreter::new();
    interpreter.set_source_file(std::path::PathBuf::from("Tools/rust_loc_counter.wfl"));

    let tokens = lex_wfl_with_positions("store placeholder as 1");
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    interpreter.interpret(&program).await.unwrap();

    let env = interpreter.global_env.borrow();
    let script_path = env.get("script_path").expect("script_path defined");
    let script_directory = env
        .get("script_directory")
        .expect("script_directory defined");
    match (&script_path, &script_directory) {
        (Value::Text(path_text), Value::Text(dir_text)) => {
            assert!(
                std::path::Path::new(path_text.as_ref()).is_absolute(),
                "script_path should be absolute, got {path_text}"
            );
            assert!(
                path_text.ends_with("Tools/rust_loc_counter.wfl")
                    || path_text.ends_with("Tools\\rust_loc_counter.wfl"),
                "unexpected script_path {path_text}"
            );
            assert!(
                dir_text.ends_with("Tools"),
                "unexpected script_directory {dir_text}"
            );
        }
        other => panic!("Expected Text globals, got {other:?}"),
    }
}

#[tokio::test]
async fn test_script_path_globals_empty_without_source_file() {
    let mut interpreter = Interpreter::new();

    let tokens = lex_wfl_with_positions("store placeholder as 1");
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();
    interpreter.interpret(&program).await.unwrap();

    let env = interpreter.global_env.borrow();
    assert_eq!(
        env.get("script_path"),
        Some(Value::Text(std::sync::Arc::from("")))
    );
    assert_eq!(
        env.get("script_directory"),
        Some(Value::Text(std::sync::Arc::from("")))
    );
}

// --- Issue #571 regression tests: runtime behaviour of the new constructs ---

async fn eval_program(source: &str) -> Value {
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("program should parse");
    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .expect("program should run without error")
}

#[tokio::test]
async fn test_slash_division_runtime() {
    match eval_program("10 / 4").await {
        Value::Number(n) => assert_eq!(n, 2.5),
        other => panic!("expected number, got {other:?}"),
    }
}

#[tokio::test]
async fn test_modulo_word_runtime() {
    match eval_program("12 modulo 5").await {
        Value::Number(n) => assert_eq!(n, 2.0),
        other => panic!("expected number, got {other:?}"),
    }
}

#[tokio::test]
async fn test_arithmetic_binds_tighter_than_comparison_runtime() {
    // `3 plus 1 is equal to 4` should be true, not a type error.
    match eval_program("3 plus 1 is equal to 4").await {
        Value::Bool(b) => assert!(b),
        other => panic!("expected boolean, got {other:?}"),
    }
}

#[tokio::test]
async fn test_is_between_runtime() {
    match eval_program("5 is between 1 and 10").await {
        Value::Bool(b) => assert!(b),
        other => panic!("expected boolean, got {other:?}"),
    }
    match eval_program("15 is between 1 and 10").await {
        Value::Bool(b) => assert!(!b),
        other => panic!("expected boolean, got {other:?}"),
    }
}

#[tokio::test]
async fn test_finally_runs_on_success_path() {
    let source = "store log as \"\"\n\
                  try:\n\
                  change log to log with \"body;\"\n\
                  when error:\n\
                  change log to log with \"caught;\"\n\
                  finally:\n\
                  change log to log with \"finally;\"\n\
                  end try\n\
                  log";
    match eval_program(source).await {
        Value::Text(s) => assert_eq!(&*s, "body;finally;"),
        other => panic!("expected text, got {other:?}"),
    }
}

#[tokio::test]
async fn test_finally_runs_after_caught_error_with_binding() {
    let source = "store log as \"\"\n\
                  try:\n\
                  change log to log with \"body;\"\n\
                  open file at \"/nonexistent/issue571.txt\" for reading as f\n\
                  when error as e:\n\
                  change log to log with \"caught;\"\n\
                  finally:\n\
                  change log to log with \"finally;\"\n\
                  end try\n\
                  log";
    match eval_program(source).await {
        Value::Text(s) => assert_eq!(&*s, "body;caught;finally;"),
        other => panic!("expected text, got {other:?}"),
    }
}

/// End-to-end unit regression for case-insensitive `header "Name" of req`
/// without spinning up an HTTP server. Builds a synthetic request object
/// whose headers map uses warp-style lowercase keys — the same shape
/// `wait for request` produces — and asserts canonical names still resolve.
#[tokio::test]
async fn test_header_access_case_insensitive_via_request_object() {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::Arc;

    let interpreter = Interpreter::new();
    let env = Environment::new_global();

    // Mimic warp: header names stored lowercase on the request object.
    let mut headers_map = HashMap::new();
    headers_map.insert(
        "user-agent".to_string(),
        Value::Text(Arc::from("wfl-header-test")),
    );
    headers_map.insert(
        "content-type".to_string(),
        Value::Text(Arc::from("text/plain")),
    );
    let headers_object = Value::Object(Rc::new(RefCell::new(headers_map)));

    let mut request_props = HashMap::new();
    request_props.insert("headers".to_string(), headers_object);
    let request_object = Value::Object(Rc::new(RefCell::new(request_props)));
    env.borrow_mut()
        .define("req", request_object)
        .expect("define req");

    let tokens = lex_wfl_with_positions(r#"header "User-Agent" of req"#);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("parse header access");
    let expr = match program.statements.first() {
        Some(crate::parser::ast::Statement::ExpressionStatement { expression, .. }) => expression,
        other => panic!("expected expression statement, got {other:?}"),
    };

    let result = interpreter
        .evaluate_expression(expr, Rc::clone(&env))
        .await
        .expect("header access should succeed");

    match result {
        Value::Text(s) => assert_eq!(
            s.as_ref(),
            "wfl-header-test",
            "header \"User-Agent\" of req must find lowercase 'user-agent'"
        ),
        other => panic!("expected text, got {other:?}"),
    }

    // Missing header → nothing (Value::Null)
    let tokens = lex_wfl_with_positions(r#"header "X-Missing" of req"#);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("parse missing header access");
    let expr = match program.statements.first() {
        Some(crate::parser::ast::Statement::ExpressionStatement { expression, .. }) => expression,
        other => panic!("expected expression statement, got {other:?}"),
    };
    let result = interpreter
        .evaluate_expression(expr, env)
        .await
        .expect("missing header should not error");
    assert!(
        matches!(result, Value::Null),
        "absent header should be nothing, got {result:?}"
    );
}

/// A WFL program can insert a list into itself through `push`. Displaying that
/// value used to recurse on the native stack until the whole process aborted.
#[tokio::test]
async fn test_display_self_referential_list_is_cycle_safe() {
    let source = r#"
create list items:
end list
push with items and items
display items
"#;
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("parse self-referential list program");
    let mut interpreter = Interpreter::new();

    let output = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let result = {
        let _capture = super::io_capture::push_capture(std::rc::Rc::clone(&output));
        interpreter.interpret(&program).await
    };

    result.expect("displaying a self-referential list must not abort or error");
    assert_eq!(&*output.borrow(), "[<cycle>]\n");
}
