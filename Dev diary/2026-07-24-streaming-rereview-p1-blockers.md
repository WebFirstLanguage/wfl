# Dev Diary — 2026-07-24: streaming re-review P1 blockers

The maintainer's re-review of the streaming/concurrency PR raised a set of P1
merge blockers on the runtime lifecycle. Each was closed with a **real-boundary**
(real-socket / real-binary) regression under the R3 profile, committed **Red
first** (a failing test-only commit that is an ancestor of the fix commit) per the
Testing Policy.

## #2 — a client disconnect is a normal cancellation, not a handler failure

The disconnect branch of a blocked upstream read returned a generic budget error
that the concurrent `main loop` fed into its single global consecutive-failure
breaker (backoff after every failure, break the whole loop at 256). A burst of
256 browser disconnects therefore tore the loop down — an ordinary "client hung
up," repeated, became a denial of service.

Fix: a distinct `HttpClientError::Disconnected` mapped to a new
`ErrorKind::Cancelled` (still catchable). The concurrent loop recognizes a
`Cancelled` handler outcome as an expected cancellation — it releases the handler
(its owned streams are already closed on unwind) without touching the failure
counter or backing off. Internal budget-cancellation keeps its `ResourceLimit`
kind, so only a real downstream disconnect is exempt.
*Test:* `concurrent_disconnect_burst_test` — 270 disconnects, then `/ping` is
still served (was refused; ~13 s → ~0.9 s).

## #3 — `outbound_stream_max_seconds` is a TRUE absolute lifetime

`next_line`/`next_chunk` served locally-buffered bytes before consulting the
handle's absolute deadline (only `stream_pull` checked it), so a proxy that pulled
a multi-line chunk kept draining the buffer past the stream's absolute lifetime.
Fix: `check_stream_deadline` runs before serving buffered bytes; on expiry the
handle is dropped (cancelling the upstream). An empty-buffer read already expired
via `stream_pull`'s identical check.
*Test:* `outbound_stream_absolute_lifetime_test`.

## #1 — per-request cancellation valid BEFORE and after the head

The disconnect signal came only from an open downstream response stream, which
does not exist during the upstream HEAD phase (`open url ... and stream response`,
before `start streaming response`). A browser that disconnected while the upstream
withheld its head was not noticed until the head timeout. Fix: a second disconnect
signal from the request's oneshot — the transport drops the receiver when the
client goes away, so the parked sender reports `is_closed()`
(`any_pending_request_disconnected`, polled). `any_client_disconnected` races both
signals; the head open **and** both body reads select against it, so a disconnect
cancels the handler in either phase (dropping the head future aborts the upstream
connection). Empirically hyper drops the pending route future's receiver on a
pre-head disconnect, so the poll observes it.
*Test:* `outbound_stream_head_disconnect_test` — upstream withholds its head, the
client disconnects, the upstream closes promptly, and an unrelated `/ping` is
still served.

## #4 — a dropped `interpret()` future still closes outbound handles

Handler-exit / program-cleanup sites close outbound handles on normal exits, but a
dropped/cancelled `interpret()` future runs none of them, leaking a parked
(opened, not mid-read) upstream until the interpreter itself is dropped. Fix: an
RAII `OutboundStreamCleanup` guard held for the whole `interpret_inner` body,
sharing `open_http_streams` and the `IoClient` via `Rc`, so its `Drop` closes the
tracked handles even as the future unwinds and the interpreter stays alive. On a
normal run the exit sites drain the list first, so the guard is a no-op.
*Test:* `dropped_interpret_cleanup_test`.

## #5 — a bracket index composes after a `.property` / `.method` access

`store ct as upstream.headers["content-type"]` mis-parsed into two statements
(`store ct as upstream.headers` + a stray `["content-type"]` list literal),
silently dropping the lookup. The identifier property-access / method-call fast
paths returned before the postfix loop could consume the `[...]`. Fix: route both
through `parse_trailing_bracket_index`, folding any chained `[...]` onto the base
(`grid.rows[0][1]`); the shared postfix loop and the static-member `.` arm are
untouched.
*Tests:* `property_index_access_test` (AST + runtime), and the strengthened dot
test in `stream_handle_type_test` (previously a false green — type-checking alone
passed on the split).

