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

#[test]
fn execute_file_shares_the_parent_recursion_depth() {
    // Recursion accounting must span the `execute file` boundary: a child cannot
    // get a fresh full call-depth allowance, or nested execute files would
    // multiply the native stack and overflow before the guard fires.
    //
    // With `max_call_depth = 20`, a parent recursed ~13 deep that then executes a
    // child recursing ~13 deep exceeds the shared ceiling (only if the child
    // inherits the parent's live depth); the child's own recursion (13 < 20)
    // would otherwise pass on a reset allowance.
    let dir = tempfile::tempdir().expect("create temp dir");
    fs::write(dir.path().join(".wflcfg"), "max_call_depth = 20\n").expect("cfg");

    let child = dir.path().join("child.wfl");
    fs::write(
        &child,
        "define action called c with parameters n:\n\
         \x20   check if n is greater than 0:\n\
         \x20       return c of (n minus 1)\n\
         \x20   end check\n\
         \x20   return 0\n\
         end action\n\
         display c of 12\n\
         display \"child-done\"\n",
    )
    .expect("child");

    let parent = dir.path().join("program.wfl");
    fs::write(
        &parent,
        format!(
            "define action called p with parameters n:\n\
             \x20   check if n is greater than 0:\n\
             \x20       return p of (n minus 1)\n\
             \x20   end check\n\
             \x20   execute file at \"{}\" and read output as out\n\
             \x20   display out\n\
             \x20   return 0\n\
             end action\n\
             display p of 12\n",
            wfl_path(&child)
        ),
    )
    .expect("parent");

    let output = Command::new(test_helpers::get_wfl_binary_path())
        .arg(&parent)
        .output()
        .expect("run wfl");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Maximum call depth (20)"),
        "the recursion ceiling must span parent + executed child; got:\n{combined}"
    );
    assert!(
        !combined.contains("child-done"),
        "the child must not get a fresh depth allowance; got:\n{combined}"
    );
}

#[test]
fn analyze_mode_consults_the_budget_in_the_front_end() {
    // `--analyze` never interprets, so an operation-budget breach it surfaces can
    // only come from the front end (lex/parse/analyze) actually consulting the
    // shared budget — proving the phases poll it, not merely measure elapsed time
    // for later interpretation.
    let program = "display 1\n".repeat(40);
    let dir = tempfile::tempdir().expect("create temp dir");
    fs::write(dir.path().join(".wflcfg"), "max_operations = 3\n").expect("cfg");
    let script = dir.path().join("program.wfl");
    fs::write(&script, &program).expect("program");

    let output = Command::new(test_helpers::get_wfl_binary_path())
        .arg("--analyze")
        .arg(&script)
        .output()
        .expect("run wfl --analyze");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("operation budget"),
        "--analyze must consult the shared budget during the front end; got:\n{combined}"
    );
}

/// Build a program whose single top-level statement (a `count` loop) holds many
/// nested statements, and parse it. Parsing happens BEFORE any budget is
/// installed so the parser's own checkpoint cannot consume the operation cap —
/// a later breach must therefore come from the phase under test recursing into
/// the nested body.
fn parse_deeply_nested_program() -> wfl::parser::ast::Program {
    let mut body = String::from("count from 1 to 3:\n");
    for _ in 0..64 {
        body.push_str("    display 1\n");
    }
    body.push_str("end count\n");

    let tokens = wfl::lexer::lex_wfl_with_positions(&body);
    wfl::parser::Parser::new(&tokens)
        .parse()
        .expect("nested program parses")
}

/// Install a run budget with a small operation cap for the lifetime of the
/// returned guard.
fn enter_capped_budget(max_operations: u64) -> wfl::exec::budget::CurrentBudgetGuard {
    let limits = wfl::exec::budget::BudgetLimits {
        max_operations: Some(max_operations),
        ..Default::default()
    };
    let budget = std::sync::Arc::new(wfl::exec::budget::ExecutionBudget::new(limits));
    wfl::exec::budget::ExecutionBudget::enter(budget)
}

#[test]
fn analyzer_polls_the_budget_inside_nested_bodies() {
    // The entry checkpoint in `analyze` fires once, before traversal. Only the
    // recursive checkpoint in `analyze_statement` can trip the operation cap
    // from inside the loop body, so tripping it here proves nested analysis
    // honors the budget rather than only the phase boundary.
    let program = parse_deeply_nested_program();
    let _guard = enter_capped_budget(5);

    let result = wfl::analyzer::Analyzer::new().analyze(&program);
    let errors = result.expect_err("nested analysis must trip the operation cap");
    assert!(
        format!("{errors:?}").contains("operation budget"),
        "analyzer must surface the operation-budget breach from a nested body; got: {errors:?}"
    );
}

