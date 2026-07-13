# Dev Diary — 2026-07-13: Issue #610 Phase 1 — Initial Scorecard Baseline

## Context

Second Phase 1 task of the production-readiness tracker (#610):

> - [ ] Record initial scorecard values with links to evidence

This entry records the **baseline** weighted scorecard. Each area is scored
0–10 against its definition-of-done, multiplied by its weight, and summed. The
target for release is **≥ 8.0**; this baseline establishes the starting point,
not the goal. Scores are deliberately conservative and evidence-backed —
consistent with `SECURITY.md`'s "Alpha Software" self-designation and the
project's "validate, don't assert" policy.

**Measured against:** WFL **26.7.36** (`fc21f2f`), Linux, `cargo build --release`
+ `cargo test --all`.

## Baseline scorecard — 2026-07-13

| Area | Weight | Score | Weighted | Evidence & rationale |
|---|---:|---:|---:|---|
| Correctness | 25% | 6.0 | 1.50 | `cargo test --all`: **1477 passed / 0 failed / 16 ignored** (94 suites). Open-issue inventory: **0 open critical**, **2 open high** correctness defects (#592, #578); 10 correctness defects fixed and repro-verified this cycle (#557/#566/#567/#569/#571/#580/#582/#583/#588/#590). Runtime is correct even where former static diagnostics were wrong (those false positives are now fixed). Held below 8 by the two open high defects and the absence of a formal parser/analyzer/type-checker/runtime **consistency suite** (a mandatory gate). |
| Security | 20% | 5.5 | 1.10 | Shared **ExecutionBudget complete and integrated** (`src/exec/budget.rs`, PR #609) across lexer/parser/analyzer/pattern-VM/web/module loading. Recent hardening: subprocess policy (#608), Phase 0 concurrency (#607), password KDFs (#594), auth/session crypto + RNG lint (#595). `SECURITY.md`: private reporting + 48h SLA. **Gaps:** Dependabot reports **5 dependency alerts (1 high, 1 moderate, 3 low)** on the default branch; adversarial limit tests, supply-chain audit policy, and the security re-audit are all Phase 3. |
| Reliability | 15% | 5.0 | 0.75 | ExecutionBudget provides ceilings for wall-clock, ops, recursion, import depth, pattern transitions/states, source size, HTTP body, and WebSocket capacity — so uncontrolled hangs/exhaustion are mitigated **by design**. CI asserts `panic=abort` is rejected (unwinding preserved). **Gaps:** no fuzzing exists; no adversarial/boundary tests per limit; panic/crash/hang classification not started (all Phase 3). Fail-safe behavior is engineered but not yet *demonstrated* under adversarial load. |
| Testing | 15% | 6.5 | 0.975 | **1477 unit/integration tests pass.** `.github/workflows/ci.yml`: cross-platform integration matrix (**ubuntu + windows**), live **PostgreSQL + MariaDB** DB tests, and WFL-program runs on both OSes; `nightly.yml` runs a **Windows installer smoke test**. 71 `tests/*.rs`, ~129 top-level TestPrograms. **Gaps:** no sustained fuzzing; docs examples not executed in CI; **32 `CI-SKIP` programs** without a per-skip justification record. |
| Compatibility | 10% | 6.0 | 0.60 | `GOVERNANCE.md` §3.1 "Backward compatibility is sacred" + §3.2 No-Unlearning invariant + §2.2 decision authority ("must satisfy backward-compatibility rules") — a **breaking-change policy exists**. Supported behavior is documented and exercised by TestPrograms. **Gaps:** no test suite tagged specifically as a compatibility guard; the supported-language specification is a Phase 4 deliverable; aspirational-syntax marking is still in progress (#571/#578). |
| Operations | 5% | 5.0 | 0.25 | `nightly.yml` builds a versioned **MSI via `cargo-wix`**, uploads artifacts, and **smoke-tests the installer**; `versioning.yml` automates version bumps; `Docs/02-getting-started/installation.md` documents install. **Gaps:** no **checksums** published for artifacts (artifacts "verifiable" gate unmet); no upgrade/rollback/known-limitations docs (Phase 4). |
| Documentation | 5% | 5.0 | 0.25 | Extensive `Docs/` (6 sections + reference/guides); validation tooling exists (`scripts/validate_docs_examples.py`, `scripts/test_docs_code_blocks.py`, `TestPrograms/docs_examples/_meta/manifest.json`). **Gap (mandatory gate):** docs examples are **not executed in any CI workflow** — the harness is local-only. "Supported examples execute in CI" is currently unmet. |
| Maintenance | 5% | 7.0 | 0.35 | Full governance suite at repo root: `GOVERNANCE.md` (roles, ownership, decision authority, release lifecycle §3.7), `SECURITY.md` (reporting + supported versions + SLA), `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `AI_POLICY.md`. **Gap:** **supported platforms / support boundaries are not explicitly documented** (separate open Phase 1 task). |
| **Weighted total** | **100%** | | **≈ 5.78** | Rounded **5.8 / 10** (target ≥ 8.0). |

### Weighted math

```
Correctness   6.0 × 0.25 = 1.500
Security      5.5 × 0.20 = 1.100
Reliability   5.0 × 0.15 = 0.750
Testing       6.5 × 0.15 = 0.975
Compatibility 6.0 × 0.10 = 0.600
Operations    5.0 × 0.05 = 0.250
Documentation 5.0 × 0.05 = 0.250
Maintenance   7.0 × 0.05 = 0.350
                          ------
Weighted total          = 5.775  ≈ 5.8 / 10
```

## Biggest gaps to 8.0 (highest-leverage first)

1. **Correctness (25% weight):** resolve the two open high defects (#592, #578)
   and stand up the parser/analyzer/type-checker/runtime consistency suite.
2. **Security (20%):** clear the Dependabot high, add adversarial tests for every
   ExecutionBudget limit, and add a supply-chain audit policy (Phase 3).
3. **Reliability (15%):** establish fuzz targets (lexer/parser/pattern/module —
   an open Phase 1 task) and boundary tests; classify every crash/hang.
4. **Testing (15%):** wire docs-example validation into CI, add sustained
   fuzzing, and justify/record the 32 skipped programs.

## Compatibility / resource impact

None. Documentation + tracker hygiene only; no code, runtime, or language
behavior was modified. Scores are a point-in-time assessment to be re-scored
monthly through October per the tracker's review cadence.
