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


---

# Second re-review round — deeper P1 blockers (head `e8c9712`)

Round of fixes for the maintainer's re-review at `e8c9712`. Risk class **R3**
(concurrency, cancellation, lifecycle, streaming, backward compatibility). Each
behavioral change has a Red→Green real-boundary test; the Red evidence is a
test-only commit that is an ancestor of the source commit (verified by running
each new test with the source fixes stashed).

## Cancellation / disconnect

- **`wait for next line` pre-response disconnect (P1-1).** The line read watched
  only the downstream response stream, which does not exist before `start
  streaming response`; it now races the same combined pending-request/downstream
  signal as `wait for next chunk`, so a blocked pre-response line read is
  cancelled the moment the client goes away. *Test:*
  `wait_line_pre_response_disconnect_test`.
- **Sibling-prune cancellation race (P1-2).** `any_pending_request_disconnected`
  treated an owned request id missing from `pending_responses` as "still
  connected". But the only removal that leaves an id in `open_pending_requests`
  is a sibling `wait for request`'s global prune, which deletes ONLY closed
  (disconnected) senders — so a missing owned id is now treated as a terminal
  disconnect, and a parked pre-head handler is no longer stranded until its idle
  timeout. *Test:* `concurrent_prehead_prune_race_test`.
- **Classify every client disconnect as cancellation (P1-3).** The buffered
  `respond`, streaming-head, and response-stream `write` send failures returned a
  General runtime error, which fed the concurrent loop's structural-failure
  breaker — so a burst of >256 disconnects at those paths tore the loop down.
  They are now `ErrorKind::Cancelled`. *Test:*
  `concurrent_disconnect_paths_burst_test` (buffered-respond and stream-write
  bursts; `/ping` survives).

## Lifecycle (P1-4)

- **Absolute outbound lifetime is real-time (a).** `outbound_stream_max_seconds`
  was only re-checked on the next read, so an opened-but-unread upstream outlived
  the cap. `stream_handles` is now shared via `Arc` and each open spawns a reaper
  that drops the handle (cancelling the upstream) when the deadline elapses.
  *Test:* `outbound_stream_open_expiry_test`. *Docs:* configuration-reference
  updated to state the cap is enforced in real time.
- **Dropped-run cleanup covers server streams + pending (b).** The
  interpret-scoped guard covered only outbound streams; the server response
  streams and pending requests (`Rc`-shared now) are also finalized on a dropped
  `interpret()`, so a cancelled run does not leave a client body hanging on a
  reused interpreter. *Test:* `dropped_interpret_server_cleanup_test` (holds the
  interpreter alive after the drop to prove it is the guard, not interpreter
  teardown, that closes the body).
- **Backpressured write is bounded (c).** `tx.send(bytes).await` past the 64-slot
  channel could park forever against a connected-but-non-reading client (a `main
  loop` is deadline-exempt). It is now capped by
  `web_server_response_timeout_seconds`. *Test:*
  `response_stream_backpressure_test` (a >send-buffer payload genuinely blocks;
  the write fails at the cap instead of pinning). *Docs:* config reference notes
  this timeout bounds streaming writes.

## Backward compatibility / correctness

- **Branch-aware ambiguous-write type check (P1-5).** `write line|chunk <v> to
  <target>` has a stream reading and a classic file-write reading; the checker
  now validates the reading the runtime actually takes (by the target type),
  instead of always checking the stream `value` — so a valid file write is no
  longer rejected on the never-run stream branch, a broken file write is caught,
  and a concrete non-streamable payload (Map/List/Nothing) to a real stream is a
  static error. *Test:* `ambiguous_write_branch_typecheck_test`.
- **`flush` no longer steals a zero-arg action (P1-6).** `flush cache` used to
  auto-invoke an action named `flush cache`; the streaming `flush` dispatch now
  carries the full merged phrase and the interpreter/typechecker/analyzer prefer
  a defined action of that name before treating the operand as a stream. *Test:*
  `flush_action_backcompat_test`.
- **Postfix composition on write / web-clause operands (P1-9).** `write line
  chunks[0] to out`, `write line upstream.status to out`, `headers
  upstream.headers`, and `content type upstream.headers["content-type"]` compose
  their trailing `[...]`/`.field` accessors instead of leaving them to dangle.
  *Test:* `write_web_postfix_test`.
- **Analyzer walks call/pattern continuations (P1-10).** `analyze_ambiguous_write`
  now recurses in parallel through `starts/ends with`, pattern, index, and
  function/action/method-call shapes, so an undefined name in a shared
  continuation is reported instead of reaching runtime. *Test:*
  `ambiguous_write_analyzer_test`.

## Test infrastructure / CI

- Clippy runs `--all-features` (matching the binding gate in `testing.md`).
- The Integration Tests job runs the documented
  `scripts/run_integration_tests.{sh,ps1}` on both OSes, so the intentional-error
  TestPrograms are actually asserted (their assertions lived only in that script,
  which CI never invoked).
- `run_integration_tests.ps1` redirects stdout/stderr to two distinct temp files
  (PowerShell 7 rejects reusing a single `NUL` target, which left the Windows
  integration command unrunnable).
- `run_web_tests.ps1` fails the run — not merely warns — when a server cannot be
  killed/waited or a TLS temp dir leaks, since the pass is counted before the
  `finally` cleanup.
- Streaming visibility coverage: `response_stream_backpressure_test` also proves
  an early chunk is delivered on the wire ~2s before the late one (head/first
  chunk visible before body completion, not buffered to close).
