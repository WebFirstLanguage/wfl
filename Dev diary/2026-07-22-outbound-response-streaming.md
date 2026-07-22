# Dev Diary — 2026-07-22 — Generic outbound response streaming

## Context

A downstream app (a browser chat UI talking to a model endpoint) needs the WFL
runtime to proxy a slow upstream to the browser without buffering. That request
came in as five items: (1) outbound response streaming, (2) incremental
chunk/line reads, (3) streamed *server* responses, (4) concurrent request
handlers, and (5) lifecycle guarantees (timeouts, backpressure, cancellation,
catchable errors, close-on-every-exit).

Items 3 and 4 are large and, importantly, item 4 (concurrent handlers) is
**already governed** by `Docs/development/concurrency-phase-plan.md` — a
maintainer-locked, gated plan (locked marker `main loop concurrently:`, "no
Rc→Arc rewrite of the interpreter core", TDD-first, stop-for-review between
phases). This entry covers the first shippable slice: **items 1, 2, and the
streaming-relevant parts of item 5 (outbound/client side)**, which are net-new
and do *not* touch the locked concurrency core. Server streaming (3) and the
concurrent loop (4) are separate follow-on changes.

## What shipped

New surface, mirroring the existing `open url` client:

```wfl
open url at "<url>" [with method .. and headers .. and body ..] and stream response as upstream
wait for next line  from upstream as line   // Text, or nothing at clean EOF
wait for next chunk from upstream as chunk   // Binary, or nothing at clean EOF
close upstream                               // cancels the in-flight upstream
```

`stream response as` returns as soon as the status/headers arrive — **without
buffering the body** — and binds an object exposing `status`, `ok`, `headers`,
and an internal `_stream` id. The body stays parked in the interpreter and is
pulled incrementally.

## Design notes

- **No new lexer tokens.** `stream`, `next`, `chunk`, `line`, `upstream` are all
  contextual identifiers; `response`/`from`/`as` are existing keywords. The
  lexer's identifier-merging means `next chunk`/`next line` arrive as a single
  token, handled in `parse_wait_for_statement`.
- **Handle model follows the existing pattern.** Open resources in WFL are
  opaque ids into side-tables on `IoClient` (files, DB pools, processes). Added
  `stream_handles: Mutex<HashMap<String, HttpStreamHandle>>`. `HttpStreamHandle`
  holds a `Pin<Box<dyn Stream<Item = reqwest::Result<Vec<u8>>> + Send>>` (from
  `response.bytes_stream()`), a leftover-byte buffer for line splitting, a
  `done` flag, and a running `bytes_read` total.
- **No lock held across the network await.** `next_chunk`/`next_line` *take* the
  handle out of the map, await, then put it back — so a slow read on one stream
  never blocks map access for another (this matters once concurrent handlers
  land).
- **Lifecycle (item 5, client side).** The head phase and each per-chunk read go
  through the existing `run_http_with_budget` select, so connect/read timeouts,
  the response-byte ceiling (`web_server_max_response_size`, enforced
  incrementally on the running total, not just `Content-Length`), and
  cooperative cancellation all apply. Mid-stream network errors surface as
  catchable `RuntimeError`s from the `wait for next ...` statement. Dropping the
  handle — on clean EOF, error, explicit `close`, or interpreter teardown —
  drops the reqwest body future and cancels the upstream. Reading a
  closed/drained handle is a predictable catchable error.
- **`close` extended, not duplicated.** `close <x>` already closed files;
  it now also accepts a streaming-response object and closes its stream.
- **EOF semantics.** A final unterminated line is delivered before EOF; the
  handle is re-inserted (drained, `done`) so the *next* read returns `nothing`
  once, then the handle is freed — the `check if line is nothing: break` loop
  works and handles don't leak across many streamed requests.

## Files

- AST: `HttpStreamStatement`, `WaitForNextChunkStatement`,
  `WaitForNextLineStatement` (`src/parser/ast.rs`).
- Parser: `stream response as` clause (`src/parser/stmt/io.rs`), `wait for next
  chunk|line from` (`src/parser/stmt/processes.rs`).
- Interpreter: `IoClient::{open_http_stream, next_chunk, next_line,
  close_stream}` + helpers, three statement arms, `close` extension
  (`src/interpreter/mod.rs`).
- Analyzer/typechecker/transpiler: variable-binding registration and an explicit
  "not supported in JS transpilation" arm.
- Docs: `Docs/04-advanced-features/interoperability.md` (new "Streaming a
  response incrementally" section) + validated example
  `TestPrograms/docs_examples/interoperability/streaming_response.wfl`.

## Tests

`tests/http_stream_test.rs` — parser tests for all three statements, and
offline runtime tests against a local one-shot TCP server: status/headers
available immediately, `next line` yields lines then `nothing`, a final
unterminated line is delivered, `next chunk` yields binary, and reading a closed
stream is an error. `cargo fmt`, `clippy -D warnings`, and the existing
`http_request_*` / `http_outbound_budget` suites are green.

## Not in this change (follow-ons)

- **Server-side streaming** (`write chunk`/`flush`/`close` on a response) —
  needs the `oneshot<WflHttpResponse>` reply path reworked into a chunked body
  channel through warp.
- **Concurrent request handlers** — Phase 1 of the concurrency plan
  (`main loop concurrently:`); the keystone that makes a slow upstream stream
  not stall other requests. Follows the gated plan, not this change.
