# 2026-10-08 — Integration-suite pilot (issue #693)

First batch of the single-binary integration pattern: ten crypto/stdlib
files move from `tests/*.rs` (one Cargo test target each) into
`tests/suite/`, compiled as modules of one `suite` binary.

`#694` landed after the issue was filed, so five of the ten files now
declare `mod common;`. That is declared once on `tests/suite/main.rs` as
`#[path = "../common/mod.rs"] mod common;`, and those five files use
`crate::common`. No `tests/test_helpers.rs` consumers and no externally
pinned target names are in this batch.

The remaining 154 top-level `tests/*.rs` files stay standalone. Later
batches still need the `mod common;` / `mod test_helpers;` consumers,
the three pinned `--test` names, and Group A last.
