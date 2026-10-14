# 2026-10-08 — Integration-suite parser/value batch (issue #693)

Fourth batch of the single-binary integration pattern: nineteen parser,
container, and value files move from `tests/*.rs` (one Cargo test target
each) into `tests/suite/`, compiled as modules of the existing `suite`
binary.

The batch needs no `mod common;`, `mod test_helpers;`, `#[path]`,
`include!`, inner attributes, `extern crate`, or `fn main()`, and none
of the filenames are pinned as `--test` names in CI or scripts.
`web_server_tls_parser_test.rs` is parse-only (no bind). Group A
concurrency files and the stream-named typechecker siblings stay
top-level. `tests/test_helpers.rs` stays put because it is still a
standalone target and a `mod`-included helper.

The remaining top-level `tests/*.rs` files stay standalone. Later
batches still need the `mod common;` / `mod test_helpers;` consumers,
the pinned `--test` names, and Group A last.
