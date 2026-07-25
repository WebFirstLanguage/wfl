# Stream Terminal and Type-Flow Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate completed outbound-stream retention and make typechecking match runtime state across `try`, loop backedges, and deferred handler bodies.

**Architecture:** Clean EOF becomes a lightweight `StreamTerminal::CleanEof` entry in the existing bounded recent-terminal queue; the live slot, owner ID, body, cancel channel, and reaper are removed as soon as the final unterminated line is returned. The typechecker will use symbol-type snapshots as a small control-flow lattice: branch endpoints are joined before `finally`, loop headers are widened to a fixed point and checked once at that fixed point, and deferred event/WebSocket bodies are checked in isolated child scopes whose refinements are restored afterward.

**Tech Stack:** Rust 2024, Tokio, reqwest streaming, WFL analyzer/typechecker, Rust unit and integration tests.

## Global Constraints

- Risk class is **R3** because this changes streaming lifecycle and language backward-compatibility behavior.
- Every behavioral fix requires a test-only Red commit that is an ancestor of its Green commit.
- Existing WFL programs remain compatible; gradual `Unknown`/`Any` joins must remain permissive while concrete invalid branches remain diagnostics.
- Error aliases introduced by `when` clauses remain clause-local.
- Clean EOF retention uses the existing hard limits: at most **64** lightweight records for at most **60 seconds**, and the record is consumed by one follow-up read.
- No required test may be retried, skipped, quarantined, muted, or weakened.
- Required final gates are `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --jobs 1 -- -D warnings`, `cargo test --all --jobs 1`, a release build, Windows integration/web scripts, and forced docs example validation. Cargo jobs are bounded to one on this Windows host because the unbounded compiler process hit pagefile error 1455 before any test binary ran.

---

### Task 1: Test-only Red for clean EOF terminalization

**Files:**
- Modify: `src/interpreter/mod.rs` (unit-test module only)

**Interfaces:**
- Consumes: `IoClient::open_http_stream`, `IoClient::claim_stream_owner`, `IoClient::next_line`, `StreamRegistry::{live,recent}`, and `MAX_RECENT_STREAM_TERMINALS`.
- Produces: a regression proving live state is gone before the one-shot EOF read.

- [x] **Step 1: Strengthen the final-unterminated-line test**

After the first `Some("abc")`, inspect the registry and owner before any follow-up read:

```rust
let (live_slots, clean_eof_records) = {
    let registry = interpreter
        .io_client
        .stream_handles
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    (
        registry.live.len(),
        registry
            .recent
            .iter()
            .filter(|entry| entry.reason == StreamTerminal::CleanEof)
            .count(),
    )
};
assert_eq!(live_slots, 0);
assert_eq!(clean_eof_records, 1);
assert_eq!(
    interpreter
        .open_http_streams
        .borrow()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .len(),
    0
);
```

Then retain the existing assertions that one delayed read returns `None` and a later read reports an already-closed handle.

- [x] **Step 2: Add a bounded no-follow-up-read wave**

Open and claim more than `MAX_RECENT_STREAM_TERMINALS` `/unterminated` streams under one `Interpreter`, read only each final line, and assert:

```rust
assert_eq!(registry.live.len(), 0);
assert!(registry.recent.len() <= MAX_RECENT_STREAM_TERMINALS);
assert!(registry
    .recent
    .iter()
    .all(|entry| entry.reason == StreamTerminal::CleanEof));
assert!(interpreter
    .open_http_streams
    .borrow()
    .lock()
    .unwrap_or_else(|error| error.into_inner())
    .is_empty());
```

- [x] **Step 3: Run the focused tests and verify Red**

Run:

```powershell
cargo test --lib interpreter::outbound_stream_deadline_tests::final_unterminated_line_survives_deadline_after_clean_eof -- --nocapture --test-threads=1
cargo test --lib interpreter::outbound_stream_deadline_tests::unconsumed_clean_eof_records_are_bounded -- --nocapture --test-threads=1
```

Expected and observed: the lifecycle assertions fail because the completed
handle remains in `registry.live`; Red is a behavioral failure, not a
compile-failure placeholder.

- [x] **Step 4: Commit Red evidence**

```powershell
git add src/interpreter/mod.rs
git commit -m "test: expose retained clean eof stream state"
```

### Task 2: Bounded one-shot `CleanEof`

**Files:**
- Modify: `src/interpreter/mod.rs`

**Interfaces:**
- Consumes: `StreamRegistry::remember_recent` and `take_recent`.
- Produces: `StreamTerminal::CleanEof`; `take_stream` returns `Ok(None)` for that one-shot result.