#[test]
fn type_checker_polls_the_budget_inside_nested_bodies() {
    // `with_analyzer` skips the analyzer pass, so the breach can only come from
    // the recursive checkpoint in `check_statement_types` — not the analyzer and
    // not a top-level-only poll. The fatal `TypeCheckError::Budget` variant
    // proves the nested type-check surfaced the breach on the fatal channel.
    let program = parse_deeply_nested_program();

    let mut analyzer = wfl::analyzer::Analyzer::new();
    wfl::stdlib::typechecker::register_stdlib_types(&mut analyzer);

    let _guard = enter_capped_budget(5);

    let mut type_checker = wfl::typechecker::TypeChecker::with_analyzer(analyzer);
    let outcome = type_checker.check_types(&program);
    assert!(
        matches!(outcome, Err(wfl::typechecker::TypeCheckError::Budget(_))),
        "nested type checking must surface the operation-budget breach as the fatal TypeCheckError::Budget variant; got: {outcome:?}"
    );
}

#[tokio::test]
async fn task_local_budget_is_isolated_across_interleaved_runs() {
    // The regression that motivated task-local scoping: two runs with DISTINCT
    // budgets interleaved on ONE thread (a library embedder `join!`ing two
    // `!Send` interpreter futures) must each keep seeing their OWN budget across
    // an `.await`. A thread-local held across the await would let the second run
    // overwrite the first's current budget and corrupt it. `#[tokio::test]`
    // defaults to a current-thread runtime, so `join!` genuinely interleaves the
    // two futures at the `yield_now` points on a single thread.
    use std::sync::Arc;
    use wfl::exec::budget::{BudgetLimits, ExecutionBudget};

    fn budget_with_ops(n: u64) -> Arc<ExecutionBudget> {
        Arc::new(ExecutionBudget::new(BudgetLimits {
            max_operations: Some(n),
            ..Default::default()
        }))
    }

    async fn observe_across_await() -> (Option<u64>, Option<u64>) {
        let first = ExecutionBudget::current().and_then(|b| b.limits().max_operations);
        tokio::task::yield_now().await; // hand control to the sibling run
        let second = ExecutionBudget::current().and_then(|b| b.limits().max_operations);
        (first, second)
    }

    let run_a = ExecutionBudget::scope(budget_with_ops(11), observe_across_await());
    let run_b = ExecutionBudget::scope(budget_with_ops(22), observe_across_await());
    let (a, b) = tokio::join!(run_a, run_b);

    assert_eq!(
        a,
        (Some(11), Some(11)),
        "run A must see only its own budget"
    );
    assert_eq!(
        b,
        (Some(22), Some(22)),
        "run B must see only its own budget"
    );
}

#[test]
fn run_with_interpreter_stack_provides_a_large_stack() {
    // The public helper embedders use to get the CLI's stack safety must run its
    // work on a genuinely large stack: recursion deep enough to overflow the
    // default (2 MiB) test-thread stack completes when driven through the helper.
    fn deep(n: u64) -> u64 {
        // ~4 KiB per frame, so ~20k frames need ~80 MiB — far past 2 MiB, far
        // under the reserved 1 GiB. `black_box` blocks tail-call/dead-code
        // optimization so these are real stack frames.
        let filler = [0u8; 4096];
        std::hint::black_box(&filler);
        if n == 0 {
            0
        } else {
            deep(n - 1).wrapping_add(u64::from(filler[0]))
        }
    }

    let result =
        wfl::run_with_interpreter_stack(|| deep(20_000)).expect("large stack should spawn");
    assert_eq!(result, 0);
}

#[test]
fn parser_polls_the_budget_inside_one_huge_expression() {
    // A single statement whose expression is a giant list literal. The
    // statement-boundary checkpoint fires only once (there is one statement), so
    // only the strided per-operand checkpoint in expression parsing can trip the
    // budget — proving a huge single expression is now interruptible, not parsed
    // to completion after the deadline.
    let mut src = String::from("store big as [");
    for i in 0..6000 {
        if i > 0 {
            src.push_str(", ");
        }
        src.push('1');
    }
    src.push_str("]\n");

    // Lex with no budget installed so the parser (not the lexer) is what trips.
    let tokens = wfl::lexer::lex_wfl_with_positions(&src);
    let _guard = enter_capped_budget(1);
    let result = wfl::parser::Parser::new(&tokens).parse();
    assert!(
        result.is_err(),
        "a huge single expression must trip the strided parser budget checkpoint"
    );
}

