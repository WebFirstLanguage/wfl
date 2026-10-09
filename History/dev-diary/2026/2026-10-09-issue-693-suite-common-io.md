# 2026-10-09 — Integration-suite file-I/O / subprocess leftovers (issue #693)

Seventh batch of the single-binary integration pattern: ten file-I/O,
database-transaction, subprocess, lint-CLI, and issue-regression files
move from `tests/*.rs` into `tests/suite/`.

Seven of them still declared `mod common;` and are rewritten to
`crate::common::…` against the suite-level
`#[path = "../common/mod.rs"] mod common;`. The other three
(`file_io_concurrent_test.rs`, `lint_cli_test.rs`, `subprocess_test.rs`)
were already crate-root-safe.

Hang-safety timeouts (`tokio::time::timeout`, child-process watchdogs,
one 20-second non-advancing-loop refusal in `issues_698_700_test.rs`)
are hang detectors, not performance assertions. Retry backoff sleeps in
`subprocess_security_test.rs` stay local to that helper.
`file_io_concurrent_test.rs` is file I/O only (no ports).

Skipped: `mod test_helpers;` consumers, pinned `--test` names, Group A
web/stream/lifecycle files, rustyline CLI, port-binding HTTP/web, the
15-second CLI process bound in `database_transaction_cli_test.rs`, and
`password_policy_lifecycle_test.rs` (kept standalone to isolate
process-wide hashing limits). `tests/test_helpers.rs` stays put.

The remaining top-level `tests/*.rs` files stay standalone. Later
batches still need the remaining `mod common;` consumers that bind
ports, the `mod test_helpers;` consumers, the pinned `--test` names,
and Group A last.
