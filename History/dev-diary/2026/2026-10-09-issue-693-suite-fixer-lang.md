# 2026-10-09 — Integration-suite fixer/language batch (issue #693)

Fifth batch of the single-binary integration pattern: fourteen fixer,
language, and isolated file-I/O files move from `tests/*.rs` (one Cargo
test target each) into `tests/suite/`, compiled as modules of the
existing `suite` binary.

The batch needs no `mod common;`, `mod test_helpers;`, `#[path]`,
`include!`, inner attributes, `extern crate`, or `fn main()`, and none
of the filenames are pinned as `--test` names in CI or scripts.
`write_line_backcompat_test.rs` only mentions `listen` in source
strings. File-I/O tests rewrite relative paths into temp dirs. Group A
concurrency/web-server files, CLI rustyline tests, and the stream-named
typechecker siblings stay top-level. `tests/test_helpers.rs` stays put.

The remaining top-level `tests/*.rs` files stay standalone. Later
batches still need the `mod common;` / `mod test_helpers;` consumers,
the pinned `--test` names, and Group A last.
