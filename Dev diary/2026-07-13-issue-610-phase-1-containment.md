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

All work was verified against a `cargo build --release` binary on Linux; the
branch is rebased onto **WFL 26.7.37** (current `main`), whose source is
identical to 26.7.36 apart from the version bump and #613's docs, so the repro
observations carry over unchanged.

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

**Tests:** `tests/execution_budget_test.rs` (32 tests) covers config parsing of
every `.wflcfg` budget key plus end-to-end enforcement (recursion → clean
catchable error instead of SIGABRT; oversized source refused before running).

**Verdict:** the mandatory gate *"Shared runtime execution budget covers parsing,
evaluation, pattern matching, web handling, and module loading"* is **met**. The
adjacent gate — *adversarial* tests for each limit — is explicitly **Phase 3**
and is not claimed here.

## 2. Known correctness defects → regression tests (per-issue, with a representative #578 sample)

New suite `tests/phase1_correctness_regression_test.rs` maps every inventoried
correctness **issue** to regression coverage. **Scope, stated honestly:** this is
*per-issue* coverage, **not** a test for every conceivable sub-defect — **#578 is
an umbrella** (~26 checkboxes), and only its reproducible confirmed
functional/silent-wrong-result bugs are encoded; its remaining sub-items are
tracked on the issue, not here. So the Phase 1 task *"convert every known
correctness defect into an end-to-end regression test"* is satisfied at the
issue level with a representative #578 sample — it does **not** claim exhaustive
coverage of #578's tail. Two halves:

- **Fixed defects → passing guards.** New binary-level guards for **#569**
  (action-call result is `Text`, not `Nothing`), **#571** (precedence, both
  `divided by` and `/` division, `modulo`, `is between`), and **#590** (a
  CLI-level end-to-end guard for the self-recursive indexed-result case, added on
  review — complementing the in-process `recursive_action_return_type_test.rs`).
  Defects already covered elsewhere (#582/#557/#566/#567/#583/#588 in
  `github_issues_batch_test.rs`, #580 in `include_of_form_resolution_test.rs`)
  are indexed in the file's module doc rather than duplicated. **#573 is fixed**
  (binary read/write + MIME shipped in #574; guarded by `web_server_binary_test.rs`
  + `binary_io_test.rs` + `binary_file_and_mime_test.wfl`; the issue is open only
  pending a close click).
- **Open defects → `#[ignore]`d desired-behaviour tests** that reproduce the bug
  today (they fail under `--ignored`) and flip green when the fix lands:
  - **#592** — bare zero-arg included action fatal at top level **and** in an
    action body (parameterized).
  - **#578** (reproducible confirmed bugs) — `list files … with pattern` returns
    0; `one or more` quantifier matches per-char (16 vs 4 words); `repeat N times`
    is a parse error; `Number plus Text` silently concatenates; no text→number
    conversion; `format_date` echoes friendly patterns; and `with`-form action
    calls silently concatenate.

CI stays green (open defects are `#[ignore]`d, not failing).

Re-verification against the release binary corrected the inventory in several
places: #578's *`X ends with Y` misparse*, its *`add` to `List<Any>` drops in
`--test` mode*, and its *`double of 5 minus 1` → Nothing* inference items **no
longer reproduce** on the current build (fixed), so they are not encoded as open
defects; #573 is **fixed** (above), not the open limitation an earlier draft
listed.

## 3. Fuzz targets — three of four surfaces established; module loading still open

New standalone cargo-fuzz workspace under `fuzz/` (kept out of the stable root
build via its own `[workspace]` and root `exclude = ["fuzz"]`):

| Target | Surface |
|---|---|
| `fuzz_lexer` | `lex_wfl_with_positions_checked` |
| `fuzz_parser` | lex → `Parser::parse` |
| `fuzz_pattern` | `pattern\0haystack` **pair** → `create pattern` parse → `CompiledPattern::compile` → `find_all(haystack)` (ReDoS surface: the fuzzer controls both the pattern and the input it runs against) |
| `fuzz_frontend` | compiler **frontend** on arbitrary source: **checked** lex → parse → include/load-module detection → `Analyzer::analyze` → `TypeChecker::check_types` |

Each target's invariant: no arbitrary input may panic, overflow the stack, or
hang. Tracked seed inputs live in `fuzz/seeds/<target>/`; the live corpus and
artifacts are gitignored (the corpus dir is created from the seeds on first run —
see `fuzz/README.md`). The standalone `fuzz/Cargo.lock` **is** committed for
reproducible builds.

Targets **type-check cleanly on stable** (`cargo check --manifest-path
fuzz/Cargo.toml`), and a new `fuzz-check` CI job runs exactly that on every PR so
API drift can't silently break the excluded fuzz crate. The *sustained run* (with
corpus retention) is deliberately **Phase 3** work; no run duration is claimed at
baseline (see metrics below).

