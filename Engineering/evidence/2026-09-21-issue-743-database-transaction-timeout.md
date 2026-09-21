# Issue #743 — Windows `database_transaction_test.wfl` TIMEOUT

## Risk class

R3 — lifecycle, timeouts, database locks, process shutdown.

## Cause

The suite-versus-isolated discrepancy is a 30-second SQLite pool wait that
coincides with `scripts/run_integration_tests.ps1`'s unchanged program
deadline:

- sqlx default `acquire_timeout` is 30 seconds.
- `Pool::close` waits indefinitely for outstanding connections.
- File-backed pools used DELETE journal mode with five connections.
- The runner discarded child stdout/stderr, so a completed test report or a
  database error could not be distinguished from a hang.

Isolated runs finish because they do not contend with a prior suite's file
locks, antivirus scan backlog, or leftover `tx.db` sidecars. The full Windows
suite does.

## Red

Commit `1217f513` (`test: reproduce SQLite pool waits matching the 30s runner`).

```text
cargo test --test database_transaction_test runner_deadline -- --nocapture --test-threads=1
```

Both new tests failed for the intended reason after ~8 seconds:

- `file_backed_pool_does_not_wait_the_runner_deadline_when_busy` — sixth acquire
  waited longer than 8s.
- `closing_a_file_backed_pool_does_not_wait_the_runner_deadline` — close with
  an outstanding connection waited longer than 8s.

## Green

After bounding acquire/close, enabling WAL, closing leftover CLI pools, and
exiting 0 after a successful `--test` run:

```text
cargo test --test database_transaction_test -- --test-threads=1
# 23 passed (including the two previously failing runner-deadline tests)

cargo test --test database_transaction_cli_test -- --nocapture
# 1 passed in 0.04s

cargo test --test database_test -- --test-threads=1
# 20 passed

target/debug/wfl --test TestPrograms/database_transaction_test.wfl
# Total: 8  Passed: 8  exit 0 in 0.073s
```

Clippy (`--test database_transaction_test --test database_transaction_cli_test --bin wfl -- -D warnings`) and `python3 scripts/check_repo_hygiene.py --mode static` passed. The runner deadline remains 30 seconds.

## Residual risk

Windows Defender or a leftover WAL from a killed process can still slow the
first open of a file-backed database. The five-second bound turns that into a
reported error plus retained child logs instead of a runner TIMEOUT. REPL
database handles are intentionally left open across commands.
