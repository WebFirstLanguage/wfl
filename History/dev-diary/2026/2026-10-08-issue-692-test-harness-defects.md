# Issue #692: three test-harness defects

The 2026-08-14 test-shrink pass recorded three independent harness defects
outside that loop's scope. This change fixes all three on current `main`
without weakening assertions.

## 1. Flaky `test_capture_process_output`

`read_process_output` drains the capture buffer. The test spawned `echo`
(or `cmd.exe /C echo` on Windows), slept a fixed 200 ms, then asserted
once. Under load the reader task had not finished, so the assertion saw
an empty string. A sibling `test_spawn_and_kill_process` used the same
fixed-sleep pattern after `kill`.

Both now wait on the actual condition with a five-second bound: the
capture test accumulates drained chunks until they contain `test output`;
the kill tests poll `is_process_running` until the child is gone. The
original assertions are unchanged.

## 2. Opaque `NotFound` without a release binary

`tests/common/mod.rs` already diagnosed a missing `target/release/wfl`.
`tests/binary_io_test.rs` and three other spawners still built that path
locally and failed with `Os { kind: NotFound }`. They now go through
`wfl_release_exe()`, and `require_existing_release_binary` is tested with
a path that cannot exist so the diagnostic stays attached to the helper.

`tests/test_helpers.rs` and `tests/contains_unification_test.rs` keep
their release-then-debug fallback; they already panic before spawn.

## 3. File-I/O fixtures left in the repo root

Performance, concurrent, and error-handling suites already use
`tempfile::tempdir()`. `tests/file_io_execution_test.rs` still wrote
bare relative names and cleaned them up only after the assertions, so a
panic skipped cleanup and tripped the hygiene job. Those six tests now
splice absolute temp-dir paths, matching the sibling pattern, and a
panic-path test checks that the original payload is preserved and the
repo root stays clean.
