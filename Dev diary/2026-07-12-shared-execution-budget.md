# Dev Diary — 2026-07-12: Shared ExecutionBudget

## Context

WFL enforced a dozen unrelated resource ceilings from a dozen unrelated places,
and several audit-flagged resources had **no** ceiling at all:

| Resource | Before |
|---|---|
| Wall-clock timeout | `Interpreter.max_duration` + `op_count` throttle (`check_time`) |
| Interpreter operations | none (the op counter only throttled clock reads) |
| Recursion depth | none in release (a `debug_assert!(< 10_000)` only) |
| Import / include depth | circular-detection only; otherwise unbounded |
| `execute file` depth | a lone `const MAX_EXECUTE_FILE_DEPTH = 4` |
| Pattern transitions | a lone `const MAX_STEPS = 100_000` in the VM |
| Pattern active states | none — the NFA state set could fan out unbounded |
| Source-file size | none |
| HTTP request body | `web_server_max_body_size` |
| HTTP response body | none |
| Pending HTTP requests | `web_server_request_queue_bound` |
| WebSocket queues | none — `unbounded_channel` (flagged as a Phase 0 follow-up) |
| WebSocket connections | none — the registry grew without bound |

This entry replaces that scatter with **one** object,
`exec::budget::ExecutionBudget`, that travels with a run through parsing,
evaluation, pattern matching, web handling, and module loading, and owns every
ceiling above.

## What changed

### New module `src/exec/budget.rs`

- `BudgetLimits` — the immutable per-run ceilings. `BudgetLimits::from_config`
  maps the existing `.wflcfg` keys (`timeout_seconds`,
  `web_server_max_body_size`, `web_server_request_queue_bound`) plus the new
  budget keys onto its fields, so nothing about the old knobs changed.
- `ExecutionBudget` — the shared object. **`Send + Sync`**: every mutable field
  is an atomic (`AtomicU64`/`AtomicUsize`/`AtomicBool`), so an
  `Arc<ExecutionBudget>` clones into the multi-threaded web transport (warp
  accept tasks, per-connection WebSocket tasks) without any `Rc`/`RefCell`
  crossing a thread boundary. This does **not** violate the "no `Rc`→`Arc`
  rewrite of the interpreter core" rule — the budget is a separate atomic-only
  object, shared exactly like `Arc<WflConfig>` already is.
- `BudgetExceeded` — a typed breach that each subsystem maps onto its own error
  (`RuntimeError` in the interpreter, `PatternError` in the VM). The deadline
  variant keeps the historic `"Execution exceeded timeout (Ns)"` wording and
  `ErrorKind::Timeout` so existing timeout handling/tests still match; a new
  `ErrorKind::ResourceLimit` covers the rest.
- RAII guards (`RequestGuard`, `WsConnectionGuard`) release their atomic slot on
  drop.

### Interpreter (`src/interpreter/mod.rs`)

- Removed the `op_count`/`started`/`max_duration` fields; added
  `budget: Arc<ExecutionBudget>`. `check_time()` now calls
  `budget.charge_operation(enforce_limits)` where `enforce_limits = !in_main_loop`
  — preserving the rule that a `main loop` is exempt from the timeout, while
  cooperative cancellation still applies. Clock reads stay throttled to one per
  1024 operations inside the budget.
- Recursion ceiling: `call_function` checks `budget.check_call_depth(stack_len)`
  before pushing a frame.
- Import ceiling: both `load module` and `include from` check
  `budget.check_import_depth(loading_stack_len)` after the circular check.
- `execute file` depth: `MAX_EXECUTE_FILE_DEPTH` const deleted; the guard is now
  `budget.check_execute_file_depth(execute_depth)` (default still 4).
- Response ceiling: `respond` checks `budget.check_response_bytes(len)` before
  handing the body to the transport.
- Request body + pending-request bounds now read from the budget
  (`max_request_body_bytes`, `max_pending_requests`); values are identical to the
  old config reads.
