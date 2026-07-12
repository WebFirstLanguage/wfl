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

/// Render a path for embedding in a WFL string literal. WFL treats `\` as an
/// escape character, so Windows paths must use forward slashes (which the
/// runtime accepts on every platform).
fn wfl_path(p: &std::path::Path) -> String {
    p.display().to_string().replace('\\', "/")
}

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
    assert_eq!(cfg.max_pattern_steps, 5_000_000);
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
fn catching_a_recursion_limit_leaves_a_consistent_interpreter() {
    // A caught call-depth ResourceLimit must not corrupt the interpreter: the
    // enclosing `count` loop keeps running, its `count` variable stays readable,
    // and re-recursing after the catch stays bounded (no native stack overflow,
    // no depth under-count). Guards the dedicated call_depth counter and the
    // "don't mutate state in budget_error" contract.
    let program = "\
define action called deep with parameters n:
    return deep of (n plus 1)
end action

store caught as 0
count from 1 to 3:
    try:
        store dummy as deep of 0
    catch:
        change caught to caught plus 1
        display \"iteration \" with count
    end try
end count
display \"caught \" with caught with \" times\"
";
    let output = run_with_cfg(Some("max_call_depth = 20\n"), program);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("caught 3 times"),
        "the count loop must survive 3 caught recursion errors; got:\n{combined}"
    );
    // `count` stays readable inside the loop after a caught error.
    assert!(
        combined.contains("iteration 1") && combined.contains("iteration 3"),
        "the count variable must remain valid after a caught error; got:\n{combined}"
    );
    assert!(
        !combined.contains("stack overflow"),
        "catch-and-recurse must stay bounded; got:\n{combined}"
    );
    assert!(
        output.status.success(),
        "an all-caught program must exit zero; got:\n{combined}"
    );
}

const PATTERN_PROGRAM: &str = "\
create pattern digits:
    one or more digit
end pattern
check if \"123456789\" matches digits:
    display \"MATCHED\"
otherwise:
    display \"NO-MATCH\"
end check
";

#[test]
fn pattern_step_limit_is_enforced_and_propagated() {
    // A configured low pattern-step ceiling must surface as a catchable error at
    // the interpreter's `matches` operator — NOT be swallowed into a non-match.
    let output = run_with_cfg(Some("max_pattern_steps = 3\n"), PATTERN_PROGRAM);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("step limit"),
        "a low pattern-step ceiling must trip; got:\n{combined}"
    );
    assert!(
        !combined.contains("NO-MATCH"),
        "a budget breach must not be reported as a non-match; got:\n{combined}"
    );
}

#[test]
fn pattern_step_limit_is_catchable() {
    // The propagated pattern budget error is a ResourceLimit, catchable by a
    // general `try`/`when`.
    let program = "\
create pattern digits:
    one or more digit
end pattern
try:
    check if \"123456789\" matches digits:
        display \"MATCHED\"
    end check
catch:
    display \"CAUGHT\"
end try
";
    let output = run_with_cfg(Some("max_pattern_steps = 3\n"), program);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("CAUGHT"),
        "a pattern budget breach must be catchable; got:\n{combined}"
    );
}

#[test]
fn patterns_run_normally_under_default_budget() {
    // The raised per-instruction default must not trip on an ordinary match.
    let output = run_with_cfg(None, PATTERN_PROGRAM);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("MATCHED"),
        "an ordinary pattern must match under the default budget; got:\n{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
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

#[test]
fn nested_execute_file_source_is_size_checked() {
    // The source-size ceiling must cover nested sources, not only the top-level
    // file: a small main file that `execute file`s an oversized source is
    // refused when the nested file trips the cap.
    let dir = tempfile::tempdir().expect("create temp dir");
    // main.wfl fits under the cap; big.wfl does not.
    fs::write(dir.path().join(".wflcfg"), "max_source_size = 400\n").expect("cfg");
    let big = dir.path().join("big.wfl");
    fs::write(&big, format!("// {}\ndisplay \"hi\"\n", "x".repeat(500))).expect("big");
    let main = dir.path().join("program.wfl");
    fs::write(
        &main,
        format!(
            "execute file at \"{}\" and read output as out\n",
            wfl_path(&big)
        ),
    )
    .expect("main");

    let output = Command::new(test_helpers::get_wfl_binary_path())
        .arg(&main)
        .output()
        .expect("run wfl");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Source file too large"),
        "nested execute-file source must be size-checked; got:\n{combined}"
    );
}

#[test]
fn execute_file_shares_the_parent_operation_budget() {
    // The child interpreter created for `execute file` must share the parent's
    // budget, so work cannot be split across executed files to evade the
    // operation ceiling.
    //
    // Two runs pin the behavior without depending on an exact op count: a
    // ~50-iteration loop costs on the order of ~125 operations, so under a
    // 200-op ceiling the loop *alone* passes, but the loop plus an executed
    // child that runs the same loop (~250 ops total) must fail — which only
    // happens if the child shares the parent's budget rather than resetting it.
    let dir = tempfile::tempdir().expect("create temp dir");
    fs::write(dir.path().join(".wflcfg"), "max_operations = 200\n").expect("cfg");
    let loop_body =
        "store total as 0\ncount from 1 to 50:\n    change total to total plus 1\nend count\n";

    let child = dir.path().join("child.wfl");
    fs::write(&child, format!("{loop_body}display total\n")).expect("child");

    let run = |name: &str, body: String| -> String {
        let path = dir.path().join(name);
        fs::write(&path, body).expect("write program");
        let output = Command::new(test_helpers::get_wfl_binary_path())
            .arg(&path)
            .output()
            .expect("run wfl");
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    };

    // Anchor: the loop alone stays under the 200-op ceiling.
    let alone = run("alone.wfl", format!("{loop_body}display total\n"));
    assert!(
        !alone.contains("operation budget"),
        "the loop alone should stay under the ceiling; got:\n{alone}"
    );

    // The loop plus the executed child crosses the shared ceiling.
    let combined = run(
        "program.wfl",
        format!(
            "{loop_body}execute file at \"{}\" and read output as out\ndisplay out\n",
            wfl_path(&child)
        ),
    );
    assert!(
        combined.contains("operation budget"),
        "the operation ceiling must span parent + executed child; got:\n{combined}"
    );
}
