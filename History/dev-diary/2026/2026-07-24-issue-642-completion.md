# Dev Diary — 2026-07-24: PR #641 / issue #642 re-review repair

Issue [#642](https://github.com/WebFirstLanguage/wfl/issues/642) requested a
fresh review of PR #641 from reviewed head
`8e8be0fcde944d0d7b357b94d5951497af5ff0b7`. This pass repaired every newly
confirmed product defect in auditable test-only Red → later Green commits,
replaced false-positive R3 tests with causal tests, and recorded the older
evidence gap without rewriting history.

No merge or PR comment is part of this work. The previously preserved green
Actions run is `30142079511`; it is not final evidence for the repaired
candidate. Final local gates and a new complete Linux/Windows Actions matrix are
required after the documentation and characterization commit.

## Risk, compatibility, and gate status

- **Risk class:** R3.
- **Triggers:** concurrency, cancellation, HTTP lifecycle, streaming, resource
  ownership, bounded retention, async control flow, and backward-compatible WFL
  grammar/typechecking.
- **Compatibility:** no WFL syntax or public `ErrorKind` variant was removed.
  The parser fixes restore ordinary-expression parity and legacy `flush`
  behavior; typechecker fixes accept every runtime-viable classic/streaming
  branch without changing runtime binding rules.
- **External state:** none. Rollback is a source revert; no data migration is
  involved.
- **Policy gate:** unresolved pending maintainer approval of the Section 17
  exception for pre-existing work that lacks retained Red chronology. New
  defects found during this re-review do have valid Red ancestry.

## Implemented behavior

1. Streaming response `status` parses the full clause-aware expression grammar
   without consuming `headers`, `content type`, or `as`.
2. Seeded write/response/flush operands resume postfix composition after `of`
   calls, matching ordinary expressions.
3. Bare `type` is no longer treated as a nonexistent response clause boundary.
4. Same-line unmerged `flush` operands reach stream parsing while genuinely
   bare legacy bindings/actions keep their old meaning.
5. Repeat, try, and count bodies receive checker child scopes; conditional and
   possibly-zero-iteration control flow conservatively joins all runtime-viable
   binding types.
6. Locally opened files are recreated as `Custom("File")` when analyzer scope
   reconstruction leaves no current checker symbol.
7. A final unterminated outbound line is followed by clean EOF even after the
   former absolute deadline.
8. Expired unread streams release the live body, reaper, and handler ownership.
   Typed terminal results use at most 64 lightweight records with a 60-second
   TTL and are consumed by the next read.
9. The complete buffered/streaming response precommit phase—including the
   request operand, actions it calls, all response fields, ownership precheck,
   sender take, and transport commit—observes disconnects as
   `ErrorKind::Cancelled`.
10. Cancellation drops the active future before restoring action/loop state and
    closes only resources opened by that response attempt. Ordinary expression
    failures and duplicate/forged response errors retain their prior behavior.

## Auditable Red → Green ledger

Every Red below is a test-only ancestor of its Green implementation. Fixture
corrections and Green-first characterization commits are listed separately and
are not represented as Red evidence.

| Behavior | Affected base | Test-only Red | Green implementation | Focused command |
|---|---|---|---|---|
| Full streaming status operands | `8e8be0fcde944d0d7b357b94d5951497af5ff0b7` | `09115f88b0ba1bcf8ecbdba3ca81ab62eaa07e40` | `99353201518917b350009822554c7d41f6662582` | `cargo test --test write_web_postfix_test -- --nocapture --test-threads=1` |
| Post-`of` postfix continuation | `f23fb6bc0c3b2b77cf1f9eeab567b38032710f9c` | `d97f15b6d9a05be7f35d54b1bbf3d627472ea7d6` | `764685c081f62123a56cf2bbe11aa2b4617d2711` | `cargo test --test write_web_postfix_test -- --nocapture --test-threads=1` |
| Remove false bare-`type` boundary | `764685c081f62123a56cf2bbe11aa2b4617d2711` | `c8cfa08c0352555bd4d302fd4ded21b827e5ceca` | `485bc34b4daad1354b838a74e59938b5041c0db5` | `cargo test --test write_web_postfix_test -- --nocapture --test-threads=1` |
| Reach unmerged flush targets | `485bc34b4daad1354b838a74e59938b5041c0db5` | `55f3d507c741f44576afce24affbf643ee7d258e` | `4a838459bf985611338e69f81253b2a6eee0e269` | `cargo test --test flush_action_backcompat_test -- --nocapture --test-threads=1` and `cargo test --test http_server_streaming_test -- --nocapture --test-threads=1` |
| Checker child scopes | `4a838459bf985611338e69f81253b2a6eee0e269` | `8b10f8bff36fba1df4e6bae0eda07e9f05c16721` | `a1bdd9d75bd3c8134cb0fb49dc601ff209d6c26f` | `cargo test --test typechecker_response_stream_scope_test -- --nocapture --test-threads=1` |
| Conditional/loop type joins | `7bafc6da8682de19886bb3c47cc14e67c5d2b9e2` | `24f57d63dcd7018a1ea31d1f14c63c2e4069a982` | `046b012e8fd9d79a34ee2032fd2ae36da405816d` | `cargo test --test typechecker_response_stream_join_test -- --nocapture --test-threads=1` |
| Recreate local File symbols | `046b012e8fd9d79a34ee2032fd2ae36da405816d` | `a30fe4f50f8beff3d3b3af67aa234723f6d858fb` | `370073e4431af2e3cbad7273bace3ee0ff307e9d` | `cargo test --test open_file_local_type_test -- --nocapture --test-threads=1` |
| Stable clean EOF after final line | `0e98fe35415abe1e067293edfaa47a4509446303` | `5bef23578d0c315dd12b5613e63f6c9192d4e79a` | `af800a7dfe44a6188b591aebfd4f8211d51719e8` | `cargo test --lib interpreter::outbound_stream_deadline_tests::final_unterminated_line_survives_deadline_after_clean_eof -- --nocapture --test-threads=1` |
| Bounded expired-stream state | `af800a7dfe44a6188b591aebfd4f8211d51719e8` | `5d8fa3d6775145f8f63a4684f365f5f2e95c55c4` | `c7f57b9594a7286d57692efa066502fd6c08c16e` | `cargo test --lib interpreter::outbound_stream_deadline_tests::unread_expired_stream_metadata_and_ownership_are_bounded -- --nocapture --test-threads=1` |
| Cancel buffered content and streaming-head evaluation | `00a2a3fa5f60bf414ac211b2c76d546f784b0d49` | `4c45f1617097cbd39f92183bbe9dcbd986cea41d` | `0d4b26b23bcd356bd62fc4de6abf89e062e0279c` | `cargo test --lib interpreter::response_expression_disconnect_tests -- --nocapture --test-threads=1` |
| Cancel request operands; clean precheck and commit races | `edb8ce89c3d693015f655daac012c73cbc12d293` | `3bc38c668a91229e213a10d8eaebdba3789556a9` | `c73260ff61a32694c5ecfe72ab8749810033de0d` | `cargo test --lib interpreter::response_expression_disconnect_tests -- --nocapture --test-threads=1` and `cargo test --lib interpreter::response_disconnect_result_tests -- --nocapture --test-threads=1` |

The intended Red failures included incomplete/misbounded ASTs, unreachable flush
forms, leaked checker types, unknown local File types, stale-deadline Timeout,
unbounded live stream/owner populations, response evaluation that remained
pending after its client disconnected, stale pending-response ownership, and
upstream streams retained after commit-time cancellation.

Post-Green fixture corrections were
`f23fb6bc0c3b2b77cf1f9eeab567b38032710f9c`,
`7bafc6da8682de19886bb3c47cc14e67c5d2b9e2`,
`f0dc05db5b2c6722a3a400e629544295a3b07609`, and
`85768c2384b2e414fe66c26751b28f12f2614890`. They correct or broaden
test fixtures; none is claimed as a new Red.

## R3 characterization and preservation evidence

- `0e98fe35415abe1e067293edfaa47a4509446303` proves the classic write fallback
  with a real opened File handle.
- `90a225d4de36fc74a0b17e4312889c2fa3511c93` makes simultaneous body/expiry
  arbitration deterministic.
- `edb8ce89c3d693015f655daac012c73cbc12d293` replaces timing-only lifecycle
  coverage with active-read close, spawned-reaper, exact disconnect
  classification, zero/fractional timeout, backpressure, and real client
  disconnect tests. The real TCP test covers buffered content plus streaming
  status/content-type/headers, asserts upstream EOF, and proves `/ping`
  remains serviceable.
- `tests/concurrent_disconnect_paths_burst_test.rs` uses causal release markers
  and iteration barriers. Each 256-client wave is fully consumed before the
  next wave or `/ping`; fixed handler sleeps are not used as proof.
- Green-first breadth checks cover builtin status operands, ordinary/seeded AST
  parity and runtime behavior after `of`, a genuinely bare non-callable
  `flush` binding, outer Text/File scope reconstruction, and ordinary
  expression/error preservation.

Focused preservation commands run on the repaired tree include:

```text
cargo test --lib interpreter::response_expression_disconnect_tests -- --nocapture --test-threads=1
cargo test --lib interpreter::response_disconnect_result_tests -- --nocapture --test-threads=1
cargo test --lib interpreter::request_wait_timeout_tests -- --nocapture --test-threads=1
cargo test --lib interpreter::outbound_stream_deadline_tests -- --nocapture --test-threads=1
cargo test --test response_expression_disconnect_runtime_test -- --nocapture --test-threads=1
cargo test --test concurrent_disconnect_paths_burst_test -- --nocapture --test-threads=1
cargo clippy --lib -- -D warnings
```

All completed focused commands passed without retry, skip, quarantine,
weakened assertions, or replacement with timing-only assertions.

## Historical evidence gap

The original PR work before reviewed head `8e8be0fc` does not have retained
test-only Red ancestors for every behavioral change. Actions run `30106107011`
is the only located durable pre-Green Red artifact for that earlier work.
Writing passing tests now, reverting finished code, or rewriting commit history
would not establish the missing chronology.

The repository therefore contains a narrowly scoped Section 17 exception draft
under `Docs/development/testing-policy-exceptions/`. It records the exact
missing rule/scope, reason, compensating verification, residual risk,
containment, rollback, owner, repair deadline, and seven-day R3 expiry. It is
explicitly **PENDING MAINTAINER APPROVAL**. Until approved, the testing-policy
merge/release gate remains unresolved; the exception does not turn missing
evidence into a pass.

## Required final verification

The final candidate must run these exact commands after all code, tests, and
documentation are committed:

```text
cargo fmt --all -- --check
git diff --check
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
cargo test --all --verbose --jobs 2
scripts/run_integration_tests.sh
python3 scripts/validate_docs_examples.py --ci --force
scripts/run_web_tests.sh
```

After push, the new GitHub Actions run must finish successfully across Linux
and Windows, including the integration gate, TestPrograms, docs validation, web
tests, TLS, PostgreSQL, MariaDB, and fuzz-target compilation. Those results
belong in the final handoff rather than being preclaimed here.

## Residual risk and recovery

- Recent typed stream terminals are deliberately bounded to 64 records and 60
  seconds. A much later read, or a read after capacity eviction, receives the
  documented unknown/closed-handle result rather than retaining metadata
  indefinitely.
- A response request operand is raced against the stable set of requests owned
  when it begins; normal handlers own one request. Newly accepted resources are
  treated as work created by the attempt and are cleaned if it is cancelled.
- The Section 17 approval is an explicit unresolved governance risk, not a
  product-test failure.
- No deployment or persistent state changed. A rollback returns the complete
  affected change set to its recorded base and reruns the gate; it must not
  claim the reverted behavior remains repaired. Forward repair is preferred
  for any later race or platform defect.
