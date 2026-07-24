# Dev Diary — 2026-07-22 — Immediate 500 when a handler ends without responding

## Context

PR #641 review (maintainer P1 #3, echoed by CodeRabbit) flagged that the
`ResponseCompletion` drop guard does **not** cover every pre-response path. The
guard is armed only once a handler *reaches* a `respond` / `start streaming
response` statement (it takes the request's sender out of `pending_responses`
into the guard). A handler that dequeues a request with `wait for request comes
in` and then ends **before** responding — a runtime error, a `break`, or simply
returning without `respond` — leaves the sender parked in `pending_responses`.
The client then waits out the request timeout instead of getting a prompt 500.

## Fix — arm the fallback at dequeue, disarm at respond

Each handler now tracks the request ids it dequeued but has not answered, in
per-handler run-state (`open_pending_requests`, part of the `RunState` swapped
per poll — the same mechanism that isolates `count`/recursion state and tracks
open response streams). On exit, any still-unanswered request is answered 500:

- `wait for request comes in` pushes the request id.
- `respond` / `start streaming response` remove the id from the map
  (`pending_responses.remove`) *and* disarm the tracking. Because a responded
  request is gone from the map, the exit-time sweep is naturally idempotent — it
  only 500s ids still present.
- On handler exit (any path) the tracked ids are swept:
  `fail_unanswered_requests` removes each from `pending_responses`, and if the
  sender is still live (`try_lock` + `oneshot::send`, fully synchronous) sends a
  500. Wired at every boundary, mirroring the stream close-on-exit:
  `IsolatedHandler`'s `Drop` (concurrent), the serial `main loop`'s per-iteration
  drain, and program exit.

The sweep is synchronous so it runs from `Drop`. The sender's mutex is only held
during `respond`, which a finished handler is no longer inside, so `try_lock`
succeeds; the transport request timeout remains the backstop for the impossible
case where it does not.

## Testing (Red → Green)

`tests/concurrent_main_loop_test.rs::test_handler_that_never_responds_gets_immediate_500`:
a `main loop concurrently:` server whose `/drop` path dequeues the request and
ends without responding. The client must get a **prompt 500**, and the server
must keep serving (`/ok` still works).

- **Red** (500-on-exit disabled): the `/drop` request hangs — the client waits
  out the transport timeout, exceeding even the 120s test harness timeout, i.e.
  exactly the "client left waiting" symptom.
- **Green**: `/drop` returns 500 in well under a second; `/ok` still returns
  `ok`.

Risk class **R3** (lifecycle). The test asserts both the status *and* that it
arrives before the timeout, so a regression to "eventually times out" fails.

## Also in this change (from review)

- The response-byte-ceiling and client-disconnect paths now also untrack the
  stream id from `open_response_streams`, so a handler that catches the error and
  continues keeps no stale ids.
- Typechecker: HTTP/response header type hints widened to `map[text, any]`
  (values may be text, numbers, or booleans, converted to text).