- [x] **Step 1: Add the terminal reason and optional take result**

```rust
enum StreamTerminal {
    CleanEof,
    Timeout,
    Closed,
}

fn take_stream(
    &self,
    handle_id: &str,
) -> Result<Option<TakenStream>, HttpClientError>
```

When `take_recent` yields `CleanEof`, return `Ok(None)`; typed failures continue through `stream_terminal_error`.

- [x] **Step 2: Terminalize in `put_stream`**

Make the upstream `None` observation the linearization point:
`stream_pull` first-wins latches `CleanEof` on the shared cancel state before
returning control to `next_line`/`next_chunk`. In `put_stream`, handle the
latched `handle.done` case before generic cancellation or wall-deadline
rejection:

```rust
if handle.done {
    let terminal = cancel.terminate(StreamTerminal::CleanEof);
    if terminal == StreamTerminal::CleanEof {
        drop(handle);
        let now = Instant::now();
        let mut registry = self
            .stream_handles
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        registry.prune_recent(now);
        if let Some(mut slot) = registry.live.remove(handle_id) {
            slot.cancel.terminate(StreamTerminal::CleanEof);
            if let Some(abort) = slot.reaper_abort.take() {
                abort.abort();
            }
            drop(slot.handle.take());
            remove_stream_owner(&mut slot, handle_id);
        }
        registry.remember_recent(
            handle_id.to_string(),
            StreamTerminal::CleanEof,
            now,
        );
        return Ok(());
    }
}
```

`remember_recent` replaces any record for the same ID, so this restores exactly
one one-shot result even when close or the reaper already removed the live
slot. Do not clear either deadline and do not park a completed handle. If
`Timeout` or `Closed` won before upstream EOF was observed, preserve that
earlier typed terminal instead.

- [x] **Step 3: Consume clean EOF in both read APIs**

```rust
let Some(TakenStream { mut handle, cancel }) = self.take_stream(handle_id)? else {
    return Ok(None);
};
```

Use the same form in `next_chunk` and `next_line`. Lifecycle-only call sites that cannot legitimately observe clean EOF map it to closed without retaining heavy state.

- [x] **Step 4: Verify Green and adjacent lifecycle behavior**

Run:

```powershell
cargo test --lib interpreter::outbound_stream_deadline_tests -- --nocapture --test-threads=1
cargo test --test http_stream_test -- --nocapture --test-threads=1
```

Expected: all focused lifecycle and real-boundary streaming tests pass.

- [x] **Step 5: Commit Green**

```powershell
git add src/interpreter/mod.rs
git commit -m "fix: terminalize clean eof streams immediately"
```

### Task 3: Test-only Red for checker control-flow state

**Files:**
- Modify: `tests/typechecker_response_stream_join_test.rs`
- Modify: `tests/typechecker_response_stream_scope_test.rs`
- Create: `tests/typechecker_try_finally_join_test.rs`

**Interfaces:**
- Consumes: direct `Program`/`Statement` AST construction and the public `TypeChecker::check_types`.
- Produces: later-iteration, `finally`, alias-isolation, event-handler, and WebSocket-handler regressions.

- [x] **Step 1: Add later-iteration loop tests**

Construct `WhileLoop` and `RepeatWhileLoop` bodies in this order:

```rust
Statement::StreamWriteStatement {
    value: Expression::BinaryOperation {
        left: Box::new(Expression::Literal(Literal::Integer(10), 2, 1)),
        operator: Operator::Minus,
        right: Box::new(text_literal("not a number")),
        line: 2,
        column: 1,
    },
    target: Expression::Variable("out".to_string(), 2, 1),
    is_line: true,
    fallback_content: Some(Box::new(text_literal("valid file text"))),
    line: 2,
    column: 1,
},
stream_binding(),
```

Precede the loop with an `OpenFileStatement` binding `out`. Each test must expect `Cannot perform Minus operation`: the first iteration takes the valid File fallback, while the backedge can make the next iteration take the invalid ResponseStream reading.

- [x] **Step 2: Add `try` endpoint/finally tests**

Build a `TryStatement` whose handler binds `out` as a response stream and whose `finally` flushes `out`, with an outer concrete File binding. Assert the checker accepts the joined gradual state rather than resolving only the outer File.

Add a control where outer Number bindings reuse the handler error name and `error_message`; subtraction in `finally` must remain valid, proving both aliases stay clause-local.

- [x] **Step 3: Add deferred-handler isolation tests**

For both `EventHandler` and `WebSocketHandlerStatement`, start with outer `out: Number`, put `stream_binding()` in the registered body, and subtract one from outer `out` after registration. Assert typechecking succeeds.

