# Dev Diary — 2026-07-13: Issue #610 Phase 1 — Baseline & containment (remaining tasks)

**Tracker:** [#610 — WFL Production Readiness: 8/10 by January 1, 2027](https://github.com/WebFirstLanguage/wfl/issues/610)
**Phase 1 target window:** July 12 – August 15, 2026.

This entry records the five Phase 1 tasks executed after the issue inventory
(`…-phase-1-inventory.md`) and scorecard baseline (`…-phase-1-scorecard-baseline.md`):

1. Finish and integrate the shared ExecutionBudget
2. Convert every known correctness defect into an end-to-end regression test
3. Establish fuzz targets for lexer, parser, pattern engine, and module loading
4. Record baseline metrics
5. Define supported platforms and support boundaries

All work was measured against **WFL 26.7.36** (`fc21f2f`, Linux, `cargo build --release`).

---

## 1. Shared ExecutionBudget — finished & integrated (verified)

The shared budget landed in **#609** (`src/exec/budget.rs`, `src/exec/mod.rs`).
Phase 1's job here was to confirm it is *finished and integrated*, i.e. that a
single object covers every dimension the mandatory gate enumerates and that it
is wired through the whole pipeline.

**Enforcement surface (every gate dimension has a public method):**

| Gate dimension | `ExecutionBudget` method |
|---|---|
| Deadline / cancellation | `charge_operation`, `check_deadline`, `cancel`, `check_cancelled` |
| Operation ceiling | `charge_operation` (optional `max_operations`) |
| Recursion / import / execute-file depth | `check_call_depth`, `check_import_depth`, `check_execute_file_depth` |
| Pattern steps / states | `check_pattern_steps`, `check_pattern_states` |
| Source / request-body / response bytes | `check_source_bytes`, `check_request_body_bytes`, `check_response_bytes` |
| Pending HTTP requests | `max_pending_requests` |
| WebSocket queue / connections | `ws_queue_bound`, `try_acquire_ws_connection`, `max_ws_connections` |

**Wired through the pipeline** (reference counts of `budget` usages):
lexer (parsing input), parser, analyzer, type checker, interpreter (evaluation +
`respond` + module/include + `execute file`), and the pattern VM (matching). The
budget is `Send + Sync` (atomics only) so an `Arc<ExecutionBudget>` crosses into
the multi-threaded web transport without any `Rc`/`RefCell` rewrite.

**Tests:** `tests/execution_budget_test.rs` (33 tests) covers config parsing of
every `.wflcfg` budget key plus end-to-end enforcement (recursion → clean
catchable error instead of SIGABRT; oversized source refused before running).

**Verdict:** the mandatory gate *"Shared runtime execution budget covers parsing,
evaluation, pattern matching, web handling, and module loading"* is **met**. The
adjacent gate — *adversarial* tests for each limit — is explicitly **Phase 3**
and is not claimed here.

## 2. Known correctness defects → end-to-end regression tests

New suite `tests/phase1_correctness_regression_test.rs` is the single auditable
index of every inventoried correctness defect, each mapped to an end-to-end test
that drives the release `wfl` binary against the defect's own repro. It has two
halves:

- **Fixed defects → passing guards.** New coverage for **#569** (action-call
  result is `Text`, not `Nothing`, in a Text-required position — no spurious
  "found Nothing") and **#571** (precedence, `/` division, `modulo`, `is
  between`). Defects already covered elsewhere (#582/#557/#566/#567/#583/#588 in
  `github_issues_batch_test.rs`, #580 in `include_of_form_resolution_test.rs`,
  #590 in `recursive_action_return_type_test.rs`) are indexed in the file's
  module doc rather than duplicated.
- **Open defects → `#[ignore]`d desired-behaviour tests** that reproduce the bug
  today (they fail under `--ignored`) and flip green the moment Phase 2 fixes
  land — no new file needed, just remove `#[ignore]`:
  - **#592** — bare zero-arg include-exposed action is fatal at top level.
  - **#578** — `list files … with pattern` returns 0; `one or more` quantifier
    matches per-char (16 vs 4 words); `repeat N times` is a parse error;
    `Number plus Text` silently concatenates; no text→number conversion.

This satisfies *"convert every known correctness defect into an end-to-end
regression test"* while keeping CI green (open defects are ignored, not failing).

