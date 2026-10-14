# 2026-10-09 — Integration-suite language leftovers + first `mod common` conversion (issue #693)

Sixth batch of the single-binary integration pattern: seventeen language,
include/module, issue-regression, and small CLI-flag files move from
`tests/*.rs` (one Cargo test target each) into `tests/suite/`, compiled
as modules of the existing `suite` binary.

Crate-root-safe leftovers were exhausted after the fixer/language batch.
These files were deferred only because they declared `mod common;`, which
resolves at a test-crate root to `tests/common/mod.rs` and would break as
a child module. `tests/suite/main.rs` already declares
`#[path = "../common/mod.rs"] mod common;`; this batch removes each
file-local `mod common;` and switches consumers to `crate::common::…`.

None of the filenames are pinned as `--test` names in CI or scripts.
Child-process `.current_dir(tempdir)` is used in a few CLI helpers; no
process-wide `set_current_dir` / `set_var`. Group A concurrency/web-server
files, rustyline CLI tests, `mod test_helpers;` consumers, stream-named
typechecker siblings, and wall-clock bound tests stay top-level.
`tests/test_helpers.rs` stays put until its dual-duty role is resolved.

The remaining top-level `tests/*.rs` files stay standalone. Later batches
still need the remaining `mod common;` consumers, the `mod test_helpers;`
consumers, the pinned `--test` names, and Group A last.