- [x] **Step 4: Run focused tests and verify Red**

Run:

```powershell
cargo test --test typechecker_response_stream_join_test -- --nocapture
cargo test --test typechecker_try_finally_join_test -- --nocapture
cargo test --test typechecker_response_stream_scope_test -- --nocapture
```

Expected: the new later-iteration tests miss the invalid stream branch, the `finally` test rejects a File flush, and the event/WebSocket tests leak `ResponseStream` into outer `out`.

- [x] **Step 5: Commit Red evidence**

```powershell
git add tests/typechecker_response_stream_join_test.rs tests/typechecker_response_stream_scope_test.rs tests/typechecker_try_finally_join_test.rs
git commit -m "test: expose checker backedge and handler state gaps"
```

### Task 4: Checker joins, fixed points, and deferred scopes

**Files:**
- Modify: `src/analyzer/mod.rs`
- Modify: `src/typechecker/mod.rs`

**Interfaces:**
- Consumes: `Analyzer::{push_scope,pop_scope,snapshot_symbol_types,restore_symbol_types}` and `TypeChecker::join_type_snapshots`.
- Produces: `Analyzer::pop_scope_promoting_except` and `TypeChecker::check_loop_body_fixed_point`.

- [x] **Step 1: Add selective clause-scope promotion**

```rust
pub fn pop_scope_promoting_except(&mut self, excluded: &[String]) {
    if let Some(mut parent) = self.current_scope.parent.take() {
        for (name, symbol) in std::mem::take(&mut self.current_scope.symbols) {
            if !excluded.iter().any(|excluded_name| excluded_name == &name) {
                parent.define_or_replace(symbol);
            }
        }
        self.current_scope = *parent;
    }
}
```

This models the runtime’s shared try child while dropping only the temporary error aliases.

- [x] **Step 2: Join `try` endpoints before `finally`**

Within the shared try checker scope:

1. Snapshot entry.
2. Check the body and capture the success endpoint.
3. Form a conservative handler entry from entry plus body endpoint.
4. Restore that entry before every handler.
5. Check a handler in an alias child scope, promote all non-alias bindings, and capture its endpoint.
6. Restore handler entry before `otherwise` and capture its endpoint.
7. Join success, handler, otherwise, and possible unmatched-error endpoints.
8. Restore the join, then check `finally` once.

- [x] **Step 3: Compute a widening loop-header fixed point**

```rust
fn check_loop_body_fixed_point(&mut self, body: &[Statement]) {
    let entry = self.analyzer.snapshot_symbol_types();
    let mut header = entry.clone();
    loop {
        self.analyzer.restore_symbol_types(header.clone());
        let error_count = self.errors.len();
        for statement in body {
            self.check_statement_types(statement);
        }
        if self.budget_error.is_some() {
            return;
        }
        self.errors.truncate(error_count);
        let backedge = self.analyzer.snapshot_symbol_types();
        let next =
            Self::join_type_snapshots(&[entry.clone(), header.clone(), backedge]);
        if next == header {
            break;
        }
        header = next;
    }
    self.analyzer.restore_symbol_types(header.clone());
    for statement in body {
        self.check_statement_types(statement);
    }
    self.analyzer.restore_symbol_types(header);
}
```

Use it inside the persistent child scope for `RepeatWhileLoop` and in the current scope for `WhileLoop`. Validate each condition once under the stable header.

- [x] **Step 4: Isolate deferred callback bodies**

For `EventHandler` and `WebSocketHandlerStatement`, push a checker child scope, snapshot types, check the body, restore the snapshot, and pop. The WebSocket server operand remains checked outside the child.

- [x] **Step 5: Verify Green and affected checker suites**

Run:

```powershell
cargo test --test typechecker_response_stream_join_test -- --nocapture
cargo test --test typechecker_try_finally_join_test -- --nocapture
cargo test --test typechecker_response_stream_scope_test -- --nocapture
cargo test --test nothing_reassign_widen_test -- --nocapture
cargo test --test open_file_local_type_test -- --nocapture
```

Expected: all pass with no duplicated diagnostics.

- [x] **Step 6: Commit Green**

```powershell
git add src/analyzer/mod.rs src/typechecker/mod.rs
git commit -m "fix: stabilize checker control-flow state"
```

### Task 4.25: Linearize EOF and align analyzer `try` scopes

The first Green pass exposed two adjacent gaps during independent review.
They were repaired with another genuine Red-to-Green pair:

- [x] **Step 1: Commit deterministic Red regressions**

Commit `03966f06e78aec7c3bcdbd40feabc2bdff37a16d` adds a deterministic
EOF-observation-versus-later-timeout regression, analyzer-created
handler/`otherwise` binding regressions, and full-pipeline alias-shadowing
coverage.