#[test]
fn analyzer_phase_budget_breach_is_fatal_and_typed() {
    // `TypeChecker::new()` runs the analyzer internally before type checking. A
    // breach during that analysis phase must surface as the fatal
    // `TypeCheckError::Budget` variant — not be silently downgraded to ordinary
    // `TypeCheckError::Types` diagnostics. This guards the gap the maintainer
    // flagged: the analyzer's breach used to be rendered as `TypeError`s while
    // the fatal budget channel stayed empty, so a caller checking it saw nothing.
    let program = parse_deeply_nested_program();
    // cap = 1: the analyzer trips on its first statement (before the type-check
    // loop is even reached), so the breach can only travel out through
    // `check_types`'s analyzer-propagation path.
    let _guard = enter_capped_budget(1);
    let mut type_checker = wfl::typechecker::TypeChecker::new();
    let outcome = type_checker.check_types(&program);
    assert!(
        matches!(outcome, Err(wfl::typechecker::TypeCheckError::Budget(_))),
        "an analysis-phase budget breach must surface as the fatal TypeCheckError::Budget variant; got: {outcome:?}"
    );
}

#[test]
fn analyzer_entry_budget_breach_is_fatal_and_typed() {
    // Companion to the test above, targeting the analysis PHASE BOUNDARY: a
    // budget already cancelled/exhausted when `analyze` is entered trips its
    // entry checkpoint *before* any statement is visited. That branch must still
    // record the typed breach on `budget_error`, so `TypeChecker::new()`'s
    // `take_budget_error()` reclassifies it as the fatal `Budget` variant rather
    // than misreading the rendered `SemanticError` as ordinary type errors. A
    // cancelled budget guarantees the entry checkpoint (not `analyze_statement`)
    // is what fires.
    use wfl::exec::budget::{BudgetLimits, ExecutionBudget};
    let program = parse_deeply_nested_program();
    let budget = std::sync::Arc::new(ExecutionBudget::new(BudgetLimits::default()));
    budget.cancel();
    let _guard = ExecutionBudget::enter(budget);
    let mut type_checker = wfl::typechecker::TypeChecker::new();
    let outcome = type_checker.check_types(&program);
    assert!(
        matches!(outcome, Err(wfl::typechecker::TypeCheckError::Budget(_))),
        "an entry-time budget breach must surface as the fatal TypeCheckError::Budget variant; got: {outcome:?}"
    );
}

// --- lexer: a typed fatal outcome, never a truncated success (P1-1) ---------

/// A source with well over `LEX_CHECKPOINT_STRIDE` (4096) raw tokens, so the
/// lexer's strided budget checkpoint fires several times. Each `display 1` line
/// is three raw tokens (`display`, `1`, newline), so 4000 lines ≈ 12k tokens →
/// checkpoints near 4096, 8192, and 12288.
fn big_lexer_source() -> String {
    "display 1\n".repeat(4000)
}

#[test]
fn checked_lexer_reports_cancellation_not_a_partial_stream() {
    use wfl::exec::budget::{BudgetExceeded, BudgetLimits, ExecutionBudget};
    let budget = std::sync::Arc::new(ExecutionBudget::new(BudgetLimits::default()));
    budget.cancel();
    let _guard = ExecutionBudget::enter(std::sync::Arc::clone(&budget));
    let result = wfl::lexer::lex_wfl_with_positions_checked(&big_lexer_source());
    assert_eq!(
        result.err(),
        Some(BudgetExceeded::Cancelled),
        "a cancelled run must abort lexing with a typed Cancelled, not a truncated stream"
    );
}

#[test]
fn checked_lexer_reports_deadline() {
    use wfl::exec::budget::{BudgetExceeded, BudgetLimits, ExecutionBudget};
    let limits = BudgetLimits {
        max_duration: Some(std::time::Duration::from_secs(0)),
        ..Default::default()
    };
    let budget = std::sync::Arc::new(ExecutionBudget::new(limits));
    let _guard = ExecutionBudget::enter(budget);
    let result = wfl::lexer::lex_wfl_with_positions_checked(&big_lexer_source());
    assert!(
        matches!(result, Err(BudgetExceeded::Deadline { .. })),
        "an elapsed deadline must abort lexing with a typed Deadline; got: {result:?}"
    );
}

#[test]
fn checked_lexer_reports_operation_exhaustion() {
    use wfl::exec::budget::{BudgetExceeded, BudgetLimits, ExecutionBudget};
    // One operation is allowed; the second strided checkpoint (~token 8192)
    // charges index 1 >= 1 and trips. `big_lexer_source` spans past two strides.
    let limits = BudgetLimits {
        max_operations: Some(1),
        max_duration: None,
        ..Default::default()
    };
    let budget = std::sync::Arc::new(ExecutionBudget::new(limits));
    let _guard = ExecutionBudget::enter(budget);
    let result = wfl::lexer::lex_wfl_with_positions_checked(&big_lexer_source());
    assert!(
        matches!(result, Err(BudgetExceeded::Operations { .. })),
        "an exhausted operation budget must abort lexing with a typed Operations; got: {result:?}"
    );
}

