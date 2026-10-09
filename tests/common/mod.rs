//! Shared helpers for integration tests.
#![allow(dead_code)]

use std::fs;
use std::net::{IpAddr, SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tempfile::TempDir;
use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Ask the OS for a currently-free TCP port on loopback, then release it so the
/// caller can bind it via WFL's `listen on port <N>`.
///
/// Unsafe in a shared suite process: dropping the probe socket lets the kernel
/// hand that port to another test's `bind(:0)` before WFL rebinds it. Suite
/// tests that start a WFL server must `listen on port 0` and read the published
/// handle instead (see [`published_web_server_addr`]). This helper remains for
/// standalone Group A binaries that still interpolate a port into source.
pub fn free_tcp_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind an ephemeral TCP port")
        .local_addr()
        .expect("read the ephemeral local address")
        .port()
}

/// Parse `PREFIXWebServer::ip:port` from a child's published readiness output.
///
/// Only complete newline-terminated lines are accepted so a partial write is
/// not treated as a bound address. The child must own the port before it
/// prints this line (`listen on port 0` then `display`). WFL formats the
/// handle as `WebServer::` + `addr.ip()` + `:` + port, so a `::1` bind is
/// `WebServer::::1:port` and a `::` bind is `WebServer:::::port`. After
/// stripping the marker, split host/port on the last colon and parse the
/// host as [`IpAddr`] (bare IPv6, no brackets).
pub fn published_web_server_addr(log: &str, prefix: &str) -> Option<SocketAddr> {
    let marker = format!("{prefix}WebServer::");
    log.split_inclusive('\n')
        .filter(|line| line.ends_with('\n'))
        .find_map(|line| {
            line.trim_end()
                .strip_prefix(marker.as_str())
                .and_then(|rest| {
                    let (ip, port) = rest.rsplit_once(':')?;
                    let port: u16 = port.parse().ok()?;
                    let ip: IpAddr = ip.parse().ok()?;
                    Some(SocketAddr::new(ip, port))
                })
        })
}

static READY_SEQ: AtomicU64 = AtomicU64::new(0);

/// Unique temp path for an in-process server to publish its bound address.
pub fn unique_ready_path(tag: &str) -> PathBuf {
    let n = READY_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("wfl_ready_{tag}_{}_{n}.txt", std::process::id()));
    let _ = fs::remove_file(&path);
    path
}

/// WFL snippet: write `PREFIX` plus the server handle, then a newline, to `ready_path`.
///
/// Call after `listen on port 0` so the child owns the socket before anything
/// connects. Variable names are prefixed to avoid colliding with test programs.
pub fn publish_ready_wfl(ready_path: &Path, prefix: &str, server_var: &str) -> String {
    let ready = ready_path.display().to_string().replace('\\', "/");
    format!(
        "store suite_ready_line as \"{prefix}\" with {server_var} with \"\\n\"\n\
         open file at \"{ready}\" for writing as suite_ready_file\n\
         wait for write content suite_ready_line into suite_ready_file\n\
         close file suite_ready_file\n"
    )
}

/// Poll `ready_path` until a published `PREFIXWebServer::ip:port` line appears.
pub async fn wait_for_published_web_server(ready_path: &Path, prefix: &str) -> SocketAddr {
    try_wait_for_published_web_server(ready_path, prefix, Duration::from_secs(10))
        .await
        .unwrap_or_else(|| {
            panic!(
                "server did not publish {prefix} at {}",
                ready_path.display()
            )
        })
}

/// Like [`wait_for_published_web_server`], but returns `None` on timeout.
pub async fn try_wait_for_published_web_server(
    ready_path: &Path,
    prefix: &str,
    timeout: Duration,
) -> Option<SocketAddr> {
    let deadline = Instant::now() + timeout;
    let marker = format!("{prefix}WebServer::");
    loop {
        if let Ok(log) = fs::read_to_string(ready_path) {
            if let Some(address) = published_web_server_addr(&log, prefix) {
                assert_ne!(address.port(), 0, "server must report its assigned port");
                return Some(address);
            }
            let published_unparsed = log
                .split_inclusive('\n')
                .any(|line| line.ends_with('\n') && line.trim_end().starts_with(marker.as_str()));
            assert!(
                !published_unparsed,
                "published {prefix}WebServer line did not parse: {log}"
            );
        }
        if Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
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

/// Panic with an actionable message when a separately-built release binary is missing.
pub fn require_existing_release_binary(path: PathBuf) -> PathBuf {
    assert!(
        path.exists(),
        "release binary not found at {}\n\
         This test runs the separately-built release binary; `cargo test` does not build it.\n\
         Run `cargo build --release` first.",
        path.display()
    );
    path
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
    require_existing_release_binary(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("release")
            .join(name),
    )
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