- WebSocket bounding (the deferred Phase 0 follow-up): the per-connection
  outbound channel and the per-server event channel became **bounded**
  `mpsc::channel(ws_queue_bound)` with `try_send` + shed-on-`Full` logging; a new
  connection acquires a `WsConnectionGuard` and is refused (close frame + log)
  past `max_ws_connections`.

### Pattern VM (`src/pattern/vm.rs`, `src/pattern/mod.rs`)

- `PatternVM` holds an `Arc<ExecutionBudget>`. `MAX_STEPS` deleted; each match
  attempt checks `step_count` against `budget.pattern_step_limit()` and — new —
  the active-state set against `budget.pattern_state_limit()` (guards exponential
  fan-out). Added `PatternError::StateLimitExceeded` and `Cancelled`.
- `PatternVM::new()` uses a standalone budget with the historic ReDoS defaults
  (steps 100_000), so stdlib pattern builtins keep their guard. New
  `CompiledPattern::{matches,find,find_all}_with_budget` let the interpreter's
  `matches`/`find` operators share the run budget (deadline/cancellation-aware);
  nested lookaround VMs share the parent's budget.

### Source size + a real recursion guard (`src/main.rs`)

- After loading config, the CLI refuses an over-`max_source_size` file before
  lexing.
- **Large-stack thread.** WFL's async tree-walking interpreter costs on the
  order of a *megabyte* of debug stack per WFL call (an 8 MiB stack overflows
  near depth ~40). A `max_call_depth` guard is meaningless if the OS stack
  overflows first, so `main` now runs the whole runtime on a dedicated thread
  with a **1 GiB** stack (reserved virtually, committed lazily). With it, the
  default ceiling of 1000 fires as a clean error well before the native limit in
  both debug (~1460-frame floor) and release. Empirically: depth 999 completes,
  depth ≥ 1000 returns *"Maximum call depth (1000) exceeded"* instead of
  `SIGABRT`.

### Config (`src/config.rs`)

- Nine new keys with positive-integer validation (`max_operations` accepts `0` =
  unlimited): `max_operations`, `max_call_depth`, `max_import_depth`,
  `max_execute_file_depth`, `max_pattern_steps`, `max_pattern_states`,
  `max_source_size`, `web_server_max_response_size`, `web_socket_queue_bound`,
  `web_socket_max_connections`. A shared `set_positive_usize` helper keeps the
  parse arms terse.

## Backward compatibility

- The three pre-existing knobs (timeout, request body, request queue) keep their
  defaults and behavior exactly; the budget just sources their values.
- The two potentially-breaking dimensions default to no new failures:
  `max_operations` is off by default, and every new ceiling (recursion, import,
  states, source/response bytes, WS) defaults generously enough that no existing
  program or `TestPrograms/` case trips it — the recursion default (1000) is far
  above any depth a program completed at under the old 8 MiB stack, and, thanks
  to the large stack, programs can now recurse *deeper* than before while
  runaway recursion becomes a clean error instead of a crash.

## Tests

- `src/exec/budget.rs`: 11 unit tests — operation ceiling, main-loop exemption,
  cancellation-always-honored, deadline, `>=` depth vs `>` pattern semantics,
  byte ceilings, RAII request/WS-connection guards, `from_config` mapping,
  `Send + Sync` assertion.
- `tests/execution_budget_test.rs`: config parsing of all new keys (defaults,
  overrides, zero/garbage rejection, `max_operations = 0`), and end-to-end —
  deep recursion is a clean *"Maximum call depth (1000)"* error and **not** a
  stack overflow; a configured low ceiling is honored; an oversized source is
  refused; a shallow program still runs.
- Existing suites unchanged: full lib tests (incl. `test_timeout_forever_loop`),
  `web_queue_bound_test`, `websocket_test`, `execute_file_test`, and the
  `TestPrograms/` integration run all pass.

## Review follow-ups (automated PR review)

Addressed in the same PR after the first round of automated review:

