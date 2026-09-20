# ORM prerequisite Red evidence — 2026-09-20

Risk class: R3 (application error unwinding, transactions, schema/data recovery).
Base source: `cb1dadaad96939a4450a6eb2b3a6a51678035b7f`.
Runtime: official Windows nightly WFL 26.9.12.

- `wfl --test TestPrograms/database_schema_transaction_test.wfl`: one test
  executed, one failed, exit 1. A create-copy-drop-rename rebuild in the existing
  native transaction lost an extension-owned `ON DELETE CASCADE` child despite
  `foreign_keys=OFF` and `defer_foreign_keys=ON` inside the block. Expected one
  child row, observed zero. The Green test will select the additive
  `for schema changes` mode on the same transaction construct; normal transaction
  behavior remains unchanged.
- `wfl --test TestPrograms/application_errors_test.wfl`: the new library fixture
  cannot run on the base because `raise_error` is not registered. The runtime
  reports an undefined action during semantic analysis. This is evidence of
  absent functionality, **not** a behavioral Red assertion or a syntax-error
  regression. Scriptorium's separate executable return-failure probe demonstrates
  the unsafe alternative: returning `no` from an ordinary native transaction
  commits its earlier write. Ordinary Boolean-return commit semantics are not
  a bug and will remain unchanged.

All scenarios and assertions are WFL. The existing CI WFL program runner
recursively discovers these TestPrograms files and recognizes their describe
blocks. No application error helper is implemented through an unrelated parse,
division or SQL failure. Tests use synthetic file-backed SQLite and close/delete
fixtures in finally blocks. Local raw command output is retained under ignored
`target/reports/orm-prerequisites/`; durable outcomes are recorded here and in
the test-only commit preceding implementation.
