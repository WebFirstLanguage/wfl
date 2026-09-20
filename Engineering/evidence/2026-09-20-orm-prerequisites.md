# Application errors and SQLite schema transaction evidence

Risk class: **R3** (migration data integrity, cancellation, connection lifecycle,
and language compatibility). Base: `cb1dadaad96939a4450a6eb2b3a6a51678035b7f`.
This is upstream runtime work required by Scriptorium's WFL ORM; no consumer
application or shared upstream checkout is modified by this branch.

## Public contracts and test coverage

| Acceptance criterion | Executable WFL coverage |
| --- | --- |
| Libraries raise actionable errors, callers catch them, finally runs, earlier writes roll back | `TestPrograms/application_errors_test.wfl` (5 tests) |
| Invalid dynamic error arguments do not disclose their values | Same suite: synthetic credential marker absent from diagnostic |
| SQLite table copy/drop/rename preserves extension-owned referring rows | `TestPrograms/database_schema_transaction_test.wfl` (2 tests) |
| Foreign-key check rejects and rolls back invalid schema, rows and ledger | Same schema suite |
| Error rollback restores enforcement | `TestPrograms/database_schema_recovery_test.wfl` (1 test) |
| In-memory database survives; nested scopes reject before setup; ordinary false returns commit; markers remain names | `TestPrograms/database_schema_compatibility_test.wfl` (3 tests) |
| Immediate migration lock acquisition has a five-second bound; a later attempt can succeed | `TestPrograms/database_schema_lock_test.wfl` (1 test), two handles to one real SQLite file |
| Uncaught errors produce nonzero CLI status; explicit exit and killed migration processes leave no partial schema or ledger | `TestPrograms/database_schema_lifecycle_test.wfl` (4 tests) |
| Source fixing preserves contextual schema markers while fixing unrelated names | Same lifecycle suite, actual lint/fix/parse CLI |
| Cancelling a suspended handler restores the surviving interpreter's single connection and permits a new schema transaction | `TestPrograms/schema_cancellation/cancellation.test.wfl` (1 test), actual WFL server and clients |

Every new assertion, scenario and fixture is WFL. Fixtures are under
`tests/fixtures/application-errors/` and `tests/fixtures/schema-transactions/`.
The existing recursive Linux/Windows TestPrograms gates discover all four
suites; their failures exit nonzero. Subprocesses use direct argument lists,
loopback peers and owned artifacts under `target/test-artifacts/`, and cleanup
runs in `finally`. Parser fuzz seeds retain valid, incomplete and repeated
contextual marker forms under `fuzz/seeds/fuzz_parser/`.

## Red evidence

The test-only predecessor `5d87d9b7` is retained as an ancestor of the Green
implementation. Its exact baseline result and nightly provenance are recorded
in [the initial Red record](2026-09-20-orm-prerequisites-red.md). The existing
transaction syntax actually deleted the extension child during a rebuild:
expected one row, observed zero. The new builtin was absent on the baseline;
its unavailable-symbol diagnostic is explicitly not claimed as a behavioral
Red assertion. A separate consumer probe demonstrated that returning `no`
cannot abort a native block, motivating an explicit application error.

Two further defects were reproduced before their corrective edits on
2026-09-20:

- A candidate supporting the new marker failed the lifecycle source-fixer
  case: the real `--lint --fix --in-place` command exited 2, expected 0. Its
  name-wide rewrite had changed the identical contextual marker. Conservatively
  protecting that spelling fixed the regression.
- Independent review found that the original registry outlived a cancelled
  transaction future. A WFL peer case entered a schema transaction, signalled
  readiness, then a concurrent `/stop` request broke the server loop. Post-loop
  work could not reuse its single connection: expected recovery readiness
  `yes`, observed `no`. A block guard now removes its exact scope/handle/slot
  reservation when the future is dropped, during acquisition as well as body
  execution. This also repairs the ordinary transaction lifetime gap.

These follow-up failures were observed against the in-progress candidate and
are not represented as baseline-nightly failures. The initial attempt to use
an arbitrary sleeping handler's client disconnect did not trigger a documented
cancellation point; the final case explicitly drops the handler through
concurrent-loop shutdown.

## Local Green verification

