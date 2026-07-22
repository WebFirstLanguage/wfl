# WFL Testing — Policy & Project Profile

This repository adopts the **Logbie Testing Policy** (reproduced verbatim in
[§ Logbie Testing Policy](#logbie-testing-policy) below) and defines the WFL
project testing profile required by that policy's §4.

> **Adopted organization-policy version:** 1.0
> **Testing-profile review date:** 2026-07-22
> **Test-suite / infrastructure owner:** Maintainer (Brad, Logbie LLC)

---

## WFL project testing profile (Logbie Testing Policy §4)

### Supported configuration tuples

| Tuple | Presubmit | Release |
|---|---|---|
| Linux x86-64 (ubuntu-latest), Rust stable (MSRV **1.94+**, edition 2024) | ✅ | ✅ |
| Windows x86-64 (windows-latest), Rust stable | ✅ (integration) | ✅ |
| macOS | — | — |

> **macOS is not a gated tuple.** CI runs only `ubuntu-latest` and
> `windows-latest` (`.github/workflows/ci.yml`); there is no macOS runner in
> presubmit or release. macOS is supported only best-effort by contributors and
> is not verified by this pipeline. Add a macOS matrix entry before claiming any
> gated macOS coverage here.

Key runtime dependencies: `tokio`, `warp`/`hyper`, `reqwest`, `sqlx`, `logos`,
`tower-lsp`. The interpreter core is single-threaded (`Rc`/`RefCell`); async I/O
runs on Tokio.

### Test layers — one documented command each

| Layer | Command |
|---|---|
| Format (static) | `cargo fmt --all -- --check` |
| Lint (static) | `cargo clippy --all-targets --all-features -- -D warnings` |
| Unit + Rust integration | `cargo test --all` |
| WFL end-to-end programs | `cargo build --release` then `./scripts/run_integration_tests.sh` (`.ps1` on Windows) |
| Web-server end-to-end | `./scripts/run_web_tests.sh` (`.ps1` on Windows) |
| Docs examples validation | `python scripts/validate_docs_examples.py` |
| Benchmarks (perf, non-gating) | `cargo bench` |

**Run all presubmit checks** (clean checkout):

```bash
cargo fmt --all -- --check \
  && cargo clippy --all-targets --all-features -- -D warnings \
  && cargo test --all \
  && cargo build --release \
  && ./scripts/run_integration_tests.sh \
  && ./scripts/run_web_tests.sh \
  && python scripts/validate_docs_examples.py
```

Every command returns a non-zero exit status on failure. The presubmit suite is
hermetic: web/HTTP tests use local ephemeral servers and MUST NOT depend on the
public internet. Non-executable docs examples that need a live upstream or
external client are marked with a first-line `// CI-SKIP: <reason>` and are
validated statically (layers 1–4) via the docs-examples manifest instead.

### Required services, fixtures, credentials

- No external credentials or network for presubmit. Local TCP servers on
  ephemeral ports stand in for HTTP peers.
- SQLite for DB tests; no production data.
- TLS tests generate throwaway certs (`rcgen`).

### Critical user/operator journeys (release-blocking end-to-end)

1. **Run a program**: `wfl <file>` — lex → parse → analyze → typecheck →
   interpret, correct exit code (`tests/`, `TestPrograms/`, `run_integration_tests`).
2. **Web server request/response**: `listen` → `wait for request` → `respond`
   over a real socket (`run_web_tests`, `tests/web_server_*`).
3. **Streaming (client)**: `open url ... stream response` → `wait for next
   line|chunk` → `nothing` at EOF (`tests/http_stream_test.rs`).
4. **Streaming (server)**: `start streaming response` → `write line|chunk` →
   `close` (`tests/http_server_streaming_test.rs`).
5. **Concurrent handlers**: `main loop concurrently:` — a slow handler does not
   block a fast sibling; failures are contained (`tests/concurrent_main_loop_test.rs`).
6. **File I/O**, **outbound HTTP**, **REPL**, **crypto/hashing**.

### Risk triggers (§11)

- **Concurrency / streaming / lifecycle (§11.3):** any change to
  `main loop concurrently:`, request handling, or the streaming statements MUST
  add tests for races/ordering, cancellation, timeouts, disconnects, bounded
  queues/backpressure, resource limits, clean shutdown, and writes-after-close,
  and MUST prove one slow/failed operation does not block unrelated work. **R3.**
- **Untrusted input (§11.1):** lexer, parser, pattern VM, HTTP/multipart, and
  config readers require malformed/oversized/adversarial cases; fuzz targets
  (`cargo fuzz`) with a retained corpus for parser/pattern paths.
- **Security/crypto:** WFLHASH, password hashing, subprocess sanitization —
  positive/negative + invariant (constant-time, zeroize) tests. **R3.**
- **Backward compatibility (§11.6):** WFL is a language — every existing
  `TestPrograms/*.wfl` MUST keep passing; new syntax MUST NOT steal previously
  valid programs (regression tests required).

### Coverage & budgets

- Baseline: the policy's defaults (≥80% line / 70% branch overall; ≥90/85 on
  changed code) are the target. **Known gap:** the repo does not yet run an
  automated coverage gate in CI; establishing one is a tracked conformance item
  (§19). Until then, changed behavior MUST still ship behavior/boundary/negative
  tests at the lowest useful layer plus every affected higher layer.
- Performance budgets: Criterion benches under `benches/` are informational; no
  release-blocking latency budget is defined yet (tracked gap).

### CI jobs & gating

GitHub Actions (`.github/workflows/`) runs, per push/PR: format, clippy,
debug/release build, `cargo test`, Linux + Windows integration, **Run WFL
Programs**, web tests, database tests, and fuzz-target compilation. All are
required checks; the default branch MUST stay green. The nightly build is a
release artifact and is separately monitored.

### Evidence, runtimes, retention

- Expected presubmit runtime: minutes (Rust build dominates). Slow web/timeout
  tests use bounded deadlines, not unbounded sleeps.
- Evidence (Red→Green, CI run ids, commands) lives on the PR per §15; retained
  per §15 retention windows.

### Justified non-applicable layers

- **Accessibility/UI (§11.7):** WFL ships a CLI/LSP, no first-party GUI —
  UI/a11y layers are structurally N/A (LSP behavior is covered by `wfl-lsp`
  tests). Reviewer-confirmed.

### Conformance gaps (tracked, §19)

- No automated coverage gate in CI yet.
- No scheduled extended fuzz/soak profile yet (fuzz targets compile in CI; longer
  campaigns are not scheduled).
- No formal release-candidate artifact gate beyond the nightly build.

These gaps are owned by the Maintainer and do not authorize new untested
behavior; touched code still follows Red→Green and the risk triggers above.

---

## Logbie Testing Policy

> **Adoption note (this repository):** the text below is the organization policy
> reproduced verbatim, so it keeps the canonical **Status: Proposed** label and
> **Effective date: Upon adoption**. For WFL specifically, that adoption has
> happened: this repository has adopted policy version 1.0 as **binding and in
> force** (see the header at the top of this file, and `CLAUDE.md` / `AGENTS.md`),
> effective **2026-07-22** (the testing-profile review date). Read the "Upon
> adoption" below as "as of 2026-07-22" for this repo.

**Status:** Proposed organization policy, version 1.0
**Owner:** Logbie LLC Engineering
**Effective date:** Upon adoption
**Last updated:** July 22, 2026
**Applies to:** Every Logbie-owned software project, repository, package, service, application, game, agent, infrastructure definition, and release artifact

### 1. Purpose

Testing is executable evidence that a change behaves as intended, fails safely, and does not break supported behavior. It is part of design and implementation, not a cleanup task performed after the code appears finished.

This policy establishes the minimum testing standard for all Logbie projects. Individual projects may impose stricter rules, but they may not weaken this policy without a time-limited, recorded exception.

The objective is not to manufacture green dashboards. The objective is to make trustworthy changes and retain enough evidence for another engineer to understand what was proved.

### 2. Policy language

The terms in this document are normative:

- **MUST / MUST NOT** — mandatory. A violation blocks merge or release unless this policy explicitly permits an exception.
- **SHOULD / SHOULD NOT** — the normal expectation. Deviations require a written reason in the change record.
- **MAY** — optional and permitted.
- **Required test** — a test selected by this policy, the project's testing profile, the change's risk, or an acceptance criterion.
- **Change record** — the durable issue, ticket, or pull request that owns the work and its evidence.
- **Public contract** — behavior relied on outside the changed implementation, including user and operator workflows, public APIs, CLI behavior, protocols, events, schemas, stored formats, packages, configuration, and documented compatibility.
- **Critical journey** — an end-to-end workflow whose failure would prevent a user or operator from receiving a core outcome or would create material security, privacy, data-integrity, availability, or recovery risk.
- **Independent reviewer** — a qualified person or separately instructed review agent that did not author the implementation, examines the actual diff and evidence, and has no ability to approve merely by repeating the author's claims.
- **Release** — any production promotion, continuous-deployment rollout, package or container publication, app-store submission, infrastructure apply, or externally distributed prerelease. Renaming a release "just a deployment" does not change its gates.

### 3. Non-negotiable rules

1. Every behavioral change MUST have automated regression coverage at the lowest useful layer and every affected higher layer.
2. Every R1–R3 behavioral change MUST follow **Red → Green → Refactor → Broaden → Record**, except for the Green → Green maintenance rule in Section 6.3 or the incident rule in Section 17.
3. Every new, modified, or removed behavior MUST include auditable evidence that the relevant test failed for the expected reason before the production change made it pass. A defect fix MUST reproduce the defect.
4. A releasable product MUST have real end-to-end tests for its critical user and operator journeys. Those tests are release-blocking.
5. A test MUST NOT mock, stub, or bypass the boundary it claims to verify.
6. A mocked component test MUST NOT be labeled end-to-end.
7. Required tests MUST NOT be made green through automatic retries, skips, ignores, quarantine, muted failures, relaxed assertions, or unexplained snapshot regeneration.
8. A flaky required test is a failing test. It blocks merge until repaired or until the responsible change is reverted.
9. CI MUST test the integrated change from a clean checkout. A passing developer machine is useful evidence, not final evidence.
10. Coverage numbers MUST NOT substitute for behavior, boundary, negative-path, recovery, or end-to-end tests.
11. Tests and test infrastructure are production-quality code. They receive review, ownership, maintenance, and security controls.
12. AI-generated code, tests, summaries, and claims receive exactly the same verification as human-written work. "The model said it works" is not test output; it is optimism wearing a tiny hard hat.

### 4. Repository testing profile

Every repository MUST contain a root-level `testing.md`. It MUST either include this policy or link to the canonical version, and it MUST define a project-specific testing profile containing:

- The supported platform, architecture, runtime, browser, database, and dependency configuration tuples, including which run in presubmit and which run at release
- One documented command for each available test layer
- A single command or workflow that runs all presubmit checks
- Required services, containers, fixtures, credentials, hardware, and test data
- The project's critical user and operator journeys
- The risk triggers that require security, migration, performance, concurrency, recovery, fuzz, compatibility, or accessibility testing
- Coverage measurement and thresholds
- Performance and resource budgets, when applicable
- CI job names and which jobs block pull requests, merges, and releases
- The cadence, maximum evidence age, and invalidation triggers for required scheduled suites
- Expected test runtimes and the location of retained evidence
- Owners for the test suites and test infrastructure
- Any justified layer that does not apply to the project
- The adopted organization-policy version and the testing-profile review date

Commands MUST work from a clean documented environment and MUST return a nonzero exit status on failure. Local project rules may be stricter than this policy but MUST NOT silently redefine terms such as "end-to-end," "pass," or "release-ready."

The canonical organization copy of this file takes precedence over stale copied text. Repositories MUST adopt a new policy version before their next release and within 30 days unless a valid Section 17 exception says otherwise.

A monorepo MAY use one root profile, but every independently releasable package, service, application, or artifact MUST have an identifiable subprofile covering its commands, critical journeys, owners, compatibility matrix, and release gates.

A project-level "not applicable" declaration is permitted only when a layer is structurally impossible for that project type. It requires engineering-owner approval, a review date, and a technical explanation. It cannot override a risk introduced by a particular change.

If a repository lacks a valid testing profile, behavioral changes to that repository are not ready to merge.

### 5. Change risk classes

Every change MUST be assigned the highest applicable risk class before implementation. Executable changes default to R2 until the change record justifies another class. Risk may be raised during review; it MUST NOT be lowered merely to avoid a test gate.

When classification is ambiguous, the higher class applies. An R1 classification MUST explain why the change cannot affect a public contract, persistent state, security boundary, process boundary, or critical journey, and a reviewer MUST confirm it.

| Class | Typical changes | Minimum verification |
| --- | --- | --- |
| **R0 — Non-behavioral** | Prose-only documentation, comments, spelling, and assets proven not to affect shipped output or an acceptance criterion | Formatting, link or documentation build checks as applicable; confirmation that no executable behavior changed |
| **R1 — Local behavior** | Isolated logic with no public contract, persistence, security, or process boundary | Auditable Red → Green evidence, focused unit tests, relevant component tests, full affected suite, static checks |
| **R2 — Product or boundary behavior** | Public API, CLI behavior, UI flow, database access, filesystem behavior, service integration, packaging, configuration with runtime effect | R1 plus real integration or contract tests, affected critical-journey end-to-end tests, compatibility checks, clean CI |
| **R3 — Critical behavior** | Material changes to authentication, authorization, cryptography, secrets, protected user data, destructive operations, schema migration, money, safety, untrusted-input boundaries, protocol guarantees, concurrency, cancellation, lifecycle, recovery, release controls, or high-availability behavior | R2 plus negative and failure-path tests, applicable security/property/fuzz/concurrency/recovery/performance tests, independent review, recovery evidence, and the full release-relevant end-to-end suite |

All product releases, regardless of the individual changes they contain, MUST pass the full release gate in Section 14.

### 6. Test-driven development

#### 6.1 Required loop

For each acceptance criterion or defect:

1. **Specify** — express the behavior as an observable outcome, including relevant failure behavior.
2. **Red** — add or identify the smallest useful automated test and run it. Confirm that it fails for the intended reason.
3. **Green** — make the smallest coherent production change that satisfies the test.
4. **Refactor** — improve the implementation and tests while keeping them green.
5. **Broaden** — run the affected integration, contract, end-to-end, security, compatibility, and other risk-triggered suites.
6. **Record** — attach the evidence required by Section 15 to the change record.

The Red step is invalid if the test fails because of a syntax error, broken fixture, missing dependency, unrelated failure, or an assertion that does not represent the required behavior.

#### 6.2 Acceptable Red → Green evidence

At least one of the following MUST be retained:

- A focused test-only Red commit that is an ancestor of the Green implementation commit
- An independently timestamped CI or change-record artifact created before the Green implementation commit, tied to a Red revision and showing the test name, command, expected behavior, actual failure, and failure reason
- For a reproduced defect, an automated regression test applied to the recorded affected base revision and retained in a Red commit before the Green fix

The evidence MUST identify the base, Red, and Green commit identifiers. The final history may be squashed after the evidence is attached to the change record. A newly written test that was observed only after the implementation already passed it does not establish the required Red step. Reverting or disabling completed code may prove that a regression test is capable of failing, but it does not retroactively prove TDD chronology.

#### 6.3 Refactors and non-behavioral changes

A behavior-preserving refactor or maintenance change MUST establish adequate characterization coverage and record a passing baseline before the change, then pass the same coverage afterward. Dependency, toolchain, packaging, infrastructure, and configuration maintenance also require applicable compatibility, security, integration, and end-to-end evidence. They do not need an artificial failing test when no behavior is intended to change.

R0 changes do not require a manufactured Red step. Configuration, build, workflow, dependency, infrastructure, schema, and documentation-generator changes are not R0 when they can change executable behavior.

#### 6.4 Incidents

During an active incident, the minimum reversible mitigation may precede the normal Red step only under Section 17. The defect MUST receive regression coverage before the incident ticket is closed. An emergency is a reason to reorder evidence, not to delete it.

### 7. Required test layers

The risk table, project testing profile, acceptance criteria, and Section 11 triggers determine the required layers. Within that set, a change MUST use every layer needed to prove the affected contract without duplicating tests that add no distinct evidence. "Not applicable" requires a specific technical explanation in the change record and reviewer acceptance.

#### 7.1 Static verification

Projects MUST run applicable formatting, compilation, linting, type checking, schema validation, policy checks, secret scanning, dependency checks, and generated-file consistency checks.

Static verification supplements executable tests; it does not replace them.

#### 7.2 Unit tests

Unit tests MUST cover new or changed business rules, validation, algorithms, state transitions, parsers, error classification, and policy decisions when those behaviors can be isolated.

Unit tests SHOULD be fast, deterministic, precise, and independent of network or shared external state. They SHOULD assert observable behavior rather than private implementation details.

#### 7.3 Component and service tests

Component tests verify a complete module, package, process, or service through its public interface. They MUST use real internal components for the behavior under test and MAY substitute only dependencies outside the declared component boundary.

#### 7.4 Integration tests

Integration tests MUST exercise real boundaries whenever the change affects them, including as applicable:

- Database engines, schemas, transactions, migrations, and queries
- Filesystems, permissions, paths, locks, and storage formats
- Processes, signals, standard streams, exit codes, and packaged binaries
- HTTP, WebSocket, streaming, queue, event, RPC, and protocol behavior
- Authentication, authorization, redaction, and policy enforcement
- Containers, operating-system facilities, and service discovery
- Timeouts, retries, cancellation, disconnects, backpressure, restart, and idempotency

An in-memory replacement is not evidence that the actual database, filesystem, queue, protocol, or operating-system integration works.

#### 7.5 Contract tests

Every public or cross-service interface MUST have contract tests covering successful responses, errors, versioning, required fields, optional fields, limits, malformed input, and backward compatibility.

Cross-repository contracts MUST name the provider owner, consumer owner, compatible version range, and repository responsible for candidate compatibility testing. Coordinated provider and consumer changes MUST test supported version skew before either side releases.

Provider simulators and deterministic adapters MAY support fast tests, but a project that claims compatibility with an external provider MUST also verify the contract against that provider's official test environment, sandbox, or independently controlled conformance reference defined in the project testing profile. If the provider offers no safe test environment, a versioned signed recording or reference corpus MAY substitute only with owner approval, a declared freshness limit, and proof that it covers the claimed provider version. The project MUST state that live interoperability was not verified.

#### 7.6 End-to-end tests

End-to-end tests exercise a complete user- or operator-visible journey through the production entry points and shipped artifact. They MUST:

- Start from a clean, production-like state
- Use the real application binary or packaged artifact
- Cross the real in-scope process, storage, protocol, and UI boundaries
- Assert both the final outcome and important externally visible side effects
- Exercise cleanup or recovery where the journey changes state
- Produce enough evidence to diagnose a failure

Browser products MUST use a real supported browser for browser journeys. Service products MUST use their real network interface. CLI and desktop products MUST execute the packaged binary. Libraries MUST provide consumer, conformance, or system-harness tests that exercise the published artifact as a downstream user would.

If an external paid or unsafe system is replaced, the test MUST be labeled as a system test rather than a true end-to-end test of that external integration. The external integration then requires a separate credentialed sandbox or release smoke profile, or the approved conformance-reference path in Section 7.5 when no safe provider environment exists.

Every product MUST define a small, reliable critical-journey suite that blocks merge when affected and blocks every release in full.

Every new or changed user- or operator-visible boundary behavior MUST add or update an end-to-end assertion unless an existing test already asserts that exact observable outcome. The critical-journey suite is the always-release-blocking subset, not a loophole for leaving noncritical workflows unproved.

#### 7.7 Exploratory and manual testing

Manual and exploratory testing MAY discover issues and provide useful product evidence. They MUST NOT replace required automated regression tests. Any defect found manually MUST receive automated coverage. A platform limitation that makes automation impossible requires a Section 17 exception and cannot waive a critical release gate.

### 8. Test integrity

#### 8.1 Determinism

Tests MUST control or record time, randomness, locale, time zone, network assumptions, identifiers, and ordering when those inputs affect results. Randomized tests MUST report the seed and retain failing inputs.

Date and time behavior MUST cover applicable expiration boundaries, daylight-saving transitions, leap dates, time zones, and locale changes. Tests capable of blocking MUST have an explicit bounded timeout.

Tests MUST be isolated from one another. They MUST NOT rely on execution order, shared mutable fixtures, production state, or residue from a previous run.

#### 8.2 Failures, flakes, and retries

- A required test that fails once has failed.
- Required tests MUST NOT automatically retry at the test, framework, or CI layer.
- Required-suite configuration MUST expose first-attempt results and disable hidden framework or CI retries.
- A CI job MAY be rerun only when independent evidence shows that the runner or external test infrastructure failed before a product-test result was produced. The original run, evidence, and reason MUST remain visible. This permitted infrastructure rerun is not a test retry.
- A test that passes only on retry is flaky and blocks merge.
- Required tests MUST NOT be skipped, ignored, muted, quarantined, marked "allowed to fail," or removed from the gating suite to obtain green CI.
- Platform-specific tests MAY be selected only on their declared matrix entries, but the release gate MUST execute every supported entry.
- A known flaky test on the default branch is an urgent repository defect. The default branch MUST be restored to trustworthy green before unrelated behavioral work merges.

#### 8.3 Assertions and snapshots

Tests MUST assert meaningful outcomes, error behavior, and side effects. "Did not crash" is insufficient when the behavior has a defined result.

Snapshots and golden files MUST be human-reviewable. Their changes MUST be reviewed like production code. Bulk regeneration without explaining each intentional behavioral difference is prohibited.

Negative assertions MUST be used where absence matters, including authorization denial, secret non-disclosure, duplicate prevention, rollback, cancellation, and writes outside an allowed boundary.

#### 8.4 Test doubles

Mocks, fakes, stubs, emulators, and simulators MUST be named accurately and confined to a declared boundary. A test double MUST NOT make the behavior under test impossible to fail.

Important doubles SHOULD be checked against the real implementation through contract tests so they do not become cheerful little liars with perfect uptime.

### 9. Coverage and test strength

Coverage is a diagnostic and regression floor, not a target that proves correctness.

Unless a stricter project profile applies:

- New projects MUST maintain at least **80% line coverage** and **70% branch coverage** across instrumentable first-party executable code before their first production release.
- Changed instrumentable executable code MUST achieve at least **90% line coverage** and **85% branch coverage**.
- A change MUST NOT reduce repository line or branch coverage by more than 0.5 percentage points without an approved exception. The stored baseline MUST never be silently lowered.
- Security, authorization, destructive-operation, financial, migration, and other R3 decision logic MUST have explicit tests for every identified policy outcome and failure mode, regardless of the aggregate percentage.

Generated code, vendored code, build output, and provably unreachable platform shims MAY be excluded. Exclusions MUST be reviewable configuration, not ad hoc command-line omissions.

"Changed code" means added or modified first-party executable lines relative to the target branch's merge base, with renames tracked when the tool supports them. Base and head MUST be measured in the same CI job using the same version-controlled coverage configuration, tool version, platform, exclusions, and rounding to two decimal places. A tool or configuration change that alters the denominator requires an old-versus-new comparison and test-infrastructure-owner approval.

If reliable branch coverage is unavailable for a language, the testing profile MUST name the limitation and define reviewed condition, decision, scenario, or mutation coverage that supplies equivalent evidence. Calling code "non-instrumentable" without this approved alternative is not an exclusion.

When conventional coverage is meaningless—such as declarative infrastructure, visual assets, or hardware workflows—the project testing profile MUST define scenario, requirement, state, or interface coverage instead.

R3 projects SHOULD use mutation testing or an equivalent test-strength analysis for critical logic before a major release. Surviving meaningful mutations indicate missing assertions even when line coverage looks impressive.

### 10. Test data, fixtures, and environments

- Tests MUST NOT use production secrets, credentials, private keys, or uncontrolled personal data.
- Production-derived data MUST be minimized, sanitized, approved, and documented before use.
- Destructive tests MUST run only in explicitly disposable environments with guardrails that make production targeting impossible.
- Test resources MUST use unique names or isolated namespaces and MUST clean up on success, failure, cancellation, and timeout.
- Disposable environments MUST also have an independent time-to-live or janitor cleanup path for hard runner termination, where test code cannot execute cleanup.
- Fixtures MUST be small enough to review and version unless a justified artifact store is defined.
- Schema, protocol, and file-format fixtures MUST include the oldest supported version, current version, malformed cases, boundary sizes, and forward-compatibility cases where applicable.
- Credentials for sandbox or release profiles MUST be short-lived, least-privileged, redacted from output, and unavailable to untrusted pull requests.
- Test logs and artifacts MUST be sanitized before retention.

The standard presubmit suite SHOULD be hermetic and MUST NOT depend on the public internet. Credentialed, hardware, provider, load, soak, and privileged tests belong in explicitly named profiles with controlled environments.

### 11. Risk-triggered testing

The following requirements apply whenever the corresponding risk exists.

#### 11.1 Security and privacy

Changes affecting trust boundaries, identity, authorization, secrets, untrusted input, or personal data MUST include:

- Positive and negative authorization cases
- Role, tenant, and ownership-boundary tests
- Malformed, oversized, replayed, duplicated, and adversarial input cases
- Secret-redaction and sensitive-log assertions
- Session, token, timeout, revocation, and failure behavior as applicable
- Abuse-limit and resource-exhaustion tests where applicable
- A security-focused independent review for R3 changes

Parsers, decoders, protocol handlers, and file readers exposed to untrusted input MUST have property or fuzz tests with a retained regression corpus. A bounded fuzz smoke run SHOULD execute on pull requests; longer campaigns SHOULD run on a scheduled profile.

A known Critical security finding is non-waivable for a normal release. A known High finding blocks release unless the security owner approves a narrowly scoped Section 17 exception with demonstrated mitigation. A documented false positive supported by evidence is a resolved finding, not an exception.

#### 11.2 Persistence and migrations

Schema, data, and storage-format changes MUST be tested from every supported prior version using representative data. Tests MUST prove:

- Upgrade correctness and idempotency
- Preservation of required data and constraints
- Behavior during partial failure, interruption, and restart
- Compatibility during any rolling or mixed-version deployment window
- The documented rollback, restore, or forward-repair strategy

Destructive or irreversible migrations require explicit approval, a verified backup or recovery artifact, and a rehearsal in a production-like disposable environment.

#### 11.3 Concurrency, streaming, and lifecycle

Concurrent, asynchronous, networked, streaming, or long-running behavior MUST test applicable races, ordering, cancellation, timeouts, disconnects, bounded queues, backpressure, resource limits, clean shutdown, restart, and writes after close.

Queue and event-driven behavior MUST test duplicates, delays, reordering, replay, poison messages, partial acknowledgement, and idempotent recovery when those conditions are possible.

Tests MUST prove that one slow or failed operation does not improperly block unrelated work. Where the language or platform supplies race detection, concurrency modeling, sanitizers, or deterministic schedulers, the project SHOULD include them in CI or scheduled testing.

#### 11.4 Reliability and recovery

Stateful or continuously running systems MUST test crash recovery, restart, duplicate delivery, partial completion, idempotency, lost dependencies, corrupted or stale inputs, and degraded operation.

High-availability projects MUST define scheduled chaos, failover, and soak profiles. A release MUST NOT claim a recovery property that has never been exercised.

#### 11.5 Performance and resource use

Projects with latency, throughput, memory, storage, startup-time, battery, network, or cost requirements MUST define measurable budgets in their testing profile.

Performance tests MUST use controlled workloads, warmup rules, environments, and comparison methods. A statistically meaningful budget regression blocks release unless explicitly accepted under Section 17. Microbenchmarks alone do not prove system capacity.

#### 11.6 Compatibility and packaging

Every supported matrix entry MUST be tested before release. Projects MUST test the artifact users actually install or run, including package metadata, default configuration, startup, upgrade, and uninstall or cleanup behavior when applicable.

Feature flags MUST be tested in their default and non-default states, including authorization and migration behavior affected by the flag. Removing a flag requires tests for the resulting permanent path.

Examples and published code snippets SHOULD compile or execute in CI. Public libraries MUST test supported consumers and backward compatibility according to the project's versioning policy.

#### 11.7 User interfaces and accessibility

User-facing applications MUST test critical journeys through the real UI. Applicable flows MUST cover keyboard operation, focus behavior, accessible names, error presentation, and the project's accessibility target. Automated accessibility checks MUST be supplemented by documented manual checks for major releases where automation cannot establish the behavior.

Visual regression tests MAY supplement behavioral tests but MUST NOT replace them.

#### 11.8 Infrastructure and deployment

Infrastructure-as-code and deployment changes MUST pass syntax, policy, plan, idempotency, least-privilege, secret-handling, and rollback, restore, or forward-repair checks. R2 and R3 changes MUST be exercised in a disposable or staging environment before production.

Tests MUST make destructive plans obvious and MUST prevent production mutation from ordinary CI.

#### 11.9 Games and deterministic simulations

Game and simulation projects MUST make authoritative logic testable deterministically. If presentation coupling prevents direct isolation, the project testing profile MUST define a deterministic system harness. Tests MUST cover seeded replay, invariants, boundary conditions, save/load round trips, version compatibility, economic or scoring conservation rules, and long-run stability as applicable.

Balance evaluation and bot simulations are evidence for design decisions, but they do not replace correctness tests.

#### 11.10 AI and model-backed features

Model-backed behavior MUST keep deterministic application rules under ordinary automated tests. Provider adapters require contract tests. Prompt, model, retrieval, or tool-policy changes MUST use versioned evaluation sets with documented pass thresholds, safety cases, cost limits, and regression comparisons.

Live model evaluations MUST run only in an authorized credentialed profile. Their nondeterminism MUST be measured and reported; it MUST NOT be hidden with retries until a preferred answer appears.

#### 11.11 Observability and audit behavior

R3 services and agents MUST test required audit events, security-relevant logs, metrics, trace propagation, alert conditions, and redaction for privileged operations and critical failures. Tests MUST prove both that required signals appear and that secrets or protected data do not.

### 12. Test design and maintenance

- Test names MUST describe the behavior and relevant condition.
- A test SHOULD have one clear reason to fail, while a scenario test MAY make several related assertions needed to diagnose the journey.
- Tests SHOULD use public interfaces and stable contracts.
- Shared fixtures and helpers MUST reduce accidental complexity without hiding important setup or assertions.
- Timing-based waits and arbitrary sleeps SHOULD be replaced by observable readiness conditions and bounded deadlines.
- Test suites MUST be runnable in parallel only when their isolation supports it.
- Slow tests MUST be measured and improved, split by profile, or assigned appropriate infrastructure; they MUST NOT be silently removed from required evidence.
- Deleted behavior SHOULD have obsolete tests removed. Changed tests MUST explain whether the product contract changed or the prior test was incorrect.
- A production defect that escaped existing tests MUST add or improve the layer that should have caught it, not only the layer where it was easiest to reproduce.

Test-only hooks in production code MUST NOT weaken security or alter normal behavior. If unavoidable, they require review and must be inaccessible in production builds or deployments.

Changes to CI workflows, test filters, impact maps, retry settings, coverage tools or exclusions, thresholds, required-check names, risk rules, or `testing.md` MUST receive test-infrastructure-owner approval and run the complete presubmit suite. A change MUST NOT weaken the machinery that judges that same change.

CI MUST fail closed when the change record lacks a risk class, a required job has no result, or the executed job set does not match the approved testing profile and risk triggers.

### 13. Continuous-integration profiles

Projects MUST define the following profiles where applicable.

Default and release branches MUST be protected. Required checks and review rules apply to maintainers, administrators, bots, and merge queues; ordinary work MUST NOT use an administrative bypass. Section 17 is the only bypass path and it MUST be logged.

#### 13.1 Pull request / presubmit

Runs from a clean checkout with locked dependencies and pinned or recorded tool versions, and includes:

- Formatting, build, lint, type, policy, secret, and dependency checks
- Focused and complete unit suites
- Affected component, integration, and contract suites
- Affected critical-journey end-to-end tests
- Coverage and changed-code thresholds
- Bounded property, fuzz, security, or concurrency smoke tests triggered by risk

Required checks MUST pass on the final proposed commit.

Path-based or impact-based test selection MAY reduce presubmit work only when backed by a maintained dependency map. The merge queue or another pre-merge gate MUST still run the complete impacted suite.

#### 13.2 Merge queue / integrated branch

The change MUST be tested with the latest target branch and other queued changes. The merge result MUST pass all required presubmit checks from a clean environment. Stale green results from an earlier base are insufficient.

The default branch MUST remain green. A red default branch is an incident owned ahead of feature work.

#### 13.3 Scheduled extended profile

Longer fuzzing, property exploration, compatibility matrices, credentialed sandboxes, race detection, sanitizers, load, chaos, failover, and soak tests SHOULD run on a documented schedule according to project risk.

Failures MUST create or update an owned change record and block release. They MUST block further affected merges when they invalidate presubmit evidence.

The project profile MUST define freshness for each scheduled suite; seven days is the default maximum. A new run is required sooner when relevant production code, dependencies, toolchain, tests, configuration, environment, or candidate artifacts change.

#### 13.4 Release candidate

Runs against the immutable release candidate artifact set and includes:

- The complete supported platform and dependency matrix
- The full critical-journey end-to-end suite
- Installation, startup, upgrade, migration, compatibility, and rollback, restore, or forward-repair checks
- All R3 security, recovery, concurrency, and data-integrity suites
- Required performance, load, soak, provider, hardware, and accessibility evidence
- Artifact integrity, provenance, license, dependency, and secret checks

Every released binary, package, image, installer, or other artifact MUST be identified and tested. The tested artifact set MUST be the set released, and each digest or equivalent immutable identifier MUST be recorded. Rebuilding after the gate invalidates the gate.

A platform-controlled signing, notarization, store-repackaging, or deployment transformation MAY occur after the main gate only when its input and output provenance are recorded, the transformation is deterministic or independently verified, and the distributed result passes a post-transformation smoke test. Unchanged evidence MAY be reused only for the exact same immutable artifact and while every required scheduled result remains fresh.

#### 13.5 Post-deployment smoke

Deployed services SHOULD run a small non-destructive smoke suite that verifies health and critical external paths. Production smoke tests MUST be explicitly authorized, isolated from ordinary user data, safe to repeat, monitored, and incapable of destructive action.

Post-deployment checks supplement the release gate; they do not excuse missing pre-release evidence.

### 14. Merge and release gates

Section 17 is the sole override path. An exception may authorize a clearly labeled emergency merge or build, but it never converts missing or failed evidence into a pass. A reproducible product-test failure, authorization bypass, data loss or corruption, exposed secret, unresolved Critical vulnerability, or failed destructive-migration recovery test is non-waivable.

#### 14.1 A pull request MUST NOT merge when

- A required check failed, did not run, or produced ambiguous results
- A required test was automatically retried, skipped, ignored, muted, quarantined, allowed to fail, or rerun outside the proven infrastructure-failure rule in Section 8.2
- Red → Green evidence is missing for changed behavior
- Coverage or a declared budget regressed beyond policy
- The affected critical journey lacks end-to-end coverage
- The target branch or required baseline is red, unless this pull request is narrowly scoped to repairing or reverting that failure and all unrelated required checks pass
- Required test evidence cannot be tied to the final commit
- An unresolved blocking review, security finding, migration risk, or rollback gap remains
- Test changes weaken protection without an approved contract change

#### 14.2 A release MUST NOT proceed when

- The immutable candidate did not pass the full release-candidate profile
- Any supported platform or critical journey is untested or failed
- A required scheduled test has an unresolved failure that affects the release
- A Critical security issue remains unresolved, or a High issue lacks the valid security-owner exception allowed by Section 11.1
- Data migration, compatibility, and rollback, restore, or forward-repair evidence is incomplete
- A declared performance or reliability claim lacks passing evidence
- Required artifacts, test reports, or approvals are missing

"It is probably fine," a deadline, and repeated clicking of the rerun button are not release criteria.

### 15. Required change evidence

Every behavioral pull request or equivalent change record MUST include:

- Change and risk-class summary
- Impacted repositories, release artifacts, public contracts, supported configurations, and critical journeys, with reviewer acceptance of the impact analysis
- Acceptance criteria mapped to tests
- Red evidence for each new behavior or defect fix
- Green evidence tied to the final commit
- Tests added, changed, or removed
- Exact commands or CI run identifiers for unit, integration, contract, end-to-end, and risk-triggered tests
- Coverage results and any exclusions
- Supported matrix entries exercised
- Known limitations, residual risks, and specifically justified non-applicable layers
- Review evidence required by the risk class
- Rollback, restore, or forward-repair instructions when external state can change

Recommended pull-request section:

```markdown
## Test evidence

- Risk class:
- Acceptance criteria → tests:
- Red evidence:
- Unit/component:
- Integration/contract:
- End-to-end:
- Security/migration/concurrency/performance/other:
- Coverage:
- Platforms:
- Not applicable, with reason:
- Rollback/recovery:
- Residual risk:
```

Pull-request evidence MUST remain accessible for at least 90 days after merge or closure. Default-branch and scheduled evidence MUST remain accessible for at least 180 days. Release, security, migration, rollback, and exception evidence MUST remain accessible for the supported lifetime of the release plus one year, and never less than 24 months. Evidence MUST NOT contain secrets or uncontrolled personal data.

Every release MUST archive an organization-controlled evidence manifest containing the commit, artifact-set identifiers, policy and testing-profile versions, required job results, tested configuration tuples, commands, toolchain and environment, seeds, coverage and scan summaries, exceptions, approvals, and recovery evidence. Short-lived CI links alone do not satisfy retention.

### 16. Ownership and defect handling

The author of a change owns its tests until the change is accepted. The project owner owns the ongoing health of the suites and infrastructure.

When a production defect escapes:

1. Reproduce it with the strongest feasible automated test.
2. Identify which test layer should have prevented the escape.
3. Fix the defect using the Red → Green loop.
4. Repair the missing assertion, fixture, environment, or gate.
5. Search for the same gap in adjacent behavior.
6. Record the cause and prevention evidence.

CI infrastructure failures MUST be distinguished from product failures using evidence, not guesses. Repeated infrastructure instability is itself a blocking engineering defect.

### 17. Exceptions and emergency changes

An exception records temporary risk; it does not convert missing evidence into a pass.

Every exception MUST identify:

- The exact rule and affected scope
- Why compliance is technically impossible or would worsen an active incident
- The approving project owner and, for security or privacy rules, the security owner
- Start time, expiration, and maximum affected releases
- Compensating verification and containment
- Rollback plan
- A linked repair ticket with owner and deadline

An exception is limited to one repository, an exact commit or candidate, and at most one release. The requester MUST NOT be the sole approver. Ordinary exceptions expire within 30 days; R3 exceptions expire within 7 days and require the affected security, data, reliability, or other domain owner. Renewal requires new evidence and approval; repeated renewal requires escalation to the Logbie engineering policy owner. An expired exception blocks the affected merge or release.

Schedule pressure, test duration, missing CI setup, inconvenience, and "the change is small" are not valid reasons.

Exceptions MUST NOT permit a normal release to claim that an untested critical journey, unsupported migration, or failed required suite passed. The release remains blocked or is explicitly classified as an emergency build with its limitations visible.

An ordinary exception MAY address temporarily unavailable evidence or an unavailable environment. It MUST NOT waive a known product failure or any non-waivable condition in Section 14. A required test cannot be retried, skipped, quarantined, muted, or relabeled through an exception to manufacture a pass.

During an active incident, an authorized minimum reversible mitigation MAY merge with reordered Red → Green evidence when delay would cause greater harm. All available focused tests and static checks MUST still run, the rollback path MUST be prepared, and regression coverage plus the full affected suites MUST pass within 24 hours, before the incident is closed, and before another normal deployment of the affected component.

Expired exceptions fail closed.

### 18. Definition of done

A change is done only when all applicable conditions are true:

- Acceptance criteria are observable and mapped to passing tests.
- Required Red → Green evidence exists.
- Relevant unit, component, integration, contract, and end-to-end tests pass.
- Static, coverage, security, compatibility, performance, migration, recovery, and other risk-triggered gates pass.
- The final integrated commit passes in a clean CI environment.
- Required independent review is complete and blocking findings are resolved.
- Documentation, examples, configuration, fixtures, and operational instructions are updated.
- Rollback or recovery covers both repository content and external state.
- Evidence is attached to the durable change record and contains no secrets.
- No required test is flaky, skipped, retried, quarantined, muted, or unexplained.
- Remaining work and risks are explicitly ticketed and do not violate a merge or release gate.

If a required condition is unmet, the correct state is **blocked**, **in progress**, or **ready for approval**—not **done**.

### 19. Adoption checklist

An existing repository MUST add a valid testing profile before its first behavioral pull request after policy adoption or within 30 days, whichever comes first. A temporary delay requires Section 17; no production release may occur before profile adoption. Each repository MUST complete the following before its next production release:

- [ ] Add a root-level `testing.md` adopting this policy
- [ ] Name test owners
- [ ] Document clean-environment commands for every applicable layer
- [ ] Inventory critical user and operator journeys
- [ ] Establish the supported compatibility matrix
- [ ] Establish coverage baselines and required thresholds
- [ ] Configure presubmit and merge protection
- [ ] Build a release-candidate workflow against the immutable artifact
- [ ] Remove or repair required retries, skips, quarantines, and allowed failures
- [ ] Define security, migration, concurrency, recovery, performance, and other risk triggers
- [ ] Define test-data and credential controls
- [ ] Define evidence retention and artifact locations
- [ ] Record and prioritize gaps that prevent full compliance

Until adoption is complete, gaps MUST be visible as owned tickets. A repository MUST NOT describe itself as fully compliant while a mandatory gate is missing.

Existing projects MUST apply this policy immediately to new and changed behavior. Touched legacy behavior MUST gain characterization coverage, and known repository-wide gaps MUST have a dated conformance plan. Existing debt does not authorize new untested behavior.

### 20. Canonical rule

**No behavioral change without an honest failing test. No boundary claim without a real boundary test. No release without real end-to-end proof. No green build manufactured from retries, skips, quarantine, or wishful thinking.**
