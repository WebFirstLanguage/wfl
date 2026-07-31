# Dev Diary — 2026-07-25: Stream terminal and type-flow review fixes

This pass resolves four R3 findings found while reviewing the PR #641 / issue
#642 candidate: clean EOF retained a live outbound stream indefinitely,
`try` handler state did not reach `finally` in the checker, loop backedges were
not rechecked under later-iteration types, and deferred event/WebSocket handler
types leaked into the registration scope.

The affected repair base is
`c9c748ce850b7d106ffef90e299e4b7221411517`. The final executable candidate is
`de34e32e513d7d73634b0d7308c681930953a4db`. The latest executable-test
descendant is `81b25745e757671538a833ddf7bc837e19ad83c7`; it adds deterministic
test-only coverage and does not change production code. The documentation-only
commit containing this entry does not change either executable identity.

## Risk, contracts, and compatibility

- **Risk class:** R3.
- **Triggers:** streaming lifecycle, cancellation and reaper races, bounded
  resource retention, control-flow joins, loop iteration, deferred callbacks,
  and backward-compatible typechecking.
- **Lifecycle contract:** observing upstream EOF immediately removes the live
  slot, body, owner, and reaper. Only one lightweight `CleanEof` result remains,
  sharing the existing recent-terminal bound of 64 records and 60 seconds.
- **Type-flow contract:** `try` success, handler, `otherwise`, and unmatched
  endpoints are conservatively joined before `finally`; error aliases remain
  clause-local. Loop bodies are checked at a conservative header fixed point.
  Deferred event and WebSocket bodies are checked in child scopes.
- **Compatibility:** no syntax, public error variant, or runtime binding rule
  was removed. Runtime-viable gradual joins stay permissive, while concrete
  invalid later-iteration branches now produce diagnostics.
- **External state:** none. There is no deployment, schema, data migration,
  secret, or external-service mutation.

## Acceptance criteria and exact regressions

| Acceptance criterion | Exact regression tests |
|---|---|
| A final unterminated line releases all heavy stream state immediately, retains one bounded EOF result, and remains first-wins against later timeout/close cleanup | `final_unterminated_line_survives_deadline_after_clean_eof`; `unconsumed_clean_eof_records_are_bounded`; `observed_clean_eof_wins_over_a_later_deadline_claim`; `put_stream_restores_clean_eof_after_close_removed_the_live_slot`; `put_stream_deduplicates_clean_eof_after_reaper_removed_the_live_slot` |
| Handler and `otherwise` endpoint types reach `finally`, while temporary error aliases do not | `handler_response_stream_state_is_joined_before_finally`; `handler_error_aliases_remain_clause_local_before_finally`; `handler_created_binding_is_semantically_visible_in_finally`; `otherwise_created_binding_is_semantically_visible_in_finally`; `full_pipeline_error_alias_is_clause_local` |
| Later loop iterations are checked after a body changes a target from File/Text-compatible state to `ResponseStream` | `while_loop_rechecks_stream_lead_after_tail_response_stream_rebind`; `repeat_while_loop_rechecks_stream_lead_after_tail_response_stream_rebind` |
| Merely registering a deferred handler cannot overwrite the enclosing checker binding | `event_handler_body_types_do_not_leak_after_registration`; `websocket_handler_body_types_do_not_leak_after_registration` |

The regressions live in `src/interpreter/mod.rs`'s unit-test module,
`tests/typechecker_response_stream_join_test.rs`,
`tests/typechecker_response_stream_scope_test.rs`, and
`tests/typechecker_try_finally_join_test.rs`.

## Auditable Red → Green ledger

Each Red commit below contains tests only and is an ancestor of its Green
implementation.