- **Budget spans `execute file`.** The nested interpreter now clones the
  parent's `Arc<ExecutionBudget>` instead of building a fresh one, so the
  deadline, operation ceiling, and cancellation cover the whole run — work can't
  be split across executed files to evade them. Regression test:
  `execute_file_shares_the_parent_operation_budget`.
- **Nested source sizes are checked.** `load module`, `include from`, and
  `execute file` enforce `max_source_size` via file metadata *before* reading,
  matching the top-level guarantee (which now also checks metadata pre-read).
  Regression test: `nested_execute_file_source_is_size_checked`.
- **Catchable errors keep the call stack.** `budget_error` only force-clears the
  call stack for the terminal deadline; a `ResourceLimit` (e.g. the recursion
  ceiling) is catchable by `try`/`when`, so the stack is left for
  `call_function` to unwind frame-by-frame — otherwise the depth counter
  (`call_stack.len()`) would under-count after a caught recursion error.
- **Pattern state cap fails fast.** The active-state ceiling is now checked after
  each expansion inside the step loop, not only once the whole next generation is
  built, so a runaway step can't allocate far past the cap.
- Doc/comment accuracy fixes: `PatternVM::new` no longer references the removed
  `MAX_STEPS`; the VM error-mapping comment no longer implies it samples the
  deadline; the CLI comment says "same config", not "same budget instance"; and
  the duplicate "Execution budget (resource limits)" doc heading was renamed so
  the anchor stays unique.

## Deep review round (maintainer P1/P2 + bot reviewers)

A subsequent maintainer review raised five blocking findings and several P2s;
all were addressed on this PR:

- **P1-1 — recursion guard robust under catch.** Enforcement depth moved to a
  dedicated RAII `call_depth` counter, decoupled from the diagnostic `call_stack`.
  `budget_error` no longer mutates *any* interpreter state (every breach is
  catchable), and `interpret()` resets per-run loop/depth state, so
  catch-and-recurse stays bounded and an enclosing `count` loop survives a caught
  recursion error. Test: `catching_a_recursion_limit_leaves_a_consistent_interpreter`.
- **P1-2 — pattern transitions counted for real.** Charged per *instruction* in
  `step()` (bounds epsilon-jump cycles) on one shared meter that nested
  lookaround/lookbehind VMs charge into (no reset); breaches now *propagate* as
  catchable `ResourceLimit` errors through the interpreter operators AND the
  stdlib builtins (`pattern_matches/find/find_all/split`), which reach the run
  budget via a thread-local. `max_pattern_steps` default raised to 5_000_000 for
  per-instruction granularity.
- **P1-3 — HTTP OOM/leak paths closed.** Request body enforced *while streaming*
  (chunked-safe → 413); one **global** in-flight `RequestGuard` (shared across
  listeners) held from body-read until response / disconnect / 504 timeout
  (`web_server_response_timeout_seconds`, default 300); response cap checks the
  borrowed length before duplicating.
- **P1-4 — one budget per run.** `main.rs` builds a single budget up front and
  threads it through the source check and the interpreter
  (`with_config_and_budget` + `budget()` handle); `execute file` shares it.
- **P1-5 — bounded source loader everywhere.** A bounded reader (≤ `max+1` bytes)
  covers the CLI, `load module`, `include`, `execute file`, and the REPL, so
  oversized sources are refused without allocation even when metadata lies.
- **P2** — `ConfigChecker` now knows every budget key (so `--configFix` can't
  strip them); WebSocket `close server` sends a guaranteed close frame even under
  backpressure and logs full/closed outbound queues; the large stack is deferred
  for `--help`/`--version` and falls back on reservation failure; the REPL uses a
  no-deadline budget; MSRV recorded as 1.88 (`let`-chains).

## Notes / Follow-ups

- The outbound `reqwest` client still has no per-request timeout or in-flight
  cap; that is a separate audit item (the budget's `max_pending_requests`
  covers the *inbound* accepted-request queue).
- The 1 GiB interpreter stack is a pragmatic fix for the interpreter's
  async-recursion stack cost; the deeper fix (trampolining the eval loop) is out
  of scope here.
