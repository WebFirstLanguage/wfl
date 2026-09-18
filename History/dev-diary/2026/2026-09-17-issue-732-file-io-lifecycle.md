# Issue #732: file I/O timing, isolation, and closed handles

The investigation started from current main, `c60a66e3`, and is classified
**R3** because the changes concern asynchronous file operations, handle
lifecycle, errors, and compatibility.

## What reproduced

The focused Windows baseline passed all seven concurrent tests and all seven
performance tests, but four error-handling tests exceeded their existing
five-second timeout. The concurrent and performance suites already use separate
OS temporary directories: PR #727 (`c2358419`) moved their fixtures away from
the checkout. That earlier change is part of the baseline, not a fix introduced
by this investigation. The error-handling suite still used fixed filenames in
the checkout and cleaned them up only after its assertions.

A storage probe flushed five distinct files on each volume. Flush durations
on the checkout volume were approximately 1.78, 1.90, 3.11, 1.94, and 1.91 seconds;
the temporary-directory volume took approximately 2 milliseconds per file.
This explains how a few durable writes and closes can exhaust a five-second
test deadline on this machine. It does not establish that all Windows
filesystems have the same latency. Raising deadlines would leave this fixture
dependency in place.

The review also found an independent correctness defect. Text reads and writes
treated a handle missing from the open-handle map as a filesystem path. Using
a closed handle could therefore create or overwrite a file named `file1`
instead of reporting an error. The existing closed-handle test accepted either
successful execution or a caught error, then deleted the unintended `file1`.
It did not enforce its stated contract.

The compatibility review found that the same fallback opened direct read paths
with write access and file creation enabled. A read of a missing path could
silently create an empty file, while a readable but read-only file could be
rejected. Additional CLI regressions cover literal and variable missing paths,
read-only files, exact read contents, and unchanged permissions.

## Contracts being enforced

- Rejected operations on closed handles, including aliases, must report an
  error and preserve the original bytes without creating a handle-named file.
  Text reads, text writes, appends, binary reads, partial binary reads, and
  binary writes are covered.
- Closing a handle twice remains valid. Completed writes must remain visible
  to an immediate read after close.
- Existing direct text-path reads and writes remain supported. Compatibility
  includes path variables, filenames beginning with `file`, and paths named
  `file1` even after an unrelated handle has closed. A copy of an actual closed
  handle must still be rejected.
- Error-handling fixtures must own isolated temporary directories and assert
  the actual error path and file contents. Existing operation deadlines stay
  in force.
- A direct text read opens an existing file for reading only. A missing read
  path raises an error without creating a file; reading does not require write
  permission or change the file contents or permissions.

The new `tests/file_io_lifecycle_cli_test.rs` exercises the real Cargo-built
CLI in a temporary working directory with an isolated global configuration.
Each child has a 30-second deadline and is killed and reaped on timeout.
Output capture uses temporary files so a full pipe cannot prevent timeout
enforcement. File contents and unexpected artifacts are checked outside WFL.

`TestPrograms/file_flush_test.wfl` now contains `describe`/`expect` assertions
for immediate readback, all three rapid write-close-read cycles, and exact
append contents. Its cleanup runs in `finally`. The integration runner can
therefore fail on incorrect contents instead of accepting printed `FAILED`
messages with a zero exit status.

The file I/O guide and filesystem reference explain handle lifecycle,
idempotent close, errors after close, and the distinction between paths and
generated handles.

## Verification

The baseline failures and storage comparison above were observed. The failing
regressions were retained before their implementation commits; the complete
ancestry and commands are in the
[test evidence](../../../Engineering/evidence/2026-09-17-issue-732-file-io.md).

The revised WFL flush program was separately checked with the baseline release
binary in a temporary working directory. The normal program exited zero. A
temporary copy with one deliberately incorrect expected value exited one.
Both executions removed all five output files. This verifies the new assertion
and cleanup wiring independently of the runtime fix; it does not
establish Green for the lifecycle regressions.

Follow-up compatibility review retained two additional test-only revisions,
both exercised against untouched baseline production at `c60a66e3`:

- `e816eef4`: the `direct_path_read_` CLI filter ran three tests, all failing
  for the intended defects. Missing literal and variable reads created their
  target file, and a read-only file could not be read because the fallback
  requested write access.
- `82719eae`: the `closed_dispatch_` CLI filter ran four tests. Direct text
  reads, legacy `write ... to ...`, and a size query on a closed alias failed
  their rejection assertions. The size query incorrectly reported the size of
  an unrelated existing `file1` path. The no-wait `write content` case already
  passed and records compatibility that must be preserved.

Independent review added coverage for retrying a failed close and bounding
discarded handle metadata even when opens fail. Test-only `bbaa1e20` reproduced
both defects; `5e9b3111` corrected them after the original implementation in
`51ff3080`. Final focused Windows checks passed: 15 lifecycle tests, seven
existing file-read/budget tests, ten error-handling cases, and 17 real CLI
regressions. All existing test deadlines remain unchanged.

The associated pull request records the complete Windows release/integration
flow and final Linux/Windows CI results separately from these focused checks.

## Automated review follow-up

Devin identified an unnecessary synchronization when an existing file was
opened for appending and closed without any write. CodeRabbit identified an
error-suite cleanup guard that could panic during unwinding and hide the
original assertion. Test-only commit `3540f0f5` reproduces both: the lifecycle
suite observed two unexpected close syncs, and the removed-fixture cleanup
test caught the guard's `NotFound` panic. Successful permission restoration
remains explicitly asserted.

The append repair opens without creation first and falls back to the original
create-enabled append only on `NotFound`. A successful first open is clean;
the fallback conservatively retains synchronization even if another process
created the target in between. This avoids an existence-check race and
preserves dangling symlink behavior with at most two opens. Tests also cover
append cancellation, creation followed by cancelled close, concurrent append
contents, and Unix symlinks. Permission cleanup becomes best-effort during
destruction so its failure cannot replace the original test failure.

Helper documentation now explains isolation, deadlines, handle identity,
dirty-state retention, and cleanup contracts. The linked investigation and PR
retain the focused Red/Green results and final validation for this follow-up.