| Repair | Red parent / affected base | Test-only Red | Green implementation | Intended Red observation |
|---|---|---|---|---|
| Immediate bounded clean-EOF terminalization | `c9c748ce850b7d106ffef90e299e4b7221411517` | `96d53052388f75bd809c2af42f12445944e8fc69` | `b32ff55fa76fd03b07e2ade7159d3719f2ac0642` | After the final line, the live-slot assertion observed one retained slot instead of zero. |
| Loop-header fixed points, joined `try` endpoints, and deferred-handler isolation | `96d53052388f75bd809c2af42f12445944e8fc69` | `68569b31b9fd969cb5adc3b8c0832ec604bb98e2` | `527b8fb184245e7df35fe5229b23e2a969c74520` | Later-iteration invalid branches were missed, valid `finally` cleanup was rejected, and deferred handler registration changed outer types. |
| First-wins EOF and analyzer/runtime `try`-scope parity | `527b8fb184245e7df35fe5229b23e2a969c74520` | `03966f06e78aec7c3bcdbd40feabc2bdff37a16d` | `de34e32e513d7d73634b0d7308c681930953a4db` | A later timeout displaced observed EOF; handler/`otherwise` bindings were absent in analyzer `finally`; aliases collided in the full pipeline. |

Commit `81b25745e757671538a833ddf7bc837e19ad83c7` broadens the Green
evidence with deterministic close/reaper missing-slot tests. Those tests use
the real close helper and the reaper's critical-section behavior, then assert
zero live slots and owners, exactly one `CleanEof`, one `nothing` result, and a
subsequent closed/unknown-handle error.

## Implementation

`StreamTerminal::CleanEof` now uses the same bounded, expiring, one-shot recent
terminal queue as timeout records. Upstream EOF is the first-wins
linearization point. Returning the final buffered line no longer parks the
completed handle or clears its deadline; `put_stream` drops heavy state,
removes ownership, aborts the reaper, and deduplicates/restores the one
lightweight EOF record even if close or reaper cleanup already removed the
slot.

The analyzer and typechecker now model the runtime's shared `try` child
environment. Ordinary endpoint bindings are promoted and joined before
`finally`, while the named error and `error_message` aliases are discarded
with their clause scope. `while` and `repeat while` widen entry and backedge
snapshots to a stable conservative header before their diagnostic pass.
Event and WebSocket callback bodies are checked under isolated child scopes,
matching runtime dispatch.

## Focused and boundary verification

The following completed without product-test retry, quarantine, assertion
weakening, or timing-only substitution:

```text
cargo test --lib put_stream_ --jobs 1 -- --nocapture --test-threads=1
cargo test --lib observed_clean_eof_wins_over_a_later_deadline_claim --jobs 1 -- --nocapture --test-threads=1
cargo test --lib clean_eof --jobs 1 -- --nocapture --test-threads=1
cargo test --lib interpreter::outbound_stream_deadline_tests --jobs 1 -- --nocapture --test-threads=1
cargo test --test typechecker_try_finally_join_test --jobs 1 -- --nocapture
cargo test --test typechecker_response_stream_join_test --jobs 1 -- --nocapture
cargo test --test typechecker_response_stream_scope_test --jobs 1 -- --nocapture
cargo test --test nothing_reassign_widen_test --jobs 1 -- --nocapture
cargo test --test overload_alias_resolution_test --jobs 1 -- --nocapture
cargo test --test http_stream_test --jobs 1 -- --nocapture --test-threads=1
cargo test --test stream_backpressure_test --jobs 1 -- --nocapture --test-threads=1
cargo test --test open_file_local_type_test --jobs 1 -- --nocapture
```

The respective focused results were 2, 1, 3, 10, 5, 4, 5, 7, 1, 12, 2,
and 3 tests passed with zero failures.

The official Windows integration runner completed all Rust integration targets
and then reported **110 WFL programs passed, 0 failed, 24 documented skips**.
The official web runner reported **2/2 passed**; its separate certificate-file
journey was explicitly skipped because OpenSSL is unavailable on this host.
The Rust integration suite's eight TLS server tests passed. Forced docs-example
validation reported **18 passed, 0 failed** across validation layers 1–5.
The validator also emitted existing manifest-schema-key warnings; they did not
represent failed examples.

## Infrastructure interruption and test integrity

The first `cargo test --all` invocation terminated during rustc compilation
while memory-mapping an rlib with Windows error 1455: the paging file was too
small. No test binary produced a product-test result. The complete unchanged
suite was then invoked with `cargo test --all --jobs 1`, altering only Cargo's
compiler parallelism to bound peak memory. This is an infrastructure rerun
under `testing.md` §8.2, not a product-test retry; no product failure was
retried.