**Module-loading fuzzing is NOT done (open Phase 1 item).** Phase 1 lists four
required surfaces: lexer, parser, pattern engine, **and module loading**. The
first three are covered. `fuzz_frontend` fuzzes the static frontend that a
module's *content* passes through, but it deliberately does **not** invoke the
interpreter's `LoadModuleStatement` / `IncludeStatement` paths — so filesystem
path resolution/canonicalization, bounded reads, cross-file circular/import-depth
enforcement, parent-scope construction, and module execution are **uncovered**.
Fuzzing the real loader means driving the async interpreter against on-disk
modules, and doing that *safely* is the blocker: executing fuzzer-generated WFL
would also exercise subprocess spawning, networking, the web server, and
filesystem writes. A sandboxed module-loading harness (benign module bodies +
fuzzed include-graph structure, or an execution-disabled load path) is tracked as
remaining Phase 1 work. The renamed target (`fuzz_frontend`, formerly the
misleadingly-named `fuzz_module_loading`) no longer claims to fuzz module
loading.

## 4. Baseline metrics (2026-07-13; rebased onto WFL 26.7.37, Linux)

**Methodology (corrected after the round-2 review).** The head-SHA CI run on
`f627b4e` was **green**, but its `cargo test` command tested the **root package
only** — it did **not** run `wflpkg`, so it was never a full-workspace baseline.
Observed component numbers on that run (scope-labeled):

| Scope | Command (on `f627b4e`) | passed | failed | ignored | result suites |
|---|---|---:|---:|---:|---:|
| root package | `cargo test` | 1206 | 0 | 24 | 76 |
| `wfl-lsp` | `cargo test -p wfl-lsp` | 69 | 0 | — | — |
| `wflpkg` | *(not run by CI)* | 204† | 0 | — | — |
| **workspace total** | (sum) | **≈1479** | **0** | **24+** | — |

† `wflpkg`'s 204 tests were not executed by CI on `f627b4e`; the count is from the
package, not that CI run. This is exactly why the earlier single "≈1479 aggregate"
was **derived, not measured**.

**Fix applied here:** `ci.yml`'s "Run Tests" step now runs `cargo test
--workspace` (was `cargo test`), so from this commit forward CI executes the
**whole workspace** — root + `wflpkg` + `wfl-lsp` — in one lane and reports a true
aggregate. (A full local `cargo test --all` on this head was also attempted to
confirm the number directly; it exhausted the sandbox's fixed per-session disk
allowance mid-compile — `No space left on device`, an environment limit, not a
failure — so the authoritative combined number will come from the next
`--workspace` CI run, to be recorded here with the workflow link.)

**This change's delta** to the root-package suite: the new
`phase1_correctness_regression_test` adds **3 passing** guards (#569, #571, #590)
and **9 `#[ignore]`d** reproducers (2×#592, 7×#578). The table below is therefore
a scope-labeled estimate pending the `--workspace` CI run.