Re-verification surfaced one correction to the inventory: #578's *`X ends with
Y` misparse* item **no longer reproduces** on 26.7.36 (fixed alongside #566), so
it was not encoded as an open defect.

## 3. Fuzz targets established

New standalone cargo-fuzz workspace under `fuzz/` (kept out of the stable root
build via its own `[workspace]` and root `exclude = ["fuzz"]`):

| Target | Surface |
|---|---|
| `fuzz_lexer` | `lex_wfl_with_positions_checked` |
| `fuzz_parser` | lex → `Parser::parse` |
| `fuzz_pattern` | `create pattern` parse → `CompiledPattern::compile` → `find_all` (ReDoS surface) |
| `fuzz_module_loading` | include/load-module detection + include-aware `Analyzer::analyze` on arbitrary module content |

Each target's invariant: no arbitrary input may panic, overflow the stack, or
hang. Tracked seed inputs live in `fuzz/seeds/<target>/`; the live corpus and
artifacts are gitignored. `fuzz/README.md` documents running them under nightly.

Targets **type-check cleanly on stable** (`cargo check --manifest-path
fuzz/Cargo.toml`), proving the API wiring is correct. The *sustained run* (with
corpus retention) is deliberately **Phase 3** work; no run duration is claimed at
baseline (see metrics below). Async/FS-backed module *resolution* is out of scope
for `fuzz_module_loading` and noted as follow-up.

## 4. Baseline metrics (2026-07-13, WFL 26.7.36, Linux)

**Methodology note.** The aggregate Rust test tally is taken from the
**CI-measured** baseline on this exact commit (`fc21f2f`), recorded in the
scorecard-baseline entry, **plus this change's verified delta**. The full
`cargo test --all` is *not* re-measured locally for the baseline because this
sandbox lacks the live PostgreSQL/MariaDB and web-server services CI provides, so
a local run would report environmental (not regression) failures in the
database/web suites. The delta is the one new suite added here, run in isolation:
`phase1_correctness_regression_test` → **2 passed, 0 failed, 6 ignored**.

| Metric | Baseline | Source / notes |
|---|---|---|
| Rust test count | **1479 passed / 0 failed / 22 ignored** | CI baseline 1477/0/16 (`fc21f2f`) + this change's +2 passing, +6 ignored |
| Test suites | **95** | 94 CI baseline + 1 new (`phase1_correctness_regression_test`) |
| Skipped Rust tests (`#[ignore]`) | **22** | 16 prior + the 6 new open-defect reproducers (#592, #578) |
| Skipped end-to-end programs (`CI-SKIP`) | **32** of 163 `TestPrograms/*.wfl` | mostly web-server programs needing an HTTP client (run by `run_web_tests`); the rest are #555-tracked unimplemented features / docs parse errors |
| Compiler / Clippy warnings | **0 permitted (gated)** | CI gate is `cargo clippy --all-targets --all-features -- -D warnings`; one pre-existing `deprecated` rustc warning is observable in `src/logging.rs` under a plain `cargo check` and should be cleaned up separately |
| Line coverage | **not instrumented** | no coverage tool wired (tarpaulin/llvm-cov absent); establishing a coverage baseline + CI report is Testing-dimension follow-up |
| Fuzz sustained-run duration | **0 (targets established, not yet run)** | sustained run + corpus retention is Phase 3 |
| Known crashes / hangs | **none known** | the 2 open High defects (#592, #578) are wrong-result / parse-error / silent-concat, not crashes or hangs; recursion overflow is now a clean `ExecutionBudget` error, not SIGABRT |

**Skip justification.** Of the 32 `CI-SKIP` programs, the large majority start a
web server and need an HTTP client to drive them (covered by the `run_web_tests`
scripts); the remainder are `#555`-tracked unimplemented features (session/CSRF,
direct-index) or `keyword_reference` docs examples with pre-existing parse errors
(the docs-examples-in-CI gate). Every skip carries a first-line reason.

## 5. Supported platforms & support boundaries

New reference `Docs/reference/supported-platforms.md` defines a three-tier model
grounded in what CI actually exercises:

- **Tier 1 (supported, CI-tested):** Linux `x86_64` (glibc) and Windows
  `x86_64` — both run the integration + `TestPrograms` matrix; Windows also gets
  the MSI + installer smoke test.
- **Tier 2 (best-effort, not in CI):** macOS (x86_64/Apple Silicon), musl/other
  Linux, other 64-bit Unix.
- **Unsupported:** 32-bit targets (the interpreter assumes a 64-bit address
  space and runs on a 1 GiB call-stack thread).

It also pins the toolchain (stable, MSRV 1.88, edition 2024), runtime
requirements (Tokio, FS, optional network, the `ExecutionBudget` ceilings), and
the boundary of "supported" (no untrusted-code sandbox; aspirational syntax
excluded). Linked from `Docs/README.md` and `SECURITY.md`; the stale `SECURITY.md`
version-support row was refreshed to `26.7.x` in the same change (docs-honesty).

---

## Phase 1 exit-gate read

> **Exit gate:** *No known production-readiness risk is untracked.*

- No open **Critical** issue; the 2 open **High** correctness items (#592, #578)
  are tracked with reproductions **and** regression tests.
- ExecutionBudget is finished, integrated, and test-covered.
- Every known correctness defect has an end-to-end regression test (passing
  guard if fixed; ignored repro if open).
- Fuzz targets exist for all four required surfaces.
- Baseline metrics are recorded; supported platforms and boundaries are defined.

Remaining Phase 1 → Phase 2/3 hand-offs (all tracked): the parser/analyzer/
type-checker/runtime **consistency suite**, wiring **docs examples into CI**, the
**sustained fuzz run** + corpus retention, per-limit **adversarial tests**, and
**coverage instrumentation**.
