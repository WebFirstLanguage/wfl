# `wfl --test` Failure Accounting: Runtime Errors Now Count, Assertions No Longer Double-Reported

**Date:** 2026-07-10

## What Changed

Two bugs in the built-in test framework's result accounting were fixed. Both
lived in the `TestBlock` error handler in `src/interpreter/mod.rs`.

### Bug 1 — a runtime error in a test silently "passed"

When a `test` body failed with a *runtime* error (anything other than a failed
`expect`, e.g. an undefined variable), the failure was pushed to the failures
list but `failed_tests` was **never incremented** — that counter was only ever
bumped by the `ExpectStatement` handler. The consequence:

- the summary printed `Failed: 0` even though the failure was listed, and
- `results.failed_tests > 0` was false, so the process **exited 0**.

A crashing test therefore looked green to CI. This is the test-mode analog of
the exit-code bug fixed on 2026-07-03 for the normal execution path.

### Bug 2 — a failing assertion was reported twice

The handler tried to avoid re-recording assertion failures (already recorded by
`ExpectStatement`) with this guard:

```rust
let error_msg = e.to_string();
if !error_msg.starts_with("Assertion failed:") { /* record */ }
```

But `RuntimeError`'s `Display` prepends `"Runtime error at line L, column C: "`,
so the string is `"Runtime error at line …: Assertion failed: …"` and the guard
**never matched**. Every failing assertion was pushed to the failures list a
second time (once clean from `ExpectStatement`, once with the `Runtime error …`
prefix from `TestBlock`).

## The Fix

Inspect the raw `RuntimeError::message` field (which really does begin with
`"Assertion failed:"`) instead of the `Display` string, and — when the error is
*not* an assertion failure — increment `failed_tests` alongside pushing the
failure:

```rust
if !e.message.starts_with("Assertion failed:") {
    let context = self.current_describe_stack.borrow().clone();
    let failure = TestFailure { /* … */, assertion_message: e.to_string(), /* … */ };
    let mut results = self.test_results.borrow_mut();
    results.failures.push(failure);
    results.failed_tests += 1;
}
```

Assertion failures now match the guard and are recorded exactly once; runtime
errors are recorded once *and* counted, so `total == passed + failed` holds and
the exit code is 1 whenever any test fails. The runtime-error message keeps its
`Display` form so its real line/column (which differ from the `test` block's
line) are still shown.

## Tests

Added `tests/test_framework_counting_test.rs` covering: a runtime error counting
as a failure, a failing assertion recorded exactly once, a mixed suite where
`total == passed + failed`, and short-circuiting on the first failing assertion.
All existing `.test.wfl` programs and the repo's test-framework validation
programs continue to pass.