The initial script invocations inside the restricted process sandbox did not
run product tests: Windows execution policy blocked one integration-script
launch, and a later sandbox process launch could not access Cargo's artifact
database. The same repository scripts were then run once successfully through
PowerShell with `-ExecutionPolicy Bypass` outside that boundary. Existing
ignored Rust tests and the integration runner's documented WFL skips were not
introduced or changed by this repair.

## Final local gate record

The final local gate ran on 2026-07-25 against executable candidate
`de34e32e513d7d73634b0d7308c681930953a4db`, test-only descendant
`81b25745e757671538a833ddf7bc837e19ad83c7`, and the final documentation
working tree. The host reported Windows NT `10.0.26200.0`, rustc/cargo
`1.97.0`, and PowerShell `7.6.4`. The official `.ps1` runners executed under
Windows PowerShell `5.1.26100.8875`.

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --all-targets --all-features --jobs 1 -- -D warnings` | Passed. |
| `cargo test --all --jobs 1` | Passed across the complete workspace, integration binaries, LSP, `wflpkg`, and doctests. Existing ignored tests remained reported. Pre-existing unused-code warnings were emitted only by `wfl-lsp` test fixtures; the strict Clippy gate passed. |
| `cargo build --release --jobs 1` | Passed. |
| `git diff --check` | Passed; Git emitted only the repository's Windows LF-to-CRLF working-copy notices for two Markdown files. |
| `$env:CARGO_BUILD_JOBS='1'; ... run_integration_tests.ps1 -TestOnly` | Passed all Rust integration targets and 110 WFL programs; 0 failed and 24 documented programs were skipped by the runner. |
| `... run_web_tests.ps1` | Passed 2/2 runnable journeys; the OpenSSL-dependent certificate-file journey was visibly skipped on this host. |
| `python scripts/validate_docs_examples.py --ci --force` | Passed 18/18 examples with 0 failures. |

No changed-code warning, product-test failure, retry, quarantine, mute, or
weakened assertion was used to obtain this result. Exact-candidate CI still
must exercise the repository's required platform and service matrix, including
the TLS journey that the local web script could not generate without OpenSSL.

## Coverage, review, and residual risk

Coverage was not instrumented, so there is no numeric result. Root
`testing.md` records the absent automated coverage gate as a tracked
conformance gap; no percentage or threshold pass is claimed. This change adds
behavioral, real-boundary, negative, race, and higher-layer regression
coverage.

An independent Codex review inspected `c9c748ce..de34e32e` for lifecycle and
concurrency correctness, type-flow soundness, compatibility, and test
integrity. Its three Important findings were deterministic close/reaper
missing-slot coverage, explicit 64-record/60-second clean-EOF documentation,
and current candidate chronology. Commit `81b25745` and the accompanying
documentation resolve them. Follow-up review reported no remaining Critical or
Important issue. This is review evidence, not maintainer or security-owner
approval.

Recent `CleanEof` results intentionally expire after 60 seconds and share a
64-record cap with other terminal results. After expiry, eviction, or
consumption, another read reports an unknown/already-closed handle. Type joins
conservatively widen missing or path-disagreeing bindings to the project's
gradual `Any`/`Unknown` behavior. These are documented design choices.

PR merge and release remain blocked until the exception record has the required
project/reliability and security-owner approvals, requester identity/date,
maximum-release and expiration acknowledgment, and a successful GitHub Actions
matrix on the final integrated PR head. That head must include the
`81b25745` evidence tests and this documentation; a run on production identity
`de34e32e` alone is insufficient. The older Actions run `30142079511` is Green
evidence for an earlier head, not the final candidate.

## Rollback

No external recovery is needed. Revert the complete post-base range beginning
after `c9c748ce850b7d106ffef90e299e4b7221411517`, including its test-only
commits, so intentionally failing Red tests are not left active. Preserve the
original Red and Green commits in repository history as evidence, then run the
complete gate on the rollback candidate. Prefer a forward repair if later work
depends on these compatibility or lifecycle corrections.

Related durable records are
`Dev diary/2026-07-24-issue-642-completion.md`,
`Docs/development/response-streaming-design.md`,
`Docs/development/testing-policy-exceptions/2026-07-24-pr-641-red-chronology.md`,
the implementation plan under `Docs/superpowers/plans/`, and root
`testing.md`.
