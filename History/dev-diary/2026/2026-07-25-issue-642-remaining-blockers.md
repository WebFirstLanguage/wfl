# Issue #642 — remaining lifecycle, concurrency, and compatibility blockers

**Date:** 2026-07-25
**Scope:** the items left open by the #642 re-review of the merged #641 head
(`690be0af`, merged as `62a1b30`): four release-blockers for concurrent web
serving plus the follow-up correctness/compatibility list.

Every behavioral change below landed as a Red test-only commit (asserting the
defect at base `62a1b30`) followed by its Green fix, per the root testing
policy (§6.2 evidence path 1). Red/Green pairs are noted inline.

> Note: the branch history was rewritten once before push (committer-identity
> fix only; file contents unchanged, Red→Green ordering preserved). The hashes
> in THIS document are the pushed, post-rewrite hashes — commit messages that
> say "red at <hash>" still carry the pre-rewrite value; map them through the
> pairs listed here.

## Release blockers (concurrent web serving)

### 1. `execute file ... and read output` capture is now handler-local
The io_capture stack is a thread-local; interleaved concurrent handlers shared
it, so a handler awaiting mid-capture absorbed sibling `display` output and
lost its own lines to a sibling's buffer, with LIFO guard pops mismatching on
out-of-order completion. The capture stack is now part of the per-handler
`RunState` swap (handlers inherit a clone of the ambient stack, so output
still reaches an enclosing capture), and `CaptureGuard` removes its buffer by
`Rc` identity instead of popping blindly.
Red `91ffee1` → Green `952e270` (`tests/concurrent_execute_capture_test.rs`).

### 2. `wait for request` timeout expressions no longer stall siblings
The timeout clause is an arbitrary WFL expression and was evaluated while
holding the receiver mutex shared by every handler — one slow expression
parked the whole server. It is now evaluated before locking.
Red `01ee863` → Green `4556a65` (`tests/concurrent_timeout_eval_lock_test.rs`).

### 3. Module/include loading context is now handler-local
`current_source_file` and `loading_stack` were interpreter-global: concurrent
loads falsely reported circular dependencies, resolved relative paths against
a sibling's module directory, inflated import depth, and guards popped each
other's entries. Both moved into the `RunState` swap (initialized from the
enclosing context, so true cycles through it are still detected), and
`ModuleLoadGuard` restores/removes by identity.
Red `822778d` → Green `cfd3440` (`tests/concurrent_module_loading_test.rs`).

### 4. Response streams are handler-owned
`write`/`flush`/`close` acted on the global stream map with no ownership
check, so a handler holding a sibling's handle (via a shared global) could
inject bytes into, flush, or truncate the sibling's response — and because the
write path clones the channel sender before awaiting, a sibling's `close`
didn't even stop an in-flight write. The three verbs now require the handle to
be in the current handler's open-stream list; a live stream owned by another
handler reports ownership, a missing one keeps the closed-stream error, and
re-closing one's own closed stream stays a no-op. With ownership enforced,
write and close can no longer race across handlers (a handler is sequential
within itself). Docs: web-servers.md "Ownership" note.
Red `3ba71e7` → Green `1f71935` (`tests/concurrent_stream_ownership_test.rs`).

## Follow-up correctness/compatibility

### 5. Concurrent handlers preserve live recursion depth
Handlers seeded `call_depth` from `base_call_depth` (run entry) instead of the
live depth at loop entry, so a loop nested in user actions granted every
handler a full `max_call_depth` on top of uncounted real frames — an early
variant of the regression test aborted with a genuine native stack overflow on
a 2 MB thread. Handlers now seed from the live depth (mirroring
`execute file`'s child seeding). `IsolatedHandler::drop` also drops the
handler future with the handler's own run state swapped in, so RAII guards in
a future dropped mid-suspend (call-depth, capture, module-load) unwind against
their own state rather than the ambient context.
Red `bd183ad` → Green `0320d24` (`tests/concurrent_recursion_depth_test.rs`).

### 6. Classic `write line|chunk` continuations for `starts`, `ends`, `:`
The bare-marker guard enumerated continuation tokens by hand and was missing
`KeywordStarts`, `KeywordEnds`, and `Colon` relative to
`parse_binary_continuation_inner`, so `write line starts with "/" to f`
silently mis-parsed into the streaming branch and `write line : to f`
parse-failed. The guard now includes them and documents the binary parser as
the authoritative set.
Red `dff4f3b` → Green `c4f74bc` (`tests/write_web_postfix_test.rs`).

### 7. Exact `flush (…)` / `flush call …` legacy side effects
Pre-streaming these were two statements — the bare `flush` expression
statement plus the operand expression statement. The fallback evaluated only
`Variable("flush")`, dropping the operand's side effects. Runtime now executes
both in the original order when the bare `flush` binding exists (exact form
only; merged forms' fallback already spans the operand), and the analyzer and
typechecker validate the operand the same way.
Red `cd7bef4` → Green `daca8c7` (`tests/flush_action_backcompat_test.rs`).

### 8. Typechecker alignment: repeat-until order/backedge, WebSocket binding
- `RepeatUntilLoop` checked the condition before the body (runtime runs the
  body first, in the same scope) with no backedge fixed point. It now checks
  the body under the same fixed point as `while`, then the condition against
  the POST-body state (`check_loop_body_fixed_point_post_body` — repeat-until
  always runs the body before the condition, so the header-state restore used
  for `while` would hide body retypings).
- `WebSocketHandlerStatement` pushed a checker scope but never defined the
  binding symbol, so handler bodies were checked against an outer same-named
  symbol's concrete type (false errors on runtime-valid programs). The binding
  is now recreated (Type::Unknown) in the scope, shadowing like runtime
  `define_direct`.
Red `f2e28d9` → Green `59a71cd`
(`tests/typechecker_repeat_until_backedge_test.rs`,
`tests/typechecker_websocket_binding_scope_test.rs`).

### Deferred: try/finally error-alias scope
The re-review also lists "align try/finally error-alias scope". At runtime the
error alias (`error_message` and the `when` clause name) is defined in the
shared try child-env and stays live through `finally`; statically both the
analyzer and typechecker deliberately exclude the alias from promotion so
`finally` resolves the outer binding — and that static contract is locked in
by `tests/typechecker_try_finally_join_test.rs` (`:112`, `:211`). Aligning
either direction is a breaking decision: making the runtime clause-local
breaks programs that read `error_message` in `finally`; making the static
passes runtime-faithful reverses deliberate, tested behavior from #641. Left
for a maintainer ruling rather than changed unilaterally here.

## Not changed
- The consecutive-failure breaker threshold is still a private const
  (`MAX_CONSECUTIVE_HANDLER_FAILURES = 256`); the classifier fixes from the
  original #642 body were already on the merged head.
- The serial `main loop`, plain `write`/`flush` forms, and all pre-existing
  `TestPrograms/` behavior are unchanged (backward-compat gate run against the
  release build).
