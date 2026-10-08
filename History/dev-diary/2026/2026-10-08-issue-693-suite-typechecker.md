# 2026-10-08 — Integration-suite typechecker batch (issue #693)

Second batch of the single-binary integration pattern: seventeen
typechecker files move from `tests/*.rs` (one Cargo test target each)
into `tests/suite/`, compiled as modules of the existing `suite` binary.

The batch is the remaining `typechecker_*.rs` family except the two
stream-named files (`typechecker_response_stream_join_test.rs`,
`typechecker_response_stream_scope_test.rs`), which stay with Group A.
None of the moved files declare `mod common;`, `mod test_helpers;`,
`#[path]`, `include!`, inner attributes, `extern crate`, or `fn main()`,
and none are pinned as `--test` names in CI or scripts.

The remaining top-level `tests/*.rs` files stay standalone. Later
batches still need the `mod common;` / `mod test_helpers;` consumers,
the pinned `--test` names, and Group A last.