| Metric | Baseline | Source / notes |
|---|---|---|
| Rust test count (workspace) | **≈1480 passed / 0 failed / 25 ignored** (scope-labeled estimate; `--workspace` CI run pending) | root ≈1207/0/25 (observed 1206/0/24 on `f627b4e` + this change's +1 passing #590, +1 ignored) **+** `wfl-lsp` 69 **+** `wflpkg` 204 |
| Result suites (root package) | **76** (observed on `f627b4e`); `wfl-lsp` + `wflpkg` add their own binaries | the earlier "95" figure was mis-scoped; 76 is the observed root-package result-suite count |
| Skipped Rust tests (`#[ignore]`, root) | **25** | 24 observed on `f627b4e` + 1 new (`with`-form #578 reproducer); #590 is a *passing* CLI guard, not ignored |
| Skipped end-to-end programs (`CI-SKIP`) | **32** of 163 `TestPrograms/*.wfl` | see skip justification below |
| Compiler / Clippy warnings | **0 (gate: `clippy --all-targets --all-features -- -D warnings`)** | the one pre-existing `deprecated` rustc warning in `src/logging.rs` is **fixed in this change** (`parse` → `parse_borrowed::<2>`) |
| Line coverage | **not instrumented** | no coverage tool wired (tarpaulin/llvm-cov absent); a coverage baseline + CI report is a Testing-dimension follow-up |
| Fuzz sustained-run duration | **0 s** (targets established + a `fuzz-check` compile job on PRs; not yet run for duration) | sustained run + corpus retention is Phase 3 |
| Known crashes / hangs | **none reproducible** | the 2 open High defects (#592, #578) reproduce as wrong-result / parse-error / silent-concat, not crashes or hangs; #578's listed nested-`for each` crash did **not** reproduce on the current build; recursion overflow is now a clean `ExecutionBudget` error, not SIGABRT |

**Skip justification.** Of the 32 `CI-SKIP` programs, 15 start a web server and
need an HTTP client to drive them. The `run_web_tests` scripts exist to drive
these, **but no CI workflow currently invokes them**, so those paths are skipped
in CI's `TestPrograms` runner and lack automated CI coverage today (tracked;
wiring `run_web_tests` into CI is a Testing follow-up). The remainder are
`#555`-tracked unimplemented features (session/CSRF, direct-index) or
`keyword_reference` docs examples with pre-existing parse errors (the
docs-examples-in-CI gate). Every skip carries a first-line reason.

## 5. Supported platforms & support boundaries

New reference `Docs/reference/supported-platforms.md` defines a three-tier model
grounded in what CI actually exercises:

- **Tier 1 (supported, CI-tested):** Linux `x86_64` (glibc) and Windows
  `x86_64` — both run the integration + `TestPrograms` matrix on every PR. The
  doc includes a **per-platform PR CI coverage table** because coverage is *not*
  symmetric: the full `cargo test` unit/LSP/clippy/DB suite runs on **Linux
  only**; Windows PR CI runs the integration + `TestPrograms` lanes. The MSI +
  installer smoke test is **nightly/post-merge**, not a PR gate.
- **Tier 2 (best-effort, not in CI):** macOS (x86_64/Apple Silicon), musl/other
  Linux, other 64-bit Unix.
- **Unsupported:** 32-bit targets (the interpreter assumes a 64-bit address
  space and runs on a 1 GiB call-stack thread).

It also pins the toolchain (stable channel; MSRV 1.88 **declared but not
gate-tested** — CI runs stable; edition 2024), runtime requirements (Tokio, FS,
optional network, the `ExecutionBudget` ceilings), and the boundary of
"supported" (no untrusted-code sandbox; docs-in-CI still an open gate;
aspirational syntax excluded). Linked from `Docs/README.md` and `SECURITY.md`;
the stale `SECURITY.md` version-support row was refreshed to `26.7.x`, its footer
version to `26.7.37`, and its "no cryptographic functions" / `max_nesting_depth`
recursion claims corrected in the same change (docs-honesty).

---

## Phase 1 exit-gate read

> **Exit gate:** *No known production-readiness risk is untracked.*

- No open **Critical** issue; the 2 open **High** correctness items (#592, #578)
  are tracked with reproductions **and** regression tests.
- ExecutionBudget is finished, integrated, and test-covered.
- Every inventoried correctness **issue** has regression coverage (passing guard
  if fixed; ignored repro if open) — with a **representative**, not exhaustive,
  sample of #578's umbrella sub-items (the rest tracked on the issue).
- Fuzz targets cover **three of the four** required surfaces (lexer, parser,
  pattern engine). **Module-loading fuzzing is not done** — see §3; it is an
  explicitly open Phase 1 item.
- Baseline metrics are recorded (scope-labeled; the authoritative full-workspace
  aggregate comes from the now-`--workspace` CI run); supported platforms and
  boundaries are defined.

**Phase 1 is therefore not fully complete.** Explicitly open Phase 1 items,
carried forward and tracked: (1) a **module-loading fuzz target** (safe async
harness); (2) the record of the **`--workspace` CI aggregate** with a workflow
link; (3) exhaustive per-item **#578 classification**. Larger hand-offs to Phase
2/3 (also tracked): the parser/analyzer/type-checker/runtime **consistency
suite**, **docs examples into CI**, the **sustained fuzz run** + corpus
retention, per-limit **adversarial tests**, and **coverage instrumentation**.