- [x] **Step 2: Commit the Green implementation**

Commit `de34e32e513d7d73634b0d7308c681930953a4db` makes terminal signals
first-wins at upstream EOF and gives analyzer clauses/finally the same shared
runtime child environment while retaining clause-local error aliases.

### Task 4.5: Resolve independent-review evidence gaps

**Files:**
- Modify: `src/interpreter/mod.rs` (unit-test module only)
- Modify: `Docs/development/response-streaming-design.md`
- Modify: `Docs/development/testing-policy-exceptions/2026-07-24-pr-641-red-chronology.md`

- [x] **Step 1: Cover close/reaper missing-slot paths after observed EOF**

Add deterministic tests that latch `CleanEof`, remove the live slot through
the real close helper and a reaper-equivalent critical section, then prove
`put_stream` leaves exactly one one-shot `CleanEof`, no owner, no live slot,
one `nothing` read, and then the closed-handle error.

- [x] **Step 2: Document bounded clean-EOF retention**

Document that the one-shot result shares the 64-record recent-terminal queue,
expires after 60 seconds, and can be evicted or consumed.

- [x] **Step 3: Refresh the candidate chronology**

Record the exact executable candidate and all three new Red-to-Green chains
without treating independent review as maintainer approval.

- [x] **Step 4: Obtain follow-up review**

Have the independent reviewer verify the new deterministic tests and both
documentation repairs, and resolve any remaining Critical or Important issue.

### Task 5: Evidence, documentation, and full verification

**Files:**
- Create: `Dev diary/2026-07-25-stream-terminal-typeflow-review-fixes.md`
- Modify: `Docs/superpowers/plans/2026-07-25-stream-terminal-typeflow-fixes.md`
- Modify: `Docs/development/response-streaming-design.md`
- Modify: `Docs/development/testing-policy-exceptions/2026-07-24-pr-641-red-chronology.md`

**Interfaces:**
- Consumes: Red/Green commit IDs and exact command output.
- Produces: durable R3 acceptance-criteria mapping and residual-risk record.

- [x] **Step 1: Record change evidence**

The Dev Diary entry must include:

```markdown
- Risk class: R3
- Acceptance criteria -> exact test names
- Base, Red, and Green commit IDs for all three Red/Green pairs
- Focused, unit, integration, web, docs, format, and clippy commands
- Windows platform result and any explicitly non-applicable layers
- Rollback: revert the complete post-base repair range, including test-only
  commits, so intentionally failing Red tests are not left active; preserve
  the original Red/Green commits in history as evidence
- Residual risk: recent CleanEof records are intentionally capped at 64/60s
```

- [x] **Step 2: Run static and complete Rust gates**

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --jobs 1 -- -D warnings
cargo test --all --jobs 1
cargo build --release --jobs 1
```

Expected: every command exits zero without warnings from changed code.

- [x] **Step 3: Run real-boundary and documentation gates**

```powershell
$env:CARGO_BUILD_JOBS='1'
& 'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe' -NoProfile -ExecutionPolicy Bypass -File '.\scripts\run_integration_tests.ps1' -TestOnly
& 'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe' -NoProfile -ExecutionPolicy Bypass -File '.\scripts\run_web_tests.ps1'
python scripts/validate_docs_examples.py --ci --force
```

Expected: all required programs, web journeys, and docs examples pass without retry.

- [x] **Step 4: Obtain independent review**

Review the complete diff from `c9c748ce` through the final code commit for specification compliance, concurrency/lifecycle correctness, type-lattice soundness, compatibility, and test integrity. Resolve every Critical or Important finding and rerun its covering tests.

- [x] **Step 5: Commit evidence**

```powershell
git add "Dev diary/2026-07-25-stream-terminal-typeflow-review-fixes.md" Docs/development/response-streaming-design.md Docs/development/testing-policy-exceptions/2026-07-24-pr-641-red-chronology.md Docs/superpowers/plans/2026-07-25-stream-terminal-typeflow-fixes.md
git commit -m "docs: record stream state review fixes"
```

## Self-Review

- Spec coverage: all four review findings map to Tasks 1–4; R3 evidence and full gates map to Task 5.
- Placeholder scan: no TBD/TODO/later placeholders remain.
- Type consistency: `CleanEof`, `take_stream -> Result<Option<TakenStream>, _>`, `pop_scope_promoting_except`, and `check_loop_body_fixed_point` are named consistently in every task.
- Execution choice: this request already asks for the fixes in the current branch, so execute inline in this session without pausing between tasks.
