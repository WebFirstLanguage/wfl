# Dev Diary — 2026-07-24: issue #642 PR #641 follow-up P1s

Follow-up to the exact-head re-review of #641 (`b25aed57`). CI was green but five
P1 lifecycle/compatibility blockers remained. Risk class **R3** (concurrency,
cancellation, lifecycle, streaming, compatibility).

## P1.1 — request-local failures must not stop the concurrent server

- Sticky `accepted_request` on `RunState` / `IsolatedHandler` output.
- Concurrent loop only feeds structural pre-request failures into the 256-breaker.
- `Cancelled`, `Timeout` (finite `wait for request`), and any post-accept error/panic
  are non-structural.
- `wait for request ... with timeout` expiry uses `ErrorKind::Timeout`.
- Missing pending while the handler still owns the request → `Cancelled` (sibling
  prune of a disconnected client); duplicate respond when not owned stays General.
- Tests: strengthened `concurrent_disconnect_paths_burst_test` (assert connected
  count, 15s drain, pre-streaming-head path, wait-timeout survival).

## P1.2 — outbound hard-lifetime reaper ownership

- `StreamSlot` with handle / deadline / expired tombstone / `AbortHandle`.
- Reaper marks expired and drops parked handles; mid-read `put_stream` refuses
  reinsertion and returns `Timeout`.
- EOF/error/close abort the reaper timer (bounded open/close cost).
- `outbound_stream_deadline` clamps extreme `u64` config (no Instant panic).
- Tests: `outbound_stream_reaper_race_test` (active-read near deadline + rapid
  open/close).

## P1.3 — ambiguous `write line|chunk` soundness

- Typechecker validates definedness + payload for the concrete branch; gradual
  targets validate **both** branches.
- `main loop` / `forever` typecheck push a scope; `start streaming response`
  always binds `ResponseStream`.
- Analyzer walks `PropertyAccess` in the ambiguous-write parallel walker.
- Tests: one-sided undefined classic lead, main-loop list payload, property access.

## P1.4 — merged operands match ordinary expression grammar

- `parse_trailing_postfix` gains direct-integer and `at` indexing.
- Shared `parse_merged_operand_from_lead` for write, `content type`, and `headers`.
- Tests: `at` / integer indexing, `content type mime_type of path`.

## P1.5 — full `flush` expression-statement fallback

- Full merged name bound as any value (zero-arg action, non-zero-arg function,
  overloaded without zero-arg, non-callable) → old expression-statement behavior.
- Only unbound names fall through to stream flush.
- Analyzer/typechecker stay aligned.
- Tests: non-callable binding + parameterized action without zero-arg.

## R3 test strength

- Backpressure test asserts error kind + lower timing bound.
- Open-expiry test asserts setup success + lower timing bound.
- `dropped_interpret_server_cleanup_test` covers pending-request 500 without
  streaming head.
