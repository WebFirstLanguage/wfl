//! Shared helpers for integration tests.
#![allow(dead_code)]

use std::net::TcpListener;

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Ask the OS for a currently-free TCP port on loopback, then release it so the
/// caller can bind it via WFL's `listen on port <N>`.
///
/// WFL takes a *literal* port in `listen on port <N>`, so the port must be chosen
/// before the program source is built — we cannot bind an ephemeral `:0` and read
/// the assigned port back the way the mock upstreams do. Picking a free port from
/// the OS (instead of a hardcoded constant) avoids collisions under parallel test
/// runs and on busy runners. A small TOCTOU window remains between releasing the
/// probe socket and WFL re-binding it, but it is far less flaky than a fixed port.
pub fn free_tcp_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind an ephemeral TCP port")
        .local_addr()
        .expect("read the ephemeral local address")
        .port()
}

// ---------------------------------------------------------------------------
// Shape A: run WFL source, get back `Result<Interpreter, String>` for
// inspecting arbitrary globals afterwards.
// ---------------------------------------------------------------------------

/// Run WFL code and return the interpreter for inspecting globals.
pub async fn run_wfl(code: &str) -> Result<Interpreter, String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().map_err(|e| format!("Parse error: {e:?}"))?;

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&ast)
        .await
        .map_err(|e| format!("Runtime error: {e:?}"))?;
    Ok(interpreter)
}

pub fn get_global(interpreter: &Interpreter, name: &str) -> Value {
    interpreter
        .global_env()
        .borrow()
        .get(name)
        .unwrap_or_else(|| panic!("Variable '{name}' not found"))
}

pub fn expect_text(value: &Value) -> String {
    match value {
        Value::Text(t) => t.to_string(),
        other => panic!("Expected text, got {other:?}"),
    }
}

pub fn expect_number(value: &Value) -> f64 {
    match value {
        Value::Number(n) => *n,
        other => panic!("Expected number, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Shape B: run WFL source, get back `Result<Value, String>` read from the
// `result` global.
// ---------------------------------------------------------------------------

/// Run WFL code and return the value stored in the `result` global.
pub async fn run_wfl_code(code: &str) -> Result<Value, String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().map_err(|e| format!("Parse error: {e:?}"))?;

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&ast)
        .await
        .map_err(|e| format!("Runtime error: {e:?}"))?;

    if let Some(result_value) = interpreter.global_env().borrow().get("result") {
        Ok(result_value)
    } else {
        Err("Variable 'result' not found after execution".to_string())
    }
}

pub fn expect_text_result(result: Result<Value, String>) -> String {
    match result {
        Ok(Value::Text(t)) => t.to_string(),
        other => panic!("Expected text result, got {other:?}"),
    }
}

pub fn expect_bool_result(result: Result<Value, String>) -> bool {
    match result {
        Ok(Value::Bool(b)) => b,
        other => panic!("Expected bool result, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Shape C: run WFL source, get back a bare `Interpreter` — parse/runtime
// errors panic immediately instead of being returned.
// ---------------------------------------------------------------------------

/// Run WFL code and return the interpreter, panicking on parse/runtime errors.
pub async fn run_wfl_ok(code: &str) -> Interpreter {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .unwrap_or_else(|e| panic!("Parse error: {e:?}"));
    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .unwrap_or_else(|e| panic!("Runtime error: {e:?}"));
    interpreter
}

pub fn get_var(interpreter: &Interpreter, name: &str) -> Value {
    interpreter
        .global_env()
        .borrow()
        .get(name)
        .unwrap_or_else(|| panic!("Variable '{name}' not found"))
}

pub fn get_text(interpreter: &Interpreter, name: &str) -> String {
    match get_var(interpreter, name) {
        Value::Text(t) => t.to_string(),
        other => panic!("Expected '{name}' to be text, got {other:?}"),
    }
}

pub fn get_number(interpreter: &Interpreter, name: &str) -> f64 {
    match get_var(interpreter, name) {
        Value::Number(n) => n,
        other => panic!("Expected '{name}' to be a number, got {other:?}"),
    }
}
