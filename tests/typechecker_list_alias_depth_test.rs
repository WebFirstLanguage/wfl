//! Regression coverage for issue #654 — the type checker must terminate on
//! programs whose list-alias relation is *cyclic*.
//!
//! `ListAliasPath` carries an `index_depth`. `add_structural_list_alias` and
//! `list_alias_members_for_path` both synthesize new paths at a *translated*
//! depth (`target.index_depth + descendant.index_depth - source.index_depth`).
//! When a binding transitively aliases itself at a different depth — a list
//! that contains itself, directly or through a map — each translation produces
//! a strictly deeper path, which is a brand-new alias-map key, which qualifies
//! as a "descendant" on the next pass. Nothing bounded the depth, so the alias
//! relation had no fixpoint: the checker spun at 100% CPU with memory climbing
//! and *never emitted a diagnostic*. A web-server program simply never reached
//! its `listen` statement.
//!
//! These tests pin the observable contract: such a program type-checks in
//! bounded time. They deliberately run the checker on a worker thread with a
//! hard deadline, so the pre-fix behaviour is a *test failure* rather than a
//! hung test process.

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

/// Generous relative to the ~20ms these programs need once the alias depth is
/// bounded, and far below the unbounded spin (observed >60s, still climbing).
/// The gap is wide enough that this is a termination check, not a benchmark.
const TYPECHECK_DEADLINE: Duration = Duration::from_secs(30);

/// Type-check `source` on a worker thread, failing the test if it does not
/// finish within [`TYPECHECK_DEADLINE`].
///
/// Returns the diagnostics the checker produced (empty when it accepted the
/// program). The point of these tests is *termination*, not acceptance: a
/// cyclic alias relation may legitimately produce diagnostics, but it must
/// never hang.
fn typecheck_within_deadline(label: &str, source: &'static str) -> Vec<String> {
    let (sender, receiver) = mpsc::channel();
    let started = Instant::now();

    // The worker is detached on timeout: the pre-fix checker never returns, so
    // joining it would hang the harness we are trying to protect.
    thread::Builder::new()
        .name(format!("typecheck-{label}"))
        // The alias walker recurses over the AST; give it room so a stack
        // overflow cannot masquerade as the hang under test.
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let tokens = lex_wfl_with_positions(source);
            let mut parser = Parser::new(&tokens);
            let program = parser.parse().expect("program should parse");
            let diagnostics = match TypeChecker::new().check_types(&program) {
                Ok(()) => Vec::new(),
                Err(failure) => failure
                    .into_diagnostics()
                    .into_iter()
                    .map(|diagnostic| diagnostic.message)
                    .collect(),
            };
            let _ = sender.send(diagnostics);
        })
        .expect("worker thread should spawn");

    match receiver.recv_timeout(TYPECHECK_DEADLINE) {
        Ok(diagnostics) => diagnostics,
        Err(mpsc::RecvTimeoutError::Timeout) => panic!(
            "type checking {label:?} did not terminate within {:?} (issue #654: unbounded \
             ListAliasPath::index_depth means the list-alias relation never reaches a fixpoint)",
            TYPECHECK_DEADLINE
        ),
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!(
            "type-check worker for {label:?} died after {:?}",
            started.elapsed()
        ),
    }
}

/// The shortest program that reproduces the hang: a list pushed into itself
/// twice. The first push records `scope@0 <-> scope@1`; from then on every
/// `list_alias_members_for_path(scope@0)` reports both depths, so the second
/// push translates the alias upward again — and the depth doubles per push.
#[test]
fn self_pushed_list_type_checks_in_bounded_time() {
    typecheck_within_deadline(
        "direct self push",
        r#"
store scope as [1]
push with scope and scope
push with scope and scope
display "done"
"#,
    );
}

/// A single self-push always terminated, even before the fix. Pinning it keeps
/// a future bound from being "fixed" by simply refusing to record the first
/// alias edge at all.
#[test]
fn single_self_push_still_type_checks() {
    typecheck_within_deadline(
        "single self push",
        r#"
store scope as [1]
push with scope and scope
display "done"
"#,
    );
}

/// The shape the issue reporter hit in `lib/scribe.wfl`: a scope list holds a
/// map whose value is a bindings list, and that bindings list holds the scope.
/// The cycle runs list -> map -> list, so no statement mentions a list pushed
/// into itself.
#[test]
fn list_map_list_alias_cycle_type_checks_in_bounded_time() {
    typecheck_within_deadline(
        "list/map/list cycle",
        r#"
store scope as [1]
store binds as [1]
create map nsval:
    "binds" is binds
end map
push with scope and nsval
push with binds and scope
display "done"
"#,
    );
}

/// Two lists pushed into each other. This terminated before the fix but took
/// seconds where it should take milliseconds; the deadline pins that it stays
/// bounded.
#[test]
fn mutually_pushed_lists_type_check_in_bounded_time() {
    typecheck_within_deadline(
        "mutual push",
        r#"
store outer as [1]
store inner as [1]
push with outer and inner
push with inner and outer
display "done"
"#,
    );
}

/// The alias relation is rebuilt on each re-check of a loop body, so a cyclic
/// push inside a loop compounds what a straight-line push does once.
#[test]
fn self_pushed_list_inside_a_loop_type_checks_in_bounded_time() {
    typecheck_within_deadline(
        "self push in loop",
        r#"
store scope as [1]
store items as [1 and 2 and 3]
for each item in items:
    push with scope and scope
    push with scope and item
end for
display "done"
"#,
    );
}

/// The cycle closed through an action boundary, which routes the alias through
/// the recorded action summary rather than the straight-line statement walker.
#[test]
fn alias_cycle_through_an_action_type_checks_in_bounded_time() {
    typecheck_within_deadline(
        "cycle through action",
        r#"
define action called nest with parameters target and payload:
    push with target and payload
end action

store scope as [1]
store binds as [1]
call nest with scope and binds
call nest with binds and scope
call nest with scope and binds
display "done"
"#,
    );
}

/// Deeply-but-finitely nested aggregates are the legitimate case the depth
/// bound must not break: nesting here is real structure, not a cycle.
#[test]
fn deeply_nested_acyclic_lists_still_type_check() {
    let diagnostics = typecheck_within_deadline(
        "deep acyclic nesting",
        r#"
store leaf as [1]
store level1 as [leaf]
store level2 as [level1]
store level3 as [level2]
store level4 as [level3]
push with level4[0][0][0] and "text"
display "done"
"#,
    );
    assert!(
        diagnostics.is_empty(),
        "deeply nested acyclic lists should type-check cleanly, got {diagnostics:?}"
    );
}
