# 2026-10-08 — Integration-suite analyzer/include batch (issue #693)

Third batch of the single-binary integration pattern: seventeen analyzer,
include, and export files move from `tests/*.rs` (one Cargo test target
each) into `tests/suite/`, compiled as modules of the existing `suite`
binary.

The batch is the remaining static-analysis family that needs no
`mod common;`, `mod test_helpers;`, `#[path]`, `include!`, inner
attributes, `extern crate`, or `fn main()`, and none of the filenames
are pinned as `--test` names in CI or scripts. Stream-named typechecker
siblings and Group A concurrency files stay top-level.

The remaining top-level `tests/*.rs` files stay standalone. Later
batches still need the `mod common;` / `mod test_helpers;` consumers,
the pinned `--test` names, and Group A last.
