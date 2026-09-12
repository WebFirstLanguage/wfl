# WFL codebase audit

**Audited:** 2026-09-08 • **Revision:** `db11db976792c9ec399966a2092d814e57fd64ca` • **Version:** 26.9.3

**Assessment:** the codebase builds and has substantial regression coverage, but this revision does not pass its full Windows test gate and has confirmed source-corruption, transaction-integrity, language-correctness, and editor-integration defects. Treat the high-priority findings below as release blockers for the affected features. This audit makes no production changes and does not certify the project as secure or release-ready.

The audit covers the repository's compiler/runtime pipeline, standard library, pattern engine, REPL, configuration, CLI, LSP/MCP, VS Code extension, scripts, tests, dependency locks, documentation, CI and packaging. It combines source review, fresh builds, the full workspace test run, real CLI/database/socket/protocol probes, and dependency scanning. The original checkout was clean; old August binaries were replaced by builds of the current revision before validating findings.

## Prioritized findings

**P1:** fix promptly; material loss of program/data integrity or a core feature unusable. **P2:** actionable correctness, security, integration, or verification defect. **P3:** lower-impact configuration defect. These are engineering priorities, not CVSS scores. The 24 entries below group related failure modes; detailed component reports retain their separate causes and reproductions.

| ID | Priority | Finding and observable impact | Primary location / detailed evidence |
|---|---|---|---|
| A01 | P1 | **Core fixer corrupts source.** It overwrites valid files with Debug AST text even after reparsing fails; separately, it loses string escapes, constant qualifiers, and distinct identifier bindings. Parse success alone does not establish semantic preservation. | [fixer/mod.rs:79](G:/repos/wfl/src/fixer/mod.rs:79), [frontend F1–F4](G:/repos/wfl/target/reports/audit/frontend.md) |
| A02 | P1 | **Builtin editor formatter changes literal data.** Formatting a URL adds spaces inside it; repeated spaces inside strings are collapsed. Automatic formatting can change behavior on save. | [base-formatter.ts:146](G:/repos/wfl/vscode-extension/src/formatting/base-formatter.ts:146), [tooling T1](G:/repos/wfl/target/reports/audit/tooling.md) |
| A03 | P1 | **Extension rejects the shipped language server.** Version recognizers do not match either executable's real version output. LSP activation fails; configured CLI paths also fail recognition, although a PATH fallback can still discover the CLI. | [extension.ts:118](G:/repos/wfl/vscode-extension/src/extension.ts:118), [tooling T2](G:/repos/wfl/target/reports/audit/tooling.md) |
| A04 | P1 | **Transaction filtering permits early commits.** A leading statement separator bypasses the keyword guard. A real SQLite write survives a later failure in its enclosing transaction; ordinary COMMIT is rejected and rolls back correctly. | [database.rs:211](G:/repos/wfl/src/interpreter/database.rs:211), [runtime R1](G:/repos/wfl/target/reports/audit/runtime-security.md) |
| A05 | P2 | **Locked dependencies carry an active advisory.** Both h2 versions are flagged by one RustSec advisory; there are also unsoundness, maintenance, and yanked-version warnings. Exposure is conditional, not established by a live exploit. | [Cargo.lock:1167](G:/repos/wfl/Cargo.lock:1167), [dependency assessment](G:/repos/wfl/target/reports/audit/runtime-security.md) |
| A06 | P2 | **Duration waits exceed the execution budget.** A three-second wait under a one-second deadline completes successfully after approximately three seconds when it is the last statement. | [interpreter/mod.rs:13448](G:/repos/wfl/src/interpreter/mod.rs:13448), [runtime R2](G:/repos/wfl/target/reports/audit/runtime-security.md) |
| A07 | P2 | **Positive lookahead branches target the wrong instructions.** Copying an assertion into a zero-based subprogram does not relocate absolute jumps. Valid input is rejected and invalid input accepted. | [pattern/vm.rs:670](G:/repos/wfl/src/pattern/vm.rs:670), [frontend F5](G:/repos/wfl/target/reports/audit/frontend.md) |
| A08 | P2 | **Negative lookahead consumes the following suffix while deciding its result.** An assertion that `a` must not follow nevertheless matches `a` when followed by an `a` suffix. | [pattern/vm.rs:747](G:/repos/wfl/src/pattern/vm.rs:747), [frontend F6](G:/repos/wfl/target/reports/audit/frontend.md) |
| A09 | P2 | **Greedy quantifiers return shortest matches.** Finding digit runs in `123` returns three individual matches; zero-or-more can return empty immediately. | [pattern/vm.rs:423](G:/repos/wfl/src/pattern/vm.rs:423), [frontend F7](G:/repos/wfl/target/reports/audit/frontend.md) |
| A10 | P2 | **Lexical errors allow successful execution.** An invalid token is printed as an error then discarded; the remaining source executes with exit zero. | [lexer/mod.rs:346](G:/repos/wfl/src/lexer/mod.rs:346), [frontend F8](G:/repos/wfl/target/reports/audit/frontend.md) |
| A11 | P2 | **Docs validation claims typechecking that never ran.** The validator mistakes semantic analysis for typechecking and unconditionally records lint completion. A known type-invalid example passes layers 1–4. | [validate_docs_examples.py:278](G:/repos/wfl/scripts/validate_docs_examples.py:278), [runtime R8](G:/repos/wfl/target/reports/audit/runtime-security.md) |
| A12 | P2 | **MCP lint reports clean without invoking the linter.** A CLI-confirmed naming violation and malformed source both return success with zero lint issues over the real MCP connection. | [mcp_server.rs:525](G:/repos/wfl/wfl-lsp/src/mcp_server.rs:525), [tooling T4](G:/repos/wfl/target/reports/audit/tooling.md) |
| A13 | P2 | **`unique` drops distinct nested values.** It keys on display strings; `["a, b"]` and `["a", "b"]` become indistinguishable despite unequal WFL values. | [list.rs:281](G:/repos/wfl/src/stdlib/list.rs:281), [runtime R3](G:/repos/wfl/target/reports/audit/runtime-security.md) |
| A14 | P2 | **Multipart payloads can be silently truncated.** The parser treats arbitrary embedded boundary-prefix text as framing and manufactures an extra part. | [web.rs:326](G:/repos/wfl/src/stdlib/web.rs:326), [runtime R4](G:/repos/wfl/target/reports/audit/runtime-security.md) |
| A15 | P2 | **Invalid date formats cause native panics.** `%Q` unwinds instead of returning the builtin's promised RuntimeError. | [time.rs:68](G:/repos/wfl/src/stdlib/time.rs:68), [runtime R5](G:/repos/wfl/target/reports/audit/runtime-security.md) |
| A16 | P2 | **Salt suffixes are silently ignored.** Salted WFLHASH uses only the first 16 bytes although the public domain-separation guidance places no such restriction. | [crypto.rs:317](G:/repos/wfl/src/stdlib/crypto.rs:317), [runtime R6](G:/repos/wfl/target/reports/audit/runtime-security.md) |
| A17 | P2 | **Crypto guidance incorrectly promises security from an outer hash.** SHA256(WFLHASH(input)) preserves any inner collision; it cannot inherit collision resistance independently of WFLHASH. | [crypto-module.md:76](G:/repos/wfl/Docs/05-standard-library/crypto-module.md:76), [runtime R7](G:/repos/wfl/target/reports/audit/runtime-security.md) |
| A18 | P2 | **Type-safety prose overstates execution guarantees.** Ordinary type diagnostics are advisory and earlier side effects still run; the specifically forbidden Number-plus-Text example actually succeeds. | [variables-and-types.md:195](G:/repos/wfl/Docs/03-language-basics/variables-and-types.md:195), [runtime R9](G:/repos/wfl/target/reports/audit/runtime-security.md) |
| A19 | P2 | **CLI editor formatter uses an invalid command contract.** Its current arguments exit 2. Once that is repaired, independently reproduced dirty-buffer and diff-reconstruction bugs must also be fixed to avoid losing source. | [wfl-formatter.ts:151](G:/repos/wfl/vscode-extension/src/formatting/wfl-formatter.ts:151), [tooling T3](G:/repos/wfl/target/reports/audit/tooling.md) |
| A20 | P2 | **LSP diagnostic ranges use UTF-8 byte columns under UTF-16 defaults.** A real emoji-containing line publishes column 17 instead of 15. | [core.rs:214](G:/repos/wfl/wfl-lsp/src/core.rs:214), [tooling T5](G:/repos/wfl/target/reports/audit/tooling.md) |
| A21 | P2 | **MSI post-install actions reference absent scripts.** The authored payload omits the invoked scripts and ignores their failures. Static XML/dataflow finding; no machine-wide installer run. | [main.wxs:262](G:/repos/wfl/wix/main.wxs:262), [tooling T6](G:/repos/wfl/target/reports/audit/tooling.md) |
| A22 | P2 | **Nightly version overrides can mislabel artifacts.** The override changes filenames/status without updating compiled or packaged version sources. Static workflow finding; no release dispatched. | [nightly.yml:101](G:/repos/wfl/.github/workflows/nightly.yml:101), [tooling T7](G:/repos/wfl/target/reports/audit/tooling.md) |
| A23 | P2 | **File-I/O tests leak root files and under-assert error behavior.** Cleanup is skipped on assertion failure; several negative tests accept either success or caught error. Actual post-test hygiene fails independently of the unresolved timeout cause. | [file_io_concurrent_test.rs:13](G:/repos/wfl/tests/file_io_concurrent_test.rs:13), [test triage](G:/repos/wfl/target/reports/audit/test-triage.md) |
| A24 | P3 | **Configured linter limits are disconnected.** A maximum line length of 20 has no effect because the rule uses 100; nesting has the same hardcoded-limit issue. | [linter/mod.rs:49](G:/repos/wfl/src/linter/mod.rs:49), [frontend F9](G:/repos/wfl/target/reports/audit/frontend.md) |

