# Dev Diary — 2026-07-22 — Concurrent request handlers (`main loop concurrently:`)

## Context

Final piece of the five-capability streaming request (item 4). With outbound and
server streaming shipped, a handler can proxy a slow upstream to the browser —
but on the **serial** `main loop`, that slow handler blocks every other request
(login, history, health, other chats). This adds opt-in concurrent handling so a
slow stream no longer stalls its siblings.

This maps onto **Phase 1** of `Docs/development/concurrency-phase-plan.md` — a
maintainer-locked, gated plan. I followed its hard rules (locked marker `main
loop concurrently:`, no `Rc→Arc`/`Send` rewrite of the interpreter core, plain
`main loop` stays serial, TDD, `panic = "unwind"` gate already in CI). Phase 1
landed in one change rather than the staged 1a→1b→1c; **this is the plan's
maintainer STOP/review point.**

## What shipped

```wfl
listen on port 8080 as server
main loop concurrently:
    wait for request comes in on server as req
    // a slow handler here (e.g. streaming a slow upstream) no longer blocks siblings
    respond to req with "Hello!"
end loop
```

Plain `main loop` is unchanged (strictly serial, byte-compatible). Adding
`concurrently` is the only way to opt in — no silent semantics swap.

## Design & mechanism

- **AST:** `MainLoop` gained a `concurrent: bool` (default false). `concurrently`
  is a contextual identifier parsed only right after `main loop`, so it stays
  usable as a variable name elsewhere.
- **Execution:** `execute_concurrent_main_loop` keeps up to
  `CONCURRENT_HANDLER_LIMIT` (256) iterations of the body in flight via a
  `FuturesUnordered` of `!Send`, non-`'static`, `&self`-borrowing handler
  futures — cooperative concurrency on the one interpreter thread, exactly as the
  plan's 1a spike prescribes (no `spawn_local`, no `Send`/`Arc` across the core).
  Each iteration runs in a fresh `Environment::new_child_env` (isolation by
  default). The set is refilled to the cap, so with cap ≥ 1 it is never empty —
  avoiding the `Ready(None)` busy-spin trap.
- **Containment:** each handler future is wrapped in
  `AssertUnwindSafe(...).catch_unwind()`. A panicking handler is caught, its
  request is answered 500 by the existing `ResponseCompletion` drop guard, and
  siblings keep running. A handler that returns a `RuntimeError` is likewise
  logged and contained.
- **Ops defaults reused:** 503 (bounded transport queue), 504 (per-request
  response deadline), and 500 (drop guard) already existed at the transport
  layer, so the concurrent loop inherits them — it adds handler-level
  concurrency, not a parallel ops stack.
- **Why `wait for request` allows this:** it holds the server's receiver mutex
  only to dequeue one request, then releases it before the handler body runs — so
  concurrent iterations hand off requests one at a time and then handle them
  concurrently. No `Rc`/`RefCell` is held across an `.await` (the crate-wide
  `#![deny(clippy::await_holding_refcell_ref)]` backstop enforces this).

## Concurrent, not parallel

Cooperative concurrency on a single thread: handlers interleave at their await
points (`wait for`, outbound HTTP, stream read/write, `respond`). A tight
CPU-bound handler with no await still holds the thread — documented as the yield
cliff. This is the honest model, not multicore parallelism (Phase 3, deferred).

## Files

- `src/parser/ast.rs` (`MainLoop.concurrent`), `src/parser/stmt/control_flow.rs`
  (parse `concurrently`), `src/interpreter/mod.rs`
  (`execute_concurrent_main_loop`, `CONCURRENT_HANDLER_LIMIT`, MainLoop branch).
- Docs: `Docs/04-advanced-features/web-servers.md` ("Concurrent request
  handling") + validated example
  `TestPrograms/docs_examples/web_servers/concurrent_main_loop.wfl`;
  `concurrency-phase-plan.md` tracker updated.

## Tests

`tests/concurrent_main_loop_test.rs`:
- `main loop concurrently:` parses concurrent; plain `main loop:` stays serial.
- Concurrent: a 500 ms handler does not block a fast sibling (fast < 300 ms).
- Serial: the same slow handler *does* block the next request (fast > 300 ms) —
  proving no silent upgrade.
- Handler-error containment: an erroring handler doesn't kill the server.

`fmt`, `clippy -D warnings`, the 618 lib tests, and the existing web-server /
streaming suites are all green.

## Out of scope (Phase 2+, per the plan)

- Structured nursery / `wait for all|any of` / `change shared`.
- Multicore (Phase 3; deferred, needs profiling).
- Request-ID *structured* logging scheme and a written per-site eval-core audit
  (mechanically enforced by the clippy backstop for now).
