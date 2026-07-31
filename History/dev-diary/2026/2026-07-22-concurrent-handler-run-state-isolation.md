# Dev Diary — 2026-07-22 — Per-handler run-state isolation for `main loop concurrently:`

## Context

PR #641 shipped `main loop concurrently:` — opt-in cooperative concurrency for
HTTP request handlers, driven by a `FuturesUnordered` on the single interpreter
thread. Review (maintainer P1 #1, echoed by Copilot) flagged a soundness gap:
the concurrent loop isolated each handler's **environment** (variables), but the
interpreter's **run-state** — the count-loop variable and its flag
(`current_count` / `in_count_loop`), the live recursion depth (`call_depth`),
the diagnostic call stack (`call_stack`), and the current block's overload-dup
set — still lived on the shared `Interpreter` behind `RefCell`/`Cell`.

Under serial execution that state is never contended. Under
`main loop concurrently:` several handler futures interleave on one thread, so at
every `await` one handler's run-state was visible to — and overwritable by —
whichever sibling was polled next. A handler that yielded *inside a `count` loop*
would resume and read a `count` set by another handler.

## The bug, concretely

`count` does not resolve through the environment while a count loop is active;
`try_evaluate_variable_sync` short-circuits on `in_count_loop` and reads
`self.current_count` directly. Both fields are global, so two concurrent count
loops share one `current_count`. A handler counting `1..5` that yields mid-loop
could come back reading `100..104` from a sibling.

## Fix — a poll-swap wrapper (no `Rc`→`Arc`, no threads)

The interpreter core stays single-threaded and `Rc`-based (a hard constraint).
Rather than thread a per-handler execution context through every `&self` method,
each handler owns a `RunState` snapshot and an `IsolatedHandler` future wraps the
handler:

- On **each `poll`**, `swap_run_state` swaps the handler's `RunState` into the
  interpreter's live fields (a field-by-field `mem::swap`, its own inverse).
- The inner handler future is polled.
- The instant `poll` returns — `Ready` **or** `Pending` — the state is swapped
  back out into the handler's `RunState`.

So the interpreter's run-state fields become effectively poll-local: exactly one
handler's state is installed at a time, and a suspended handler's state is parked
in its own `RunState` where no sibling can touch it. Each handler starts from
`RunState::fresh(base_call_depth)`. The inner future is already wrapped in
`catch_unwind`, so a panic surfaces as `Ready` and the swap-back still runs,
leaving the scratch fields clean for the next sibling.

Serial execution is completely untouched — `IsolatedHandler` is used only by
`execute_concurrent_main_loop`.

## Testing (Red → Green)

`tests/concurrent_main_loop_test.rs::test_concurrent_handlers_do_not_share_count_loop_state`:
two concurrent handlers each run a `count` loop over a **disjoint** range
(`1..5` vs `100..104`), yielding via `wait for` mid-iteration and then reading
`count`. With isolation each handler observes only its own range.

- **Red** (isolation bypassed — plain handler pushed to `FuturesUnordered`):
  `/a` returned `100-101-102-103-104-`, i.e. it observed the *other* handler's
  entire count range. `assertion left == right failed`.
- **Green** (isolation restored): `/a` → `1-2-3-4-5-`, `/b` →
  `100-101-102-103-104-`.

Risk class **R3** (concurrency + lifecycle). The test asserts a concrete wrong
outcome under sharing, not merely "did not crash".

## Follow-ups still open from the review

Larger P1 items remain and are tracked in
`Docs/development/concurrency-phase-plan.md`: immediate-500 on pre-respond
failure, browser-disconnect/504 cancellation threaded into `wait for` and
upstream reads, and an absolute total-stream deadline.
