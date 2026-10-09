# 2026-10-09 — Integration-suite `test_helpers` dual-duty resolution (issue #693)

Eighth batch of the single-binary integration pattern: `tests/test_helpers.rs`
and its nine `mod test_helpers;` consumers move into `tests/suite/`.

`test_helpers.rs` was both a standalone test target (6 tests) and a
`mod`-included helper compiled into each consumer, so those 6 tests ran
once per including binary. Declaring it once as `suite::test_helpers`
stops that duplication. Consumers switch to `crate::test_helpers::…`.
`execute_file_test.rs` also drops `mod common;` for `crate::common::…`
and already binds through `common::free_tcp_port`.

The 6 helper-module tests still run, once, as
`test_helpers::tests::…`. Unique tests from each consumer are unchanged.

Skipped: rustyline CLI, pinned `--test` names, remaining port-binding
HTTP/web files (several still use fixed ports), and Group A last.
