//! Shared helpers for integration tests.
#![allow(dead_code)]

use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;
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

/// Spawn a background thread with the CLI-sized interpreter stack.
///
/// Web-server integration tests that drive [`Interpreter::interpret`] on a
/// dedicated thread must use this instead of [`std::thread::spawn`]: session-
/// aware interpreter paths overflow the default ~2 MiB OS thread stack under
/// concurrent handler load in debug builds.
pub fn spawn_interpreter_thread<F>(work: F) -> std::thread::JoinHandle<()>
where
    F: FnOnce() + Send + 'static,
{
    std::thread::Builder::new()
        .name("wfl-interpreter".to_string())
        .stack_size(wfl::INTERPRETER_STACK_SIZE)
        .spawn(work)
        .expect("spawn interpreter thread")
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

// ---------------------------------------------------------------------------
// Real-binary driver: spawn the actual `wfl` executable and capture its
// output. Unlike the in-memory shapes above, these tests exercise the real
// CLI boundary (argument parsing, process exit codes, file I/O), so they
// spawn a subprocess rather than calling into the interpreter directly.
// ---------------------------------------------------------------------------

/// Absolute path to the `wfl` binary Cargo built for *this* test run.
/// `CARGO_BIN_EXE_wfl` is injected by Cargo, so it always points at the
/// freshly-built binary matching the current test profile (debug under plain
/// `cargo test`, release under `cargo test --release`) — no stale-binary risk
/// and no cwd assumption.
pub fn wfl_exe() -> &'static str {
    env!("CARGO_BIN_EXE_wfl")
}

/// Path to the separately-built `target/release/wfl` binary. This is a
/// *different* binary from [`wfl_exe`] (which may point at a debug build) —
/// callers that need the release binary specifically (e.g. because a sibling
/// helper in the same file already assumes it, or the test predates
/// `CARGO_BIN_EXE_wfl` and was never migrated) use this instead.
///
/// Unlike [`wfl_exe`], this binary is **not** built by `cargo test`; it has to
/// exist already. The path is anchored to `CARGO_MANIFEST_DIR` rather than a
/// bare relative path so it does not depend on the test process's working
/// directory, and a missing binary fails with an actionable message instead of
/// a bare `Os { code: 2, kind: NotFound }` from the eventual spawn.
pub fn wfl_release_exe() -> PathBuf {
    let name = if cfg!(target_os = "windows") {
        "wfl.exe"
    } else {
        "wfl"
    };
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join(name);
    assert!(
        path.exists(),
        "release binary not found at {}\n\
         This test runs the separately-built release binary; `cargo test` does not build it.\n\
         Run `cargo build --release` first.",
        path.display()
    );
    path
}

/// Run inline WFL source (via [`wfl_exe`]) in a fresh temp dir, returning
/// (combined stdout+stderr, exit code).
pub fn run_src(src: &str) -> (String, Option<i32>) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("main.wfl");
    fs::write(&path, src).unwrap();
    let output = Command::new(wfl_exe())
        .arg(&path)
        .output()
        .expect("failed to execute WFL");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    drop(dir);
    (combined, output.status.code())
}

/// Run a WFL file that already lives inside `dir` (via [`wfl_release_exe`]),
/// so relative paths like `include from`/`load module from` resolve to
/// sibling files. Returns (combined stdout+stderr, exit code).
pub fn run_file_status(dir: &TempDir, name: &str, extra_args: &[&str]) -> (String, Option<i32>) {
    let path = dir.path().join(name);
    let output = Command::new(wfl_release_exe())
        .args(extra_args)
        .arg(&path)
        .output()
        .expect("Failed to execute WFL");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (combined, output.status.code())
}
