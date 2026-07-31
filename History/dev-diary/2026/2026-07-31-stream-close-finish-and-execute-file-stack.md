# 2026-07-31 — Stream close finish wait (#680) and execute-file stack (#681)

## Why

Two intermittent CI failures, both filed after they proved not job-specific and
not caused by in-flight feature work:

- **#680** — `test_early_chunk_is_visible_before_the_body_completes` failed with
  `error decoding response body` in both `Build, Test, Clippy` and
  `Integration Tests`, after also passing in those same jobs. Same commit, both
  outcomes → teardown race under load.
- **#681** — Windows integration jobs aborted with `STATUS_STACK_OVERFLOW`. The
  binary name varied until a later run named
  `test_execute_file_depth_limit_prevents_infinite_recursion`. Four nested
  `execute file` levels (the default `max_execute_file_depth`) exhaust a ~1 MB
  Windows test-thread stack *before* the depth guard can fire.

## #680 — close out must mean the body finished

`close out` (and end-of-handler auto-close) only dropped the body-chunk sender.
That signals end-of-stream to hyper; it does not wait for the terminating chunk
to leave the process. When the program then returns and the host drops the
tokio `Runtime`, the warp connection task can be aborted mid-flush — the client
sees a truncated chunked body.

Fix:

1. Pair each streaming body with a oneshot fired from
   `NotifyingChunkStream::Drop` (hyper drops the body when it is done writing
   or when the client disconnects).
2. `close out` and async end-of-handler / end-of-run drains drop the sender,
   then await that signal (bounded by the response-write timeout, or 30s when
   the timeout is disabled).
3. Sync `Drop` paths (`IsolatedHandler`, `OutboundStreamCleanup`) still only
   drop senders — they cannot await; serial programs that finish right after
   `close out` take the async path.

Regression: `test_body_is_complete_when_program_ends_immediately_after_close`
streams one chunk, closes, breaks, and asserts the client reads a complete body
with no transport decode error.

## #681 — depth guard must win over the native stack

The CLI already runs on `wfl::run_with_interpreter_stack` (1 GiB reserved). The
overflow was the **test harness**: default Windows test threads are ~1 MB, and
nested `execute file` multiplies poll frames of large interpreter futures.

Fix:

1. Move the `execute file` pipeline into its own `execute_wfl_file` async method
   and only `Box::pin` it from `_execute_statement`, so the full
   lex→parse→analyze→interpret locals are not part of the shared statement
   state machine (smaller futures for every other statement; less stack cost
   per nested execute-file level).
2. Run the depth-guard test under `run_with_interpreter_stack` so it asserts the
   promised depth error, not a native overflow.
3. Default `RUST_MIN_STACK=8388608` for Windows in
   `scripts/run_integration_tests.ps1` and the Windows Integration Tests CI
   job, matching a typical Linux default and leaving headroom for other deep
   futures in the suite.

Embedders that drive `Interpreter` without a large stack remain at risk for
deep recursion of any kind; the existing `run_with_interpreter_stack` docs and
the configuration note for `max_execute_file_depth` call that out.

## Concurrent path follow-up

A review found the same race on `main loop concurrently:`: stream ids live on
the handler's isolated `RunState`, so the top-level serial drain never sees them.
`IsolatedHandler` Drop only nowait-closed. Fix: after the handler future
completes, take any still-open response-stream ids and await
`close_response_streams` **before** reporting `Ready` (Break/Exit/Return). Drop
still nowait-closes on cancellation mid-flight.

Regression:
`test_concurrent_handler_body_is_complete_without_explicit_close_on_break`.

## Finish-wait ceiling (not the write timeout)

An early version of the close-await used `web_server_response_timeout_seconds`
(default 300s, or 30s when `0`) as the finish ceiling. That fixed the
truncation race but let a client that stops reading pin a serial `main loop`
at `close out` / end-of-iteration drain for minutes — and the docs still said
close returned immediately / that `0` disabled all waiting.

Fix: finish wait is a short fixed `RESPONSE_STREAM_FINISH_WAIT` (2s),
independent of the write-path timeout. Docs for streaming close and
`web_server_response_timeout_seconds` now describe that split accurately.

## Verification

- `cargo test --test response_stream_backpressure_test --test execute_file_test`
  — all green, including the new #680 regression and the rewritten depth guard.
