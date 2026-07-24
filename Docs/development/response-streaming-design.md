# WFL Response Streaming — Design & Status

**Audience:** WFL maintainer + AI implementers
**Origin:** downstream request (2026-07-22) for a browser chat UI proxying a slow
upstream model endpoint to the browser without buffering.

This tracks the five requested runtime capabilities, what has shipped, and the
locked design for what remains. It complements — and defers to —
`concurrency-phase-plan.md`, which governs item 4.

---

## The five capabilities

| # | Capability | Status |
|---|------------|--------|
| 1 | Outbound response streaming (`stream response as`) | ✅ Shipped |
| 2 | Incremental reads (`wait for next chunk\|line`) | ✅ Shipped |
| 3 | Streamed server responses (start / write / flush / close) | ✅ Shipped |
| 4 | Concurrent request handlers | ✅ Shipped (`main loop concurrently:`; Phase 1 of `concurrency-phase-plan.md`, awaiting maintainer review) |
| 5 | Lifecycle (timeouts, backpressure, cancellation, catchable errors, close-on-exit) | ✅ Client + server streaming + per-handler isolation/containment shipped |

---

## Shipped (items 1, 2, client-side 5)

See `Dev diary/2026-07-22-outbound-response-streaming.md`. Surface:

```wfl
open url at "<url>" [with method .. and headers .. and body ..] and stream response as upstream
wait for next line  from upstream as line     // Text,   nothing at clean EOF
wait for next chunk from upstream as chunk     // Binary, nothing at clean EOF
close upstream
```

Handle model: an opaque id into `IoClient.stream_handles`, wrapped in an object
exposing `status`/`ok`/`headers`/`_stream`. Reads go through the existing
`run_http_with_budget` select (timeouts + cancellation), enforce the
response-byte ceiling on the running total, are individually catchable, and drop
the handle (cancelling the upstream) on EOF/error/close/teardown.

---

## Item 3 — Streamed server responses (✅ shipped)

Shipped as designed below. Surface: `start streaming response to <req> [with
status <e>] [and content type <e>] [and headers <e>] as <out>`, `write
line|chunk <value> to <out>`, `flush <out>`, `close <out>`. See
`tests/http_server_streaming_test.rs` and the web-servers guide's "Streaming a
response" section. The original design (kept for reference):

## Item 3 — Streamed server responses (design)

### Surface (chosen for consistency with the client side + existing `respond`)

```wfl
// Send status + headers immediately; body stays open.
start streaming response to req with status 200 and content type "application/x-ndjson" as out

// Write body pieces. `write line` appends a newline (NDJSON-friendly);
// `write chunk` writes raw bytes/text verbatim.
write line json_text to out
write chunk raw_bytes to out

// Advisory: hand queued bytes to the transport (yields to the runtime).
flush out

// End the response body.
close out
```

- `start streaming response` leads with the merged identifier `start streaming`
  followed by the `response` keyword — dispatched like `send websocket message`
  in `parser/mod.rs`. (Distinguishable from any future Phase-2 `start <action>
  as <handle>`.)
- `write line|chunk <expr> to <out>` — branch inside the `write` dispatch on a
  following `line`/`chunk` identifier; `to` (not `into`) distinguishes it from
  file writes.
- `flush <out>` — leading identifier `flush`.
- `close out` — reuse `CloseFileStatement`, extended for a `_server_stream`
  object (as it already was for `_stream`).

### Mechanism

- **Reply payload becomes an enum.** Replace `oneshot::Sender<WflHttpResponse>`
  with `oneshot::Sender<HandlerReply>` where

  ```rust
  enum HandlerReply {
      Buffered(WflHttpResponse),
      Streaming { status: u16, content_type: String,
                  headers: HashMap<String, String>,
                  body: mpsc::Receiver<Vec<u8>> },
  }
  ```

  Update `PendingResponseSender`, `WflHttpRequest.response_sender`,
  `ResponseCompletion` (its `Drop` sends `Buffered` 500), and the `respond` path
  (builds `Buffered`).

- **Transport reply type becomes `warp::hyper::Body`.** Convert the five reply
  helpers (`overloaded_response`, `plain_status_response`,
  `payload_too_large_response`, `gateway_timeout_response`,
  `request_timeout_response`), `handle_overloaded`, and the main route's
  buffered arm from `Response<Vec<u8>>` to `Response<Body>` (`.body(Body::from(
  bytes))`). The streaming arm builds
  `Body::wrap_stream(futures_util::stream::unfold(rx, ...))` (no new dep). The
  separate redirect `warp::serve` route is untouched.

