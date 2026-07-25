# Dev Diary — 2026-07-22 — Auto-close server response streams on handler exit

## Context

PR #641's streamed server responses (`start streaming response` / `write` /
`flush` / `close out`) parked the body-channel **sender** in a long-lived
interpreter table, `server_response_streams`, keyed by an opaque `respstream*`
id. The transport turns the matching receiver into the chunked response body via
`Body::wrap_stream`, which only ends once **every** sender is dropped.

Review (Devin 🐛, Copilot, CodeRabbit) flagged the consequence: the sender was
dropped in only two places — an explicit `close out` and the write-after-
disconnect path. A handler that started a stream and ended **without** `close out`
(normal return, a caught error, a `break`) left its sender in the table forever:

- the client's chunked body was never terminated → **the client hangs**, and
- the table grew one dead entry per streamed request → **a memory leak**.

This also contradicted the shipped docs and design doc, which promised
"close-on-exit," and the original streaming spec's item 5 lifecycle guarantee:
*all streams close on every exit path.*

## Fix — tie each stream's lifetime to its handler

Each handler now tracks the `respstream*` ids it opens and closes them when it
ends, on **every** path:

- New per-handler field `open_response_streams` lives in the interpreter and is
  part of the `RunState` swapped in/out per poll (the same poll-local mechanism
  that isolates `count`/recursion state under `main loop concurrently:`), so each
  handler tracks only its own streams even while interleaved.
- `start streaming response` pushes the new id; an explicit `close out` removes it
  (keeping the list bounded to genuinely-open streams).
- `close_response_streams(ids)` removes each id from `server_response_streams`,
  dropping its sender and ending the body. It is idempotent — an id already closed
  is a no-op — so closing a handler's whole opened-list on exit is always safe.

Close-on-exit is wired at every boundary:

- **Concurrent handlers:** `IsolatedHandler`'s `Drop` closes the handler's
  `state.open_response_streams`. Drop runs whether the handler returned, errored,
  panicked (contained by `catch_unwind`), or was cancelled as the loop tore down.
- **Serial `main loop`:** each iteration drains and closes after `execute_block`,
  on the normal *and* error paths.
- **Top level:** `interpret_inner` drains at program exit for streams opened
  outside any loop, and clears the tracking on run entry (REPL reuse).

## Testing (Red → Green)

`tests/http_server_streaming_test.rs::test_stream_auto_closes_when_handler_ends_without_close`:
a handler starts a stream, writes one line, and ends **without** `close out`. The
client reads the body under a 5s `tokio::time::timeout`.

- **Red** (auto-close drain disabled): the body never finishes —
  `timeout ... Elapsed(())`; the read hangs exactly as a real client would.
- **Green** (auto-close restored): body reads back `"hello\n"` and completes.

Risk class **R3** (lifecycle/streaming). The negative outcome (a hang) is turned
into a deterministic failure by the timeout rather than a stuck test.

## Docs

`web-servers.md`, `response-streaming-design.md`, and the server-streaming dev
diary updated to describe the shipped close-on-exit behavior. Explicit `close out`
is still recommended to finalize promptly (and free the connection sooner);
auto-close is the safety net, not a substitute.