Tuple: Windows x86-64, Rust/Cargo 1.98.1, WFL 26.9.12. Final source candidate:
`target/release/wfl.exe`, SHA-256
`8c85cdf41e0400cb2534debe47f964aba028dd97f966689686ced49c1dbd9c75`.
It is a local candidate, not a released nightly replacement.

- `cargo build --release --locked -p wfl --bin wfl` passed.
- `target/release/wfl.exe --test TestPrograms/application_errors_test.wfl`: 5/5.
- `target/release/wfl.exe --test TestPrograms/database_schema_transaction_test.wfl`: 2/2.
- `target/release/wfl.exe --test TestPrograms/database_schema_recovery_test.wfl`: 1/1.
- `target/release/wfl.exe --test TestPrograms/database_schema_compatibility_test.wfl`: 3/3.
- `target/release/wfl.exe --test TestPrograms/database_schema_lock_test.wfl`: 1/1.
- `target/release/wfl.exe --test TestPrograms/database_schema_lifecycle_test.wfl`: 4/4.
- `target/release/wfl.exe --test TestPrograms/schema_cancellation/cancellation.test.wfl`: 1/1.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --all-targets --all-features -- -D warnings` passed on final source.
- `cargo check --locked --manifest-path fuzz/Cargo.toml` passed; no sustained
  fuzz campaign is claimed.
- `python scripts/check_repo_hygiene.py --mode static` passed using the bundled
  Python/Git runtime and a process-local safe-directory setting.
- `cargo test --locked --test database_transaction_test`: 21/21 passed before
  the lifetime follow-up. The final full run below supersedes that evidence.
- Final `cargo test --all --locked`: 2,414 passed, 27 existing ignored, across
  175 result records; exit 0. No ignored test was added or changed.
- `python scripts/validate_docs_examples.py --ci --force`: 36/36 passed.
- Existing `scripts/run_web_tests.ps1`: 2/2 passed. Its existing TLS case was
  skipped because OpenSSL is unavailable; Rust TLS tests passed in the full
  cargo run. No skip was added or changed.
- The first existing Windows integration runner run returned 146 pass and two
  30-second timeouts: the original seven-case schema file and unchanged
  `file_io_comprehensive.wfl`. Focused investigation passed the unchanged file
  I/O program in 2.56 seconds. The consumer team independently measured over
  ten seconds in two durable SQLite fixture DDL statements on this host drive.
  Schema cases are now split into rebuild/recovery/compatibility/locking
  programs, with unchanged assertions and lock limits. Only fixture setup
  statements are grouped in an ordinary transaction to avoid unnecessary
  individual durability flushes; database durability settings are unchanged.
  The seven reorganized cases passed together in 5.23 seconds. A new full
  runner result is pending for this changed test organization. The first
  failure remains retained in `integration-final.log`; the new run writes
  `integration-split-final.log`. No full-gate pass is claimed yet.

Logs are under ignored `target/reports/orm-prerequisites/`. The first hygiene
invocation could not spawn Git from the inherited executable search path;
using the bundled Git resolved the environment error. The existing Windows
runner needs a fresh child process with one combined `Path` entry because this
host injects both `Path` and `PATH`; no runner logic or test expectation changed.

## Review, recovery and limits

Independent review checked runtime cancellation ownership, schema cleanup,
ordinary transaction compatibility and contextual parser/fixer behavior. The
reported registry-lifetime finding was fixed, its WFL regression passed, and
source re-review found no remaining blocking issue. Maintainer approval and
remote CI are separate requirements.

No production database was touched. Within schema mode, checked commit makes
schema/data/ledger atomic; explicit errors, exit, interrupted processes and
cancelled scopes have executable rollback/recovery coverage. A cleanup failure
discards the connection instead of returning it with foreign keys disabled.
An in-memory database may be lost if its only connection must be discarded;
the diagnostic directs callers to close and reopen the handle. Schema mode is
SQLite-only, nesting is rejected, and migrations must read their ledger under
the acquired write lock. Returning `no` is still ordinary success.

Current local evidence covers SQLite on Windows. Linux, PostgreSQL/MariaDB
compatibility service jobs, the VS Code host matrix and final integrated-branch
CI remain required upstream gates. There is no new VS Code UI, package,
dependency, credential or external service integration. Existing Rust LSP and
runtime tests provide unchanged-surface coverage. The repository profile's
coverage and scheduled-fuzz gaps remain visible; this record claims neither
numeric coverage nor a release approval. Do not merge or release on this local
record alone.