#[test]
fn checked_lexer_returns_the_complete_stream_within_budget() {
    // A generous budget lexes the whole source: the checked variant returns the
    // FULL token stream (same length as the unbudgeted lexer), never a prefix.
    use wfl::exec::budget::{BudgetLimits, ExecutionBudget};
    let src = big_lexer_source();
    let full = wfl::lexer::lex_wfl_with_positions(&src).len();
    let budget = std::sync::Arc::new(ExecutionBudget::new(BudgetLimits::default()));
    let _guard = ExecutionBudget::enter(budget);
    let checked = wfl::lexer::lex_wfl_with_positions_checked(&src).expect("within budget");
    assert_eq!(
        checked.len(),
        full,
        "a within-budget checked lex must return the complete stream"
    );
}

#[test]
fn plain_lexer_never_truncates_even_under_a_breached_budget() {
    // The plain (non-budgeted) lexer must NEVER return a truncated stream — the
    // silent-truncation footgun is gone. Even with a cancelled budget installed,
    // it tokenizes the whole source (its checkpoint is a no-op), so a caller that
    // deliberately uses it (the LSP, tooling, tests) is unaffected by run state.
    use wfl::exec::budget::{BudgetLimits, ExecutionBudget};
    let src = big_lexer_source();
    let without = wfl::lexer::lex_wfl_with_positions(&src).len();
    let budget = std::sync::Arc::new(ExecutionBudget::new(BudgetLimits::default()));
    budget.cancel();
    let _guard = ExecutionBudget::enter(budget);
    let with = wfl::lexer::lex_wfl_with_positions(&src).len();
    assert_eq!(
        with, without,
        "the plain lexer must not truncate under any budget state"
    );
}

#[test]
fn cli_lex_dump_fails_on_a_budget_breach_instead_of_a_partial_dump() {
    // `wfl --lex` must NOT write a partial token dump and exit 0 when the run
    // budget is breached during lexing. With `max_operations = 1` and a source
    // past two lexer strides, tokenization trips and the CLI exits non-zero with
    // the breach message, writing no `.lex.txt`.
    let dir = tempfile::tempdir().expect("temp dir");
    fs::write(dir.path().join(".wflcfg"), "max_operations = 1\n").expect("cfg");
    let script = dir.path().join("big.wfl");
    fs::write(&script, big_lexer_source()).expect("program");
    let output = Command::new(test_helpers::get_wfl_binary_path())
        .arg("--lex")
        .arg(&script)
        .output()
        .expect("run wfl --lex");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "--lex must fail on a budget breach; got:\n{combined}"
    );
    assert!(
        combined.contains("operation budget"),
        "--lex must report the budget breach; got:\n{combined}"
    );
    assert!(
        !dir.path().join("big.wfl.lex.txt").exists(),
        "--lex must not write a partial token dump on a breach"
    );
}

// --- default public interpreter path is stack-safe (P2-5) -------------------

#[test]
fn default_public_interpreter_path_is_stack_safe_on_an_ordinary_stack() {
    // An embedder using the DEFAULT public path — `Interpreter::new()` — on an
    // ordinary 8 MiB thread stack must get a catchable call-depth resource error
    // from runaway recursion, NOT a native stack overflow. `new()` caps recursion
    // at the conservative `DEFAULT_EMBED_CALL_DEPTH`, so the guard fires well
    // before the ~40-frame debug overflow point on such a stack. (The CLI reaches
    // the full 1000 only on its dedicated 1 GiB stack.) A regression that let the
    // default path use depth 1000 here would overflow this 8 MiB stack and abort.
    // The interpreter's `Value`/`RuntimeError` are `!Send`, so reduce the outcome
    // to a `Send` summary INSIDE the thread and return only that.
    let handle = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| -> Result<(), String> {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(async {
                // No base case: recurses until the depth guard fires.
                let program = "\
define action called deep with parameters n:
    return deep of (n plus 1)
end action
display deep of 0
";
                let tokens = wfl::lexer::lex_wfl_with_positions(program);
                let ast = wfl::parser::Parser::new(&tokens).parse().expect("parses");
                match wfl::Interpreter::new().interpret(&ast).await {
                    Ok(_) => Err("runaway recursion unexpectedly succeeded".to_string()),
                    Err(errors)
                        if errors
                            .iter()
                            .any(|e| e.message.contains("Maximum call depth")) =>
                    {
                        Ok(())
                    }
                    Err(errors) => Err(format!(
                        "unexpected error (not a call-depth breach): {errors:?}"
                    )),
                }
            })
        })
        .expect("spawn ordinary-stack thread");
    let summary = handle
        .join()
        .expect("the default path must not abort with a native stack overflow");
    summary.expect("the default public path must return a call-depth resource error");
}