All P1 entries have dynamic evidence against current code. A21/A22 are explicitly static. A17 is a mathematical contract error, not evidence of a discovered cryptographic collision. A19's downstream data-loss defects are latent behind the currently failing CLI invocation. Component reports contain exact triggers, expected/actual results, fixes, and regression-test recommendations.

## Verification results

Host: Windows x86-64/MSVC, Rust 1.98.1, Cargo 1.98.1. Build space was ample; no cargo clean was needed. Locked dependencies were fetched where absent. Python came from the bundled runtime. A duplicate case-variant PATH in the host environment initially prevented the PowerShell web runner from launching; rerunning in a child with one PATH key resolved that environmental issue.

| Check | Result | Evidence |
|---|---|---|
| `cargo build --release --locked` | Pass, fresh release | [build log](G:/repos/wfl/target/reports/audit/build-release-network.log) |
| `cargo fmt --all -- --check` | Pass | [format log](G:/repos/wfl/target/reports/audit/fmt.log) |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Pass; this documented command covers the root default package | [Clippy log](G:/repos/wfl/target/reports/audit/clippy-network.log) |
| Expanded Clippy command with `--workspace` | **Fail** in LSP test targets: unused variables/dead code and Clippy diagnostics that the root-only command does not visit | [workspace Clippy log](G:/repos/wfl/target/reports/audit/clippy-workspace.log) |
| Full `cargo test --workspace --locked --no-fail-fast` | **Fail: 2,132 passed, 8 failed, 27 ignored**, including doctests, across 161 result summaries | [complete test log](G:/repos/wfl/target/reports/audit/cargo-test-complete.log) |
| WFL end-to-end selection from `run_integration_tests.ps1` | **142 passed, 24 declared skips, zero failures** | [program results](G:/repos/wfl/target/reports/audit/wfl-programs.json) |
| PowerShell web runner in normalized child environment | 2/2 HTTP scenarios pass; TLS scenario skipped for missing OpenSSL | [web log](G:/repos/wfl/target/reports/audit/web-tests-clean.log) |
| Docs validator, `--force --ci --report` | 34/34 reported pass; A11 limits what that proves | [docs log](G:/repos/wfl/target/reports/audit/docs-validation.log) |
| `python -m unittest discover -s tests/tooling -v` | 33/33 pass | [tooling tests](G:/repos/wfl/target/reports/audit/tooling-tests.log) |
| Standalone `scripts/test_bump_version.py` | 2 errors; stale mocks/version expectations | [version tests](G:/repos/wfl/target/reports/audit/version-tests.log) |
| `cargo check --locked --manifest-path fuzz/Cargo.toml` | Pass; compilation only, no fuzz campaign | [fuzz build](G:/repos/wfl/target/reports/audit/fuzz-check-network.log) |
| Root and fuzz dependency audits | Fail: one h2 advisory affects two versions in both locks | [root audit](G:/repos/wfl/target/reports/audit/cargo-audit.json), [fuzz audit](G:/repos/wfl/target/reports/audit/cargo-audit-fuzz.json) |
| Static repository hygiene | Pass | [static log](G:/repos/wfl/target/reports/audit/hygiene-static.log) |
| Working-tree hygiene after tests | Fail from test-created files; all observed residue preserved under target and final checker passes | [failure log](G:/repos/wfl/target/reports/audit/hygiene-after-tests.log), [final log](G:/repos/wfl/target/reports/audit/hygiene-final.log) |
| Credential signature scan | No matches in 844 tracked UTF-8 files; selected patterns only | [scan scope/results](G:/repos/wfl/target/reports/audit/credential-signatures.json) |
| Lexer/parser benchmark smoke | All four benchmarks complete; timing estimates below | [benchmark log](G:/repos/wfl/target/reports/audit/benchmarks.log) |