- **`start streaming response`** creates a bounded `mpsc::channel::<Vec<u8>>`
  (capacity = backpressure knob, align with existing web limits), sends
  `HandlerReply::Streaming { head.., body: rx }` over the request's oneshot, and
  stores the `tx` in a new interpreter-side map
  `server_response_streams: RefCell<HashMap<String, ServerStreamHandle>>`. Binds
  `out` = object `{ _server_stream: <id> }`.

- **`write line|chunk`** resolves `_server_stream`, `tx.send(bytes).await`
  (bounded → backpressure). A closed receiver (browser disconnected / hyper
  dropped the body) makes `send` fail → surfaced as a catchable error. In
  addition, a handler blocked in an upstream `wait for next line|chunk` no longer
  has to wait for its next `write` to notice the disconnect: the read is
  `select!`ed against `Sender::closed()` for the handler's open response streams,
  so a downstream disconnect cancels the blocked upstream read promptly (dropping
  the upstream), and the handler's owned outbound handles are then closed as it
  unwinds. This is how browser-disconnect cancellation propagates to the upstream
  (item 5, cooperatively).

- **`flush`** = advisory; `tokio::task::yield_now().await` so the transport task
  is scheduled. Documented as advisory (hyper already writes as it receives).

- **`close out`** drops the `tx` → ends the body stream → hyper finalizes the
  response. Idempotent; writes after close fail predictably.

### Lifecycle (item 5, server side)

- Timeouts: the existing per-request `overall_deadline` bounds head delivery; a
  stalled *body producer* is bounded by the handler timeout at await points (the
  yield-cliff caveat from the concurrency plan). Outbound reads are additionally
  bounded by `min(idle, remaining absolute stream deadline)`: `run_http_with_budget`
  composes the operation deadline as the minimum of the run/budget remaining time
  and the caller's configured timeout (which already carries the stream's idle +
  absolute bound), and the absolute clock starts at request initiation.
- Backpressure: bounded `mpsc` — a slow browser slows the handler's `write`.
- Disconnect → upstream cancel, in BOTH phases: a blocked upstream operation is
  `select!`ed against `any_client_disconnected`, which fires on EITHER an open
  downstream response stream's `Sender::closed()` (post-`start streaming
  response`, event-driven) OR the request's oneshot receiver being dropped
  (pre-`start streaming response` — the head phase — polled via
  `is_closed()`). So a browser disconnect cancels the handler whether it is
  blocked opening the upstream head or reading its body, dropping the upstream.
- Disconnect is a normal cancellation, not a handler failure: it unwinds with
  `ErrorKind::Cancelled`, which the concurrent `main loop` treats as an expected
  outcome (it does NOT feed the structural consecutive-failure breaker), so a
  burst of disconnects cannot tear the loop down.
- Absolute lifetime (`outbound_stream_max_seconds`) is enforced before EVERY
  read return — including reads served from locally-buffered bytes — not only on
  a network read, so a buffered drain cannot outlive the stream's absolute cap.
- Outbound close-on-exit (shipped): outbound `httpstream*` handles are also
  handler-owned — tracked in `RunState.open_http_streams` (swapped per poll) and
  dropped from `IoClient.stream_handles` when the handler ends on any path,
  cancelling the in-flight upstream request so an abandoned proxy read never leaks.
  A cancelled/dropped `interpret()` future (an embedder cancels the run) also
  closes them via an RAII guard, rather than leaking until interpreter teardown.
- Close-on-exit (shipped): each handler tracks the `respstream*` ids it opened in
  its per-handler run-state (`open_response_streams`, part of the `RunState`
  swapped in/out per poll under `main loop concurrently:`). When the handler ends
  on **any** path — normal return, caught error, or a panic contained by
  `catch_unwind` — those ids are removed from `server_response_streams`, dropping
  their `tx` and ending the response body. Concurrent handlers close via
  `IsolatedHandler`'s `Drop`; the serial `main loop` drains per iteration; a
  top-level stream closes at program exit. So a handler that forgets `close out`
  never leaves the client hanging, and the table cannot leak dead senders.
  Explicit `close out` is still preferred to finalize promptly.

### Tests (write first)

Parser tests for all four statements; runtime tests via a client that reads the
streamed body: status/headers arrive before the body, `write line` frames NDJSON,
`close` ends the stream, a dropped client makes `write` fail catchably.

---

## Item 4 — Concurrent handlers

Governed by `concurrency-phase-plan.md` (Phase 1, locked marker `main loop
concurrently:`, no Rc→Arc, TDD-first). It is the keystone that makes a slow
streamed response (items 1–3) not stall login/history/health. Server streaming
(item 3) is deliberately usable on the serial loop first; true isolation between
a slow stream and other requests arrives with Phase 1.

---

## Related

- `Docs/04-advanced-features/interoperability.md` — user docs (client streaming shipped)
- `Docs/development/concurrency-phase-plan.md` — item 4 governance
- `Dev diary/2026-07-22-outbound-response-streaming.md`
