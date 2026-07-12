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

## Notes / Follow-ups

- Stdlib pattern builtins (`pattern_replace`, `pattern_split`, and the
  `pattern_*` natives) run under a standalone budget with the same step/state
  ceilings, because native-function signatures have no interpreter handle. A
  future refactor could thread the run budget through them for deadline
  awareness; the ReDoS/state protection is already in place.
- The outbound `reqwest` client still has no per-request timeout or in-flight
  cap; that is a separate audit item (the budget's `max_pending_requests`
  covers the *inbound* accepted-request queue).
- The 1 GiB interpreter stack is a pragmatic fix for the interpreter's
  async-recursion stack cost; the deeper fix (trampolining the eval loop) is out
  of scope here.