The ordinary workspace run stopped at the first failing target. The no-fail-fast run retained failures and continued to cover all remaining targets; it was not used to turn a failure green. Because the provided integration wrapper stops when Rust integration tests fail, its WFL selection was executed separately with the **same exclusions, expected-failure rules, test-mode detection, and 30-second per-program timeout**. [wfl_programs.py](G:/repos/wfl/target/reports/audit/wfl_programs.py) reads the skip/expected-failure arrays from the current PowerShell script. This result is not a claim that the combined presubmit command passes. No existing skips or assertions were changed.

The final eight failures are three tests in `file_io_concurrent_test` and five in `file_io_error_handling_test`. They expire the test helpers' 10-second/5-second outer deadlines. The failure set changed between runs, while neighboring file execution/performance tests passed. The evidence does not establish whether the root cause is a runtime race, filesystem synchronization/storage behavior, scheduling, or inappropriate test timing assumptions. Repeated `sync_all` calls are an investigation lead, not a proven cause. Increasing timeouts or rerunning until green would not resolve this finding. [Detailed triage](G:/repos/wfl/target/reports/audit/test-triage.md).

## Dependency risk

The live RustSec database scan used commit `bf25f6575a93a35f30796c65c0ed91bee7fa19fd` dated 2026-09-08 and examined 432 root dependencies. [RUSTSEC-2026-0258](https://rustsec.org/advisories/RUSTSEC-2026-0258.html) affects the locked h2 0.3.27 and 0.4.15; the upstream patched version is 0.4.16 and its stated severity is Low. The older Warp/Hyper dependency chain requires an appropriate migration or supported fix. Actual abuse depends on HTTP/2 peers and body-draining behavior; the audit did not exercise a malicious peer.

Other warnings: [event-listener 5.4.1 unsoundness](https://rustsec.org/advisories/RUSTSEC-2026-0221.html), patched in 5.4.2; [rustls-pemfile 2.2.0 unmaintained](https://rustsec.org/advisories/RUSTSEC-2025-0134.html); and root-lock yanked chacha20 0.10.1/spin 0.9.8. WFL reachability of the event-listener misuse was not established, and yanking is not itself a vulnerability. Both lockfiles need coordinated review; an automated advisory gate is absent from the inspected presubmit workflow.

## Architecture, performance, and test-system observations

The tracked inventory contains 846 files: 267 Rust files, 265 WFL programs/fixtures, and nine TypeScript files. The lexer → parser → analyzer → typechecker → interpreter separation is recognizable, but semantic knowledge is duplicated across AST printing, CLI, REPL, LSP/MCP, docs validation, and editor code. The confirmed disagreements at those boundaries are higher-value repair targets than stylistic refactoring.

Three modules dominate the core: interpreter/mod.rs is 19,400 lines, typechecker/mod.rs 12,455, analyzer/mod.rs 6,201, including embedded tests. Extracting independently tested transport, lifecycle, printing, and validation services would reduce cross-feature reasoning cost. Do this after behavior-preservation tests exist; large files alone do not prove a defect.

The benchmark smoke run used ten samples, one-second warmup, and one-second measurement per benchmark. Criterion's middle estimates were approximately 0.994 ms for large strings, 0.962 ms for no-string input, 0.957 ms for booleans, and 8.56 µs for the parser fixture. These are local microbenchmark observations with outliers, no controlled cross-revision baseline, and no application latency/SLO claim. No performance regression is inferred. Existing policy already acknowledges absent automated coverage, scheduled extended fuzzing, and formal performance budgets.

Verification weaknesses extend beyond the eight failures: lookahead tests in an undeclared obsolete module do not run; several LSP tests simulate behavior instead of using a server; extension tests accept absence/timeouts; no extension build/test gate runs in presubmit; some file-error tests do not prove their negative path. The documented Clippy command passes its root package, but adding `--workspace` fails in LSP test targets, exposing another difference between the command's scope and the overall project. Real wire probes in this audit exposed failures those tests missed. See the component reports for exact examples. The existing large suite remains useful, but its count should not substitute for boundary assertions.

## Recommended repair order

1. Protect source and data: make the core fixer fail without changing input when rendering/validation fails, preserve literal and binding semantics, repair editor formatting, and close the transaction-control hole. Capture failing byte/AST/value/database assertions first.
2. Restore editor capability and verification honesty: align executable/version/CLI contracts; fix MCP/docs validation and UTF-16 ranges; add real shipped-binary extension acceptance tests. Repair all coupled formatter paths before enabling a formerly blocked path.
3. Repair runtime/language contracts: budget-aware waits, lookahead/greediness, fatal lexical errors, unique/multipart/date error handling, and salt/domain guidance. Respect backward compatibility, especially serialized hashes and advisory type-checking behavior.
4. Resolve dependency advisories, installer/version integrity, failing file-I/O tests, and failure-safe fixture cleanup. Preserve failure evidence until the cause and regression tests are established.
5. Add the missing sustained checks: meaningful negative paths, coverage measurement, scheduled fuzz/lifecycle campaigns, installer/extension gates, and stable performance baselines. Keep the already tracked policy gaps explicit.

Any future fixes to untrusted input, crypto, concurrency, lifecycle, or compatibility fall under the repository's R3 policy and need Red→Green evidence, relevant real-boundary/negative tests, independent review, and documentation in the same change. This audit itself is non-behavioral and deliberately contains no fixes.

## Scope limits and handoff

This is a repository-wide audit, not a line-by-line proof of all possible programs. Deep code paths were reviewed selectively and findings validated with small bounded cases. Unresolved static concerns include cyclic JSON/TOML conversion, structural parser/LSP limits, background-process lifecycle, and workload-specific KDF cancellation/capacity. They are recorded separately from confirmed defects in the runtime report.

Not performed: Linux/macOS execution, full VS Code Electron-host tests, machine-global MSI installation, release publication, GitHub branch-protection verification, network penetration testing, resource-exhaustion campaigns, long fuzz/soak runs, independent cryptanalysis, a full license/compliance audit, or historical/high-entropy secret scanning. No source or security findings were sent to public issue trackers. The TLS shell scenario was skipped; this is not a claim that the separately running Rust TLS tests were skipped too.

The final checkout is clean. Test-created residue is retained at [test-residue](G:/repos/wfl/target/test-artifacts/audit/test-residue); audit reports and reproductions are under `target/reports/audit/` and `target/test-artifacts/audit/`. These locations are ignored build-output roots and will be removed by a future cargo clean; preserve this report elsewhere if long-term retention is needed. No commit, PR, release, or production edit was made.

A separate review agent checked this report and the runtime evidence for overstated conclusions and found no material corrections. The final workspace-Clippy failure was subsequently added. The frontend report records the release binary hash used during its probes; later Cargo verification rebuilt the binary from the unchanged revision. Its final SHA-256 is `8DBA21E9010768CCAC548AAB37735AE17E3633BAA7D76773CB60EF7FCFDDB9FC`. Reproducible binary identity across different build invocations was not assessed.

Detailed handoff:

- [Frontend, patterns, linter, and fixer](G:/repos/wfl/target/reports/audit/frontend.md)
- [Runtime, data, security, and documentation](G:/repos/wfl/target/reports/audit/runtime-security.md)
- [LSP, MCP, extension, CI, and packaging](G:/repos/wfl/target/reports/audit/tooling.md)
- [Windows test-failure triage](G:/repos/wfl/target/reports/audit/test-triage.md)
- [Machine-readable totals and inventory](G:/repos/wfl/target/reports/audit/summary.json)
