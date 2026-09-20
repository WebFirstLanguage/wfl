# Subprocess lifecycle acceptance evidence

Risk: R3, process ownership, cancellation, public syntax and CLI exit behavior.
Provider: WFL. Consumer: Scriptorium's required WFL-only test runner and HTTP
integration driver. No application or Python test-driver workaround is used.

## Behavioral baseline before implementation

Base: `cb1dadaad96939a4450a6eb2b3a6a51678035b7f`.
Runtime: official Windows nightly `26.9.12`, with its bin directory first on
`PATH` so the parent and all child runtimes match. Date: 2026-09-20.

Command: `wfl --test TestPrograms/process/lifecycle.test.wfl`.
Observed: 3 tests, 0 passed, 3 failed, exit 1. All programs parsed and executed.

1. Absolute fixture source path still used the repository working directory;
   assertion expected the isolated fixture directory.
2. Numeric wait obtained child exit 0, but subsequent output retrieval raised
   `Invalid process ID`; final-output assertion found empty text.
3. Foreground execution demonstrated a real `Division by zero` stderr
   diagnostic. Background execution returned only its stdout marker; assertion
   for that diagnostic failed even though child exit 1 was observed.

These establish the behavioral requirements. The implementation will add an
explicit working-directory clause and full-result completion, preserving the
existing numeric wait's release behavior. Green tests will use the additive
API: retaining every legacy completion indefinitely would break bounded
ownership, while silently dropping retained output would hide failures.

Planned contract: direct argv, optional per-launch cwd, finite explicit wait
timeout, joined bounded stdout/stderr result and exit status, atomic release,
idempotent close, reliable kill/reap on errors, and clean explicit program exit
codes. Existing subprocess opt-in policy remains authoritative. Tests, fixture
generation and assertions are WFL; existing Rust harnesses may discover them.

Red is preserved by commit `93829ff9`. The former output-loss tests remain in
that commit; current acceptance uses the additive full-result API rather than
changing numeric wait semantics.

## Local Green and review (Windows, 2026-09-20)

The final release binary runs these WFL suites through the existing
TestPrograms discovery contract:

- `wfl --test TestPrograms/process/lifecycle.test.wfl`: 16 passed, 0 failed.
- `wfl --test TestPrograms/process/ownership.test.wfl`: 2 passed, 0 failed.
- `wfl --test TestPrograms/process/failure-cleanup.test.wfl`: 2 passed, 0 failed.
- Existing `TestPrograms/subprocess_comprehensive.wfl`: all eight groups
  completed successfully, including live-child kill and numeric completion.
- Existing `TestPrograms/exit_program_test.wfl`: exit 0 at the intended stop.

The split suites keep lifecycle and nested-child checks below the existing
per-program CI timeout. They exercise cwd and literal argv, stdout plus stderr,
bounded output, capacity denial/reuse, sibling result preservation, finite
timeouts, idempotent close, invalid exit/deadline values, program status through
finally, and descendant cleanup after timeout, success, runtime failure,
explicit status, and a failed WFL assertion. Fixtures use readiness markers
before testing descendants and write only beneath `target/test-artifacts`.

Existing cargo compatibility tests passed: `execution_budget_test` (41),
`subprocess_cleanup_test` (7), `subprocess_security_test` (19),
`subprocess_test` (13), and `typechecker_statement_operand_contract_test` (26):
106 total. The first attempt lacked the separately built release binary in the
legacy helper's expected location; after copying this worktree's own release
build to that location, all passed. No assertions were weakened.

`cargo check --all-targets --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all --check`,
and the separate fuzz workspace's `cargo check --bins --locked` passed locally.
An additional workspace-wide Clippy check found pre-existing unused/dead-code
warnings in LSP test files; those files are outside this change. The repository's
required Clippy command above remains clean.

Independent source review identified two defects before Green: changing cwd
could re-resolve an authorized relative executable, and formatting a merged
`code statusCode` token could leave its operand stale. Executable identity is
now resolved before applying cwd; the fixer conservatively protects those
operand spellings. WFL exact-path allowlist and fix-then-execute regressions
pass. The reviewer confirmed both source fixes and reported no remaining
blocking lifecycle finding.

`process-wrap` 10.0.0 uses only Tokio/process-group/job-object/kill-on-drop
features, has MSRV 1.87 (below WFL's 1.94), and is MIT/Apache-2.0 licensed.
Both root and fuzz lockfiles include it without unrelated version changes.

## Same-runtime discovery follow-up

Test-first commit `2596d29f` adds two WFL scenarios for `current_executable`:
the returned program exists and reports the running WFL version, and launching
it with a different working directory preserves its identity. The preceding
runtime rejects the valid call with `Undefined action 'current_executable'`.
This is missing-API availability evidence, not a failed runtime assertion.
After native registration, explicit-call catalog/arity metadata, and the
precise zero-argument Text contract were added, both WFL assertions passed.
All eight existing builtin/catalog contract tests and strict root Clippy passed.
The implementation rejects non-Unicode paths with an actionable error and
does not enumerate or expose environment variables. Shell lookup is unnecessary.

## Timeout diagnostics follow-up

Independent Scriptorium runner review identified lost pre-timeout diagnostics.
Commit `012e7c89` adds a WFL child that emits stdout and a deliberately unused
variable warning on stderr, writes readiness, then stalls. The WFL test waits
for readiness before a 50ms bounded wait. Against `a32c74f1`, it fails specifically
because the Timeout error omits stdout: **0/1 passed, exit 1**. No parser or
startup failure is used as Red. The Scriptorium runner counterpart also failed
on a missing recognizable pre-timeout line while later suites still ran.

The corrected wait preserves its typed cause and both bounded captures after
closing ownership. Diagnostic draining has a separate one-second limit;
incomplete capture and cleanup failure remain explicit. The same WFL test now
passes **1/1**, including stderr and capacity reuse, and Scriptorium's runner
suite passes **9/9**. Existing lifecycle **16/16**, ownership **2/2**, and failure
cleanup **2/2** suites remain Green. The four existing subprocess/security/cleanup
and execution-budget Rust suites pass **80/80**; strict root Clippy passes.
Independent source review found no remaining blocker in the follow-up.

## Remote acceptance remains required

Linux process groups/parent-death signalling cannot be executed on this Windows
host. Existing Blacksmith Linux and Windows integration/Run WFL Programs jobs,
full cargo/workspace gates, and exact-commit CI review remain required before
merge. Local Windows Green does not claim Linux acceptance.

## Integration with the merged HTTP controls

On 2026-09-20 the process branch merged upstream `main` at
`eecd658c32e23abdb6f4e0cc881d8af26dacd2d2` (HTTP PR #737). Git merged the shared
AST, interpreter, fixer, typechecker and keyword references without conflicts.
Source inspection confirmed both additive APIs remain present. No feature or
fixture assertion was changed for this integration.

A fresh Windows release build passed all five process WFL suites (**23/23**)
and the merged HTTP redirect/header suite (**8/8**). The existing subprocess
comprehensive program completed all eight groups and the legacy exit program
returned its intended status. The five existing execution-budget, subprocess,
security, cleanup and operand-contract Rust suites passed **106/106**.
Strict root Clippy, formatting, static hygiene and `git diff --check` passed.
Local logs remain under ignored `target/process-main-merge/`. The comprehensive
legacy program retains its previously observed live-child shutdown warning;
its existing behavior and assertions were preserved.

The merge commit still requires exact-head Linux/Windows CI inspection before
the PR is merged. This local integration evidence does not replace that gate.
