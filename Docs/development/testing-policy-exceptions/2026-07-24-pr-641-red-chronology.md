# WFL testing-policy exception draft: PR #641 Red chronology

> **STATUS: DRAFT — PENDING MAINTAINER APPROVAL**
>
> This exception is not active. It does not turn missing Red evidence into a
> pass. PR #641's merge and release gates remain unresolved until every approval
> condition and signature below is complete. Silence, a green test run, or merge
> authority alone is not approval.

## Exception record

| Field | Value |
|---|---|
| Exception ID | `WFL-TEST-EXC-2026-07-24-PR641` |
| Repository | `WebFirstLanguage/wfl` |
| Change record | [PR #641](https://github.com/WebFirstLanguage/wfl/pull/641) |
| Repair ticket | [Issue #642](https://github.com/WebFirstLanguage/wfl/issues/642), item 11 |
| Risk class | **R3** — concurrency, cancellation, lifecycle, streaming, backward compatibility, untrusted archive input, and release-test controls |
| Requested start | `2026-07-24T00:00:00-05:00` (`America/Chicago`) |
| Expiration | `2026-07-31T00:00:00-05:00` (`America/Chicago`), exactly seven days after the requested start |
| Maximum affected releases | **One** WFL release: the first release containing the approved candidate, and no later release |
| Affected base | `b25aed57ea50697c596796446d1f47466668773d` |
| Commits containing the earlier mixed Green work | `5e01e446ab9250d72a0f255bc81a27a79c5b5d63`, `fce5d86fe923666885e40ec484d902cfd18c4c85`, and `8e8be0fcde944d0d7b357b94d5951497af5ff0b7` |
| Exact executable candidate for this draft | `c73260ff61a32694c5ecfe72ab8749810033de0d` |
| Requested project/reliability owner approval | Brad, Maintainer, Logbie LLC — **PENDING** |
| Requested security-owner approval | Brad, Maintainer, Logbie LLC — **PENDING**, required for the archive-path item |

The exact executable candidate above is the latest production-code commit
covered by this draft. Evidence-only test or documentation descendants do not
expand the affected production scope. Any later executable change, force-push,
or different candidate invalidates this draft until the SHA, scope, evidence,
and approvals are updated and reviewed again.

If approval occurs after the requested start, the exception becomes active only
at the recorded approval time and still expires at the fixed expiration above.
It never applies retroactively to authorize an earlier merge or release.

## Exact rule and affected scope

This draft requests a temporary exception only from the retained chronology
requirements in root `testing.md`:

- Section 6.1, which requires the Red step before the Green implementation;
- Section 6.2, which requires a test-only Red ancestor or independently
  timestamped pre-Green artifact tied to the affected base and requires the
  base, Red, and Green identifiers; and
- Section 15, only the requirement to attach retained Red evidence for each
  behavior or defect fix.

It does **not** waive the R3 classification, any required test layer, Section
11.3 concurrency/lifecycle coverage, independent review, a required CI job, a
known product failure, or any non-waivable condition in Section 14. A required
test may not be skipped, retried to manufacture Green, quarantined, muted,
weakened, or relabeled under this record.

The missing chronology is limited to the following behavior that remains in the
candidate from the earlier mixed implementation commits:

| Area | Exact behavior lacking retained pre-Green Red chronology | Present regression/verification surface |
|---|---|---|
| Concurrent request containment | Once a handler has accepted a request, request-local `Cancelled`, finite request-wait timeout, response-send failure, and other post-accept failures do not feed the global structural-failure breaker; an owned pending request removed by sibling pruning is cancellation, while an unowned duplicate response remains an ordinary error; unrelated `/ping` work stays serviceable after disconnect bursts. | `concurrent_disconnect_paths_burst_test`, concurrent-handler classifier units, and finite request-timeout units |
| Pending-response and dropped-run cleanup | Dropping a handler/interpreter future closes its owned pending request and server response streams, emits the documented dropped-request response where applicable, and does not leave work attached to a reused interpreter. | `dropped_interpret_server_cleanup_test` and interpreter cleanup units |
| Response-stream backpressure | A connected client that stops reading cannot park a response-stream write indefinitely; timeout and disconnect remain distinct typed outcomes, and unrelated handlers continue. | `response_stream_backpressure_test` and response-disconnect classifier units |
| Outbound-stream deadline and close lifecycle | The configured absolute lifetime includes response-head time, bounds active and unread streams, closes the upstream socket, wakes an active reader with the typed terminal result, gives expiry priority on a ready/expiry tie, prevents reinsertion after terminal state, aborts reapers on terminal paths, and safely clamps extreme positive durations. | `outbound_stream_deadline_test`, `outbound_stream_open_expiry_test`, `outbound_stream_reaper_race_test`, and deterministic active-read close/expiry units |
| Ambiguous classic/streaming writes | Type checking selects the runtime-viable classic file or response-stream branch for concrete targets and checks both branches for gradual targets; container/property context and the analyzer's shared continuations do not hide undefined names or reject the inactive reading. | `ambiguous_write_branch_typecheck_test`, `ambiguous_write_analyzer_test`, `stream_handle_type_test`, and focused analyzer/typechecker units |
| Merged write and response operands | Existing write, response content-type, and response-header operands retain ordinary expression composition for property/index/`at` access, concatenation, nested calls, builtins, unary forms, explicit call arguments, and clause boundaries. This row excludes the later status-operand and post-`of` fixes listed below. | `write_web_postfix_test` and parser AST units |
| Legacy merged `flush` compatibility | A previously valid action, overload, non-callable binding, split/find/replace binding, postfix target, or full fallback expression beginning with merged `flush` retains expression-statement behavior; analyzer, unused-variable analysis, typechecker, and runtime use the same preserved legacy binding metadata. This row excludes the later unmerged-target dispatch and post-`of` fixes listed below. | `flush_action_backcompat_test`, `write_web_postfix_test`, and static-analyzer units |
| Fractional request waits | A positive request timeout below one millisecond is rejected deterministically instead of being rounded into the distinct zero/unlimited behavior. | request-wait timeout units |
| Portable archive containment | A rooted archive entry such as `/etc/shadow` is rejected as rooted on Windows as well as Unix before extraction and cannot escape the destination. | `wflpkg` security tests, including archive traversal/rooted-path cases |
| Gate and fixture correctness | The official Windows integration runner handles equivalent `Path`/`PATH` entries without hiding conflicting values and retains the child process exit status; the Windows web runner fails on cleanup failures; subprocess, free-port, and directory-performance fixtures exercise repository-owned bounded resources instead of shell-only commands, fixed ports, or the repository tree. | Official Linux/Windows integration and web scripts, `execute_file_test`, `file_io_performance_test`, and `subprocess_comprehensive.wfl` |

The following repairs are explicitly **outside** this exception because the
repair branch contains genuine test-only Red ancestors followed by Green
implementation commits:

- complete status-clause operands;
- postfix continuation after `of`;
- removal of the nonexistent bare `type` response boundary;
- unmerged streaming `flush` dispatch;
- ResponseStream child scopes and conservative branch/loop joins;
- local opened-File symbol recreation;
- clean EOF after a final unterminated line;
- bounded expired-stream terminal metadata;
- response-expression disconnect cancellation, including request operands,
  early prechecks, and commit-time cleanup.

The final-unterminated-line defect also has retained pre-Green CI evidence in
[Actions run 30106107011](https://github.com/WebFirstLanguage/wfl/actions/runs/30106107011).
Neither that behavior nor any later genuine Red-to-Green repair depends on this
exception.

## Why normal compliance cannot now be supplied

The earlier implementation combined regression tests and production changes in
the same commits. The focused failures described in the completion diary were
observed locally, but no test-only ancestor commit and no independently
timestamped pre-Green artifact was retained for the affected behaviors above.
A local `.git/objects/maintenance.lock` blocked the intended Git object writes
during that pass.

The missing historical ordering cannot be created after the implementation
date. Reverting or disabling completed code now would only demonstrate test
sensitivity; under Section 6.2 it would not prove the original TDD chronology.
Rewriting timestamps or presenting later characterization as earlier Red
evidence would manufacture evidence and is prohibited. This request is
therefore for temporarily unavailable historical evidence, not for schedule
pressure, test duration, inconvenience, a small-change claim, or permission to
ignore a current failure.

## Current Green evidence

- The reviewed Green head
  `8e8be0fcde944d0d7b357b94d5951497af5ff0b7` completed
  [Actions run 30142079511](https://github.com/WebFirstLanguage/wfl/actions/runs/30142079511),
  including Linux and Windows integration, TestPrograms, documentation
  validation, web/TLS, PostgreSQL, MariaDB, and fuzz-target compilation.
- The completion diary maps the affected contracts to focused Rust,
  real-socket, parser/typechecker/analyzer, WFL end-to-end, and security tests.
- Later issue #642 repairs use retained test-only Red ancestors and Green
  commits; those repairs strengthen the candidate but do not retroactively
  supply the chronology missing from the earlier mixed commits.

Actions run 30142079511 is evidence for the reviewed Green head, not automatic
evidence for the exact candidate in this draft. Before approval, the approval
record must link one complete, successful, unretried Actions run for the exact
final candidate (or its evidence-only descendant) and the final local gate
record. Until those fields are complete, current Green evidence is incomplete
for merge.

## Compensating verification and containment

Approval is conditional on all of the following:

1. Run, once and without changing test selection:
   `cargo fmt --all -- --check`, `git diff --check`,
   `cargo clippy --all-targets --all-features -- -D warnings`,
   `cargo build --release`, `cargo test --all --verbose --jobs 2`,
   `scripts/run_integration_tests.sh`,
   `python3 scripts/validate_docs_examples.py --ci --force`, and
   `scripts/run_web_tests.sh`.
2. Preserve the exact commands, exit conclusions, candidate SHA, and complete
   logs in the PR evidence record.
3. Require one complete GitHub Actions matrix on the exact candidate, covering
   Linux and Windows integration, TestPrograms, documentation validation, web
   tests, TLS, PostgreSQL, MariaDB, and fuzz-target compilation. Every required
   job must pass.
4. Obtain an independent R3 review of the implementation, regression
   assertions, real-boundary coverage, cleanup paths, and this exception's
   exact scope.
5. Confirm in the approval record that no required test was skipped, retried,
   quarantined, muted, weakened, or converted into a timing-only success
   assertion.
6. Freeze the executable candidate after approval. Any executable change
   requires a new full gate, scope review, exact SHA, and approval decision.
7. Do not release more than the single affected release, and do not merge or
   release after expiration. Expiration fails closed.

These controls establish present behavior and contain the exposure. They do not
replace or reconstruct the missing historical chronology.

## Residual risk

- Because the tests and earlier implementation were committed together, the
  record cannot prove that each test was specified independently of the chosen
  implementation. A test could encode the implementation while missing a
  different contract-preserving failure mode.
- Concurrency and socket lifecycle tests cover deterministic checkpoints and
  supported CI platforms, but do not exhaust every OS scheduler, socket-buffer
  size, cancellation ordering, or long-duration accumulation pattern.
- Parser/typechecker compatibility matrices cover the reported operand and
  scope shapes but cannot prove compatibility for every existing WFL program.
- Windows and Unix archive containment tests cover known rooted and traversal
  forms but do not constitute a proof over every filesystem namespace or future
  archive format.
- The older local Red observations are narrative only. They must not be cited
  as policy-compliant Red evidence.

There is no accepted known product-test failure in this draft. Discovery of a
reproducible product failure, authorization bypass, data loss/corruption,
exposed secret, unresolved Critical vulnerability, or another Section 14
non-waivable condition immediately invalidates this exception and blocks merge
or release.

## Rollback and recovery

No deployment, schema, persistent data, or external service state is changed by
PR #641.

- **Before merge:** stop the PR and rebuild the candidate from affected base
  `b25aed57ea50697c596796446d1f47466668773d`, preserving genuine test-only Red
  commits before each production repair. Do not force-push or rewrite evidence
  without an explicit maintainer decision and a retained mapping from the old
  candidate to the replacement.
- **After merge, before release:** revert the PR's merge/squash commit (or the
  exact affected commits if merged unsquashed), then run the complete gate on
  the revert candidate. Prefer forward repair when a broad revert would remove
  compatibility or lifecycle fixes that other changes now depend on.
- **After the one permitted release:** publish a normal tested forward repair
  or a revert release under the ordinary release gate. This exception cannot be
  reused for that release.

If a concurrency or cleanup regression appears, first disable release of the
candidate, preserve the failing boundary evidence, and repair it with a genuine
Red commit. If archive containment regresses, stop distribution of the affected
package artifacts and route the finding through `SECURITY.md`; do not disclose
new vulnerability details in a public issue.

## Repair ticket, owner, and deadline

- **Ticket:** [WebFirstLanguage/wfl issue #642](https://github.com/WebFirstLanguage/wfl/issues/642),
  testing-policy evidence gap.
- **Owner:** Brad, Maintainer and WFL test/reliability owner, Logbie LLC.
- **Deadline:** `2026-07-31T00:00:00-05:00`, before this exception expires and
  before merge or release.
- **Required resolution:** either (a) locate and retain admissible pre-Green
  artifacts for every row above, reducing or eliminating this scope; (b)
  replace the mixed implementation stack from the recorded affected base with
  genuine Red-to-Green ancestry and rerun the full gate; or (c) complete and
  approve this narrowly scoped record for the exact candidate and archive it
  with the release evidence. A later characterization run alone does not
  satisfy options (a) or (b).

Issue closure does not itself approve this exception. If the deadline passes
without one of these resolutions, the exception expires and the affected merge
or release remains blocked.

## Approval record — must be completed before activation

| Approval field | Required entry |
|---|---|
| Exact final executable candidate SHA | **PENDING** |
| Final evidence-only descendant SHA, if any | **PENDING / N/A** |
| Final local gate record | **PENDING** — commands, date, environment, and conclusions |
| Final GitHub Actions run | **PENDING** — URL and every required job conclusion |
| Independent R3 reviewer | **PENDING** — identity, date, and scope reviewed |
| Requester | **PENDING** — identity and date |
| Project/reliability owner decision | **PENDING** — Brad must record `APPROVE` or `REJECT`, rationale, date, and signature |
| Security-owner decision for archive-path scope | **PENDING** — Brad must record `APPROVE` or `REJECT`, rationale, date, and signature |
| No skip/retry/quarantine/muting/weakening/timing-only conversion attestation | **PENDING** |
| Maximum-release and expiration acknowledgment | **PENDING** |

The requester must not be the sole approver. If Brad is also the requester, a
separate authorized project/domain approver must approve; an independent review
that has no approval authority is not a substitute. The completed record must
remain attached to the PR and archived with release evidence for the retention
period required by Section 15.

**PENDING MAINTAINER APPROVAL — PR #641 MERGE GATE UNRESOLVED.**