## #6 — drop a span-mismatched classic-write fallback

The ambiguous `write line|chunk <ident> ... to <target>` form kept the classic
file-write fallback whenever it merely parsed, even if it consumed a **different**
span than the stream reading — so `write line min with a: 1 and b: 2 to <file>`
retained a partial `line min with a` fallback (the classic reading stops at the
`:` the builtin-call stream reading consumes as a named arg), corrupting the file
write. Fix: keep the fallback only when it consumed exactly to the stream
reading's end checkpoint; otherwise there is no valid classic interpretation and
the non-stream target is a clean error.
*Test:* `write_line_backcompat_test` (two new cases; the matching-span back-compat
cases still pass).

## #7 (P2) — analyze the shared continuation of a desugared ambiguous write

The analyzer deferred ALL non-simple ambiguous `write line|chunk` values to
runtime, so an undefined variable in an operator continuation
(`write line value plus missing_suffix to ...`) went unflagged. Replaced the
simple-lead check with `analyze_ambiguous_write`, a parallel walk of the stream
and classic readings (parsed from the same tokens, differing only at the leftmost
leaf): a subtree identical under both readings is pure continuation and analyzed
normally; otherwise the walk recurses on BOTH children so a lead a desugaring
duplicated into the right operand (`is between`) is matched against the fallback's
copy instead of mis-flagged, and at a differing leaf reports undefined only when
NEITHER reading resolves. Call-based desugarings still defer.
*Tests:* `write_line_backcompat_test` (desugared-continuation flag +
operator-continuation no-false-positive), all prior back-compat cases intact.

## #8 (P2) — streaming-operand type enforcement + flush operand postfix

- **Operand types.** `wait for next chunk|line` now requires an `HttpStream`
  source, and `write line|chunk` / `flush` a `ResponseStream` target (the
  ambiguous merged write also accepts a text file-path target for its classic
  reading). Unknown/Any/Error still pass for gradual typing, so only a concrete
  non-stream operand is rejected — at typecheck instead of as a runtime surprise.
  *Tests:* `stream_handle_type_test`.
- **`flush` operand postfix (review feedback).** The lexer merges `flush` with the
  following identifier, so `flush streams["a"]` / `flush obj.out` left the accessor
  tokens dangling. Generalized the property-then-index helper into
  `parse_trailing_postfix` (folds both `[...]` index and `.field` property access
  onto a lead) and route `flush`'s split-off lead through it. Also anchored that
  helper's missing-`]` end-of-input diagnostic to the `[` token's byte span rather
  than the file start (review feedback). *Tests:* `http_server_streaming_test`.

## Test infrastructure / CI

- Server integration tests now bind an OS-assigned free port
  (`tests/common/free_tcp_port`) instead of a hardcoded constant, removing a
  parallel-run flakiness class (review feedback).
- The heavy CI build jobs free ~20 GB of unused preinstalled SDKs on Linux before
  building; the debuginfo-heavy release tree plus every integration test binary
  was exhausting a runner's disk mid-link (linker `Bus error`/SIGBUS).

## Risk class & residual risk

- **R3** (concurrency / cancellation / lifecycle / streaming). Real-boundary
  tests, negative assertions (the connection actually closes / the read actually
  fails / a burst does not tear the loop down), Red evidence for each fix.
- The pre-head disconnect signal is **polled** (20 ms) because the request's
  oneshot sender lives behind an `Arc<Mutex<Option<..>>>` shared with the
  transport, not an awaitable primitive; the downstream-response-stream signal
  stays event-driven. A fully idle handler that opens an outbound stream and never
  reads it is still reclaimed at handler exit rather than by a mid-idle timer — the
  single-threaded, `!Send`-stream model has no wake point to close it earlier;
  this is noted rather than claimed as instantaneous.
