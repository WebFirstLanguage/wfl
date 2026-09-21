# 2026-09-21 — File-backed SQLite waits vs the Windows integration runner (#743)

The canonical Windows runner timed out `TestPrograms/database_transaction_test.wfl`
after its unchanged 30-second deadline. Isolated runs of the same program
passed all eight assertions. The runner then deleted the child's stdout and
stderr, so a 30-second pool wait looked exactly like a hung process.

sqlx's default `acquire_timeout` is 30 seconds and `Pool::close` waits until
every connection is returned. A full file-backed pool, or a close with an
outstanding connection, therefore matches the runner deadline. File-backed
SQLite also used DELETE journal mode with a five-connection pool, which on
Windows stacks reserved-lock waits.

The failing regressions hold all five connections and then acquire or close;
both waited past 8 seconds before the runtime change. File-backed pools now
use WAL, a five-second busy/acquire wait, and a bounded close. The interpreter
closes leftover pools at shutdown, and a successful `--test` run exits the
process the same way a failing one already did. The runner keeps child logs
under `target/test-artifacts/integration-runner/` on timeout or failure.

Transaction commit and rollback rules are unchanged.
