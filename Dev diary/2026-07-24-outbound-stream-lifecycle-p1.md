# Dev Diary — 2026-07-24: outbound-stream lifecycle P1s (deadline + ownership + disconnect)

The two remaining P1 blockers from the maintainer's streaming re-review, each
closed with a **real-socket** Red→Green regression (mock upstream via
`tokio::TcpListener` + the WFL interpreter), which is the boundary evidence the
Testing Policy requires for R3 concurrency/streaming/lifecycle work.

## P1-B — the absolute stream deadline must bound an ACTIVE read

**Bug:** `stream_pull` computed the right per-read bound —
`min(idle_timeout, remaining_absolute)` — and handed it to `run_http_with_budget`
as `configured_timeout`. But `run_http_with_budget`/`outbound_http_deadline`
derived the operation timeout purely from the run/budget duration and *discarded*
`configured_timeout` (using it only as a fallback). So with `timeout_seconds = 10`
and `outbound_stream_max_seconds = 1`, a head-then-stall upstream let
`wait for next chunk` wait ~10s instead of ~1s. The absolute clock also started
only after the head arrived, excluding connect/header time.

**Fix:** compose the operation deadline as `MIN(configured_timeout, budget)` and
report the stream `Timeout` vs the budget `Deadline` depending on which bound
fired; `configured_timeout` is always finite, so the read is bounded even with no
run-wide budget deadline. Start the absolute clock at request initiation and bound
the head phase by it too.

**Test:** `outbound_stream_deadline_test` — mock sends head then stalls; read now
fails at ~1s (was ~10s, verified Red).

## P1-A — outbound streams are handler-owned, and disconnect cancels a blocked read

Two parts:

**Part 1 — ownership / close-on-every-exit.** Outbound `httpstream*` handles lived
only in the interpreter-wide `IoClient.stream_handles`; `RunState`/
`IsolatedHandler::drop` closed downstream response streams and pending requests
but not upstream handles, so an abandoned proxy read leaked the upstream until the
whole interpreter tore down. Now `RunState.open_http_streams` tracks them
per-handler (swapped per poll); added on open, untracked on EOF/error/explicit
close, and dropped from the map on every handler exit (`IsolatedHandler::drop`
for the concurrent loop; `close_open_http_streams()` at the serial-loop/program
cleanup sites) — dropping the reqwest stream cancels the upstream.
*Test:* `outbound_stream_ownership_test` — a run ends without `close` while the
interpreter is still alive; the mock sees its client disconnect only because
handler-exit cleanup cancelled the upstream (verified Red by disabling cleanup).

**Part 2 — disconnect cancels a blocked upstream read.** A proxy handler blocked
in `wait for next line|chunk` on the upstream had no disconnect signal — it only
noticed at the next downstream `write` (or, after P1-B, the absolute deadline).
The downstream response stream's `mpsc::Sender::closed()` resolves when hyper
drops the client's body receiver, so we clone the handler's open-response-stream
senders (no `RefCell` borrow across the await) and `select!` the upstream read
against "any downstream disconnected". On disconnect the read future is dropped
(cancelling the upstream), the handle is closed, and a catchable `Cancelled` error
unwinds the handler (whose owned cleanup then runs).
*Test:* `outbound_stream_disconnect_test` — a WFL concurrent proxy relays a mock
upstream that stalls after one chunk; the client reads the first chunk and
disconnects while the handler is blocked; the mock observes its own connection
close within the window only because the blocked read was cancelled (verified Red
by disabling the `select!`).

## Risk class & residual risk

- **R3** (concurrency/cancellation/lifecycle/streaming). Real-boundary tests,
  negative assertions (connection actually closes / read actually fails), and
  Red evidence for each.
- Part 2 selects against the handler's currently-open response streams; a handler
  with no downstream stream (a pure client-side reader) keeps the plain read path
  (`pending` disconnect branch), so there is no behavior change there.
- `close_http_streams` in `Drop` is best-effort via `try_lock`; if the async lock
  is momentarily held the handles are reclaimed at interpreter teardown (they no
  longer leak *past* that, and in practice the lock is free at handler exit).
- Docs (`interoperability.md`, `response-streaming-design.md`) updated to state the
  now-accurate read bound (min idle/absolute), handler-exit release, and the
  proactive disconnect cancellation.
