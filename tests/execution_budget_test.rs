//! Integration tests for the shared `ExecutionBudget` (see `src/exec/budget.rs`).
//!
//! Two layers:
//!   1. The new `.wflcfg` budget keys parse into `WflConfig` (default, override,
//!      zero/garbage rejection) via the public `load_config` API.
//!   2. End-to-end: running a WFL program actually enforces the budget — the
//!      recursion ceiling turns runaway recursion into a clean, catchable error
//!      instead of a native stack overflow, and the source-size ceiling refuses
//!      an oversized program before it runs.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use wfl::config::load_config;

mod test_helpers;

/// Write a `.wflcfg` with the given body into a fresh temp dir and load it.
fn load_with_cfg(body: &str) -> wfl::config::WflConfig {
    let dir = tempfile::tempdir().expect("create temp dir");
    fs::write(dir.path().join(".wflcfg"), body).expect("write .wflcfg");
    load_config(dir.path())
}

/// Run a WFL `program` in a fresh temp dir alongside an optional `.wflcfg`, so a
/// budget knob can be exercised end-to-end without disturbing the shared temp
/// dir other integration tests use. Returns the process output.
fn run_with_cfg(cfg: Option<&str>, program: &str) -> std::process::Output {
    let binary = test_helpers::get_wfl_binary_path();
    let dir = tempfile::tempdir().expect("create temp dir");
    if let Some(cfg) = cfg {
        fs::write(dir.path().join(".wflcfg"), cfg).expect("write .wflcfg");
    }
    let script: PathBuf = dir.path().join("program.wfl");
    fs::write(&script, program).expect("write program");
    Command::new(binary)
        .arg(&script)
        .output()
        .expect("run wfl binary")
}

// --- config parsing --------------------------------------------------------

#[test]
fn budget_keys_use_documented_defaults() {
    let cfg = load_with_cfg("# empty\n");
    assert_eq!(cfg.max_operations, None);
    assert_eq!(cfg.max_call_depth, 1_000);
    assert_eq!(cfg.max_import_depth, 64);
    assert_eq!(cfg.max_execute_file_depth, 4);
    assert_eq!(cfg.max_pattern_steps, 100_000);
    assert_eq!(cfg.max_pattern_states, 10_000);
    assert_eq!(cfg.max_source_size, 64 * 1024 * 1024);
    assert_eq!(cfg.web_server_max_response_size, 64 * 1024 * 1024);
    assert_eq!(cfg.web_socket_queue_bound, 1_024);
    assert_eq!(cfg.web_socket_max_connections, 1_024);
}

#[test]
fn budget_keys_accept_overrides() {
    let cfg = load_with_cfg(
        "max_call_depth = 250\n\
         max_import_depth = 8\n\
         max_execute_file_depth = 2\n\
         max_pattern_steps = 5000\n\
         max_pattern_states = 500\n\
         max_source_size = 4096\n\
         web_server_max_response_size = 2048\n\
         web_socket_queue_bound = 32\n\
         web_socket_max_connections = 16\n",
    );
    assert_eq!(cfg.max_call_depth, 250);
    assert_eq!(cfg.max_import_depth, 8);
    assert_eq!(cfg.max_execute_file_depth, 2);
    assert_eq!(cfg.max_pattern_steps, 5000);
    assert_eq!(cfg.max_pattern_states, 500);
    assert_eq!(cfg.max_source_size, 4096);
    assert_eq!(cfg.web_server_max_response_size, 2048);
    assert_eq!(cfg.web_socket_queue_bound, 32);
    assert_eq!(cfg.web_socket_max_connections, 16);
}

#[test]
fn max_operations_zero_means_unlimited() {
    // 0 is the documented "no ceiling" sentinel, not an invalid value.
    let cfg = load_with_cfg("max_operations = 0\n");
    assert_eq!(cfg.max_operations, None);
    let cfg = load_with_cfg("max_operations = 25000\n");
    assert_eq!(cfg.max_operations, Some(25_000));
}

#[test]
fn zero_and_garbage_budget_values_keep_defaults() {
    // The positive-integer keys reject 0 and non-numeric input, keeping defaults.
    let cfg = load_with_cfg("max_call_depth = 0\nmax_pattern_states = nope\n");
    assert_eq!(cfg.max_call_depth, 1_000);
    assert_eq!(cfg.max_pattern_states, 10_000);
}

// --- end-to-end enforcement ------------------------------------------------

const RECURSE_PROGRAM: &str = "\
define action called recurse with parameters n:
    check if n is greater than 0:
        return recurse of (n minus 1)
    end check
    return 0
end action
display recurse of 100000
";

#[test]
fn deep_recursion_is_a_clean_error_not_a_stack_overflow() {
    // With the default ceiling (1000) and the interpreter's large stack, runaway
    // recursion must surface as a catchable runtime error, never crash the
    // process with a native stack overflow.
    let output = run_with_cfg(None, RECURSE_PROGRAM);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Maximum call depth (1000)"),
        "expected the call-depth ceiling to fire; got:\n{combined}"
    );
    assert!(
        !combined.contains("stack overflow"),
        "recursion must not reach a native stack overflow; got:\n{combined}"
    );
}

#[test]
fn configured_call_depth_is_honored() {
    // A low ceiling fires early and reports the configured value.
    let output = run_with_cfg(Some("max_call_depth = 12\n"), RECURSE_PROGRAM);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Maximum call depth (12)"),
        "expected the configured ceiling of 12; got:\n{combined}"
    );
}

#[test]
fn oversized_source_is_refused() {
    // A generous program under a tiny source ceiling is refused before running.
    let program = format!("display \"{}\"\n", "x".repeat(200));
    let output = run_with_cfg(Some("max_source_size = 50\n"), &program);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Source file too large"),
        "expected the source-size ceiling to fire; got:\n{combined}"
    );
    assert!(
        !output.status.success(),
        "an oversized source must exit non-zero"
    );
}

#[test]
fn program_within_budget_still_runs() {
    // A shallow program under default limits runs normally (no false positives).
    let program = "\
define action called recurse with parameters n:
    check if n is greater than 0:
        return recurse of (n minus 1)
    end check
    return 42
end action
display recurse of 100
";
    let output = run_with_cfg(None, program);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("42"),
        "shallow recursion should complete; got:\n{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
