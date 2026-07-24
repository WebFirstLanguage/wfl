# Dev Diary — 2026-07-24: issue #642 re-review fixes

Follow-up to the maintainer checklist on the #642 round.

## CI

- `cargo fmt --all` unblocked the formatting gate (commit `a2620362`).

## Runtime lifecycle

| Item | Fix |
|------|-----|
| Close during active read | `StreamCancel` (AtomicBool + Notify); reads select against it; close/reaper cancel + remove slot |
| Map cleanup from Drop | `std::sync::Mutex` (no silent `try_lock` abandon) |
| Reaper before insert | Insert slot then arm reaper under same lock |
| Tombstones | Finish always **removes** the slot |
| Final unterminated line | Finish after emitting (no parked done+reaper) |
| Respond/stream eval disconnect | `ensure_pending_response_owned` then evaluate; take sender only at commit |
| Stall vs disconnect | Stall write → `ErrorKind::Timeout`; disconnect → `Cancelled` |
| Breaker Timeout exemption | Only messages starting with `Timeout waiting for request` |
| Fractional wait timeout | Reject `0 < ms < 1` |

## Parser / typechecker

| Item | Fix |
|------|-----|
| Classic write to `open file` | Accept `Custom("File")` as classic-file target |
| content type / headers clauses | `parse_clause_operand_from_lead` stops at clause connectives |
| flush postfix legacy | Full expression AST fallback on phrase + postfix |
| Parameterized flush | ExpressionStatement semantics → arity error |
| ResponseStream binding | `define_or_replace` in current scope only (shadow) |
| Write definedness | Analyzer `name_is_defined_for_write` shared with typechecker |

## Red→Green note

The original #642 landing was a single mixed commit (`5e01e446`, +1399/−271).
Rewriting that history on the already-pushed branch would require a force-push;
this re-review is a **new** Green commit on top with targeted regressions.
Auditable Red-first history for a future rework can soft-reset and re-land as
test-only then fix commits if the maintainer prefers a rewritten PR stack.

## Tests run (local)

```
cargo test --test flush_action_backcompat_test --test write_web_postfix_test \
  --test ambiguous_write_branch_typecheck_test --test concurrent_disconnect_paths_burst_test \
  --test outbound_stream_reaper_race_test --test response_stream_backpressure_test \
  --test outbound_stream_close_during_read_test
```
All green.
