# Owned process results for WFL test runners

Scriptorium's WFL-only runner needs disposable working directories, exact child
status and diagnostics, bounded timeouts, and cleanup of nested server fixtures.
The existing process forms could start children, but numeric completion removed
their output handles and asynchronous reads exposed only stdout. The documented
exit-code syntax was also unavailable.

The Red commit records three valid WFL programs failing on official nightly
26.9.12. The additive API gives execute/spawn an optional `in directory` clause,
adds `with timeout ... and read result as ...` to completion, idempotent
`close process`, and `exit program with code` (including `exit with code`).
Existing numeric waits still consume their handles. Full completion joins both
bounded stream readers and consumes only that child's ownership. Polling and
sibling cleanup preserve unread results; concurrent launches check capacity
under the insertion lock.

Opting into `kill_on_shutdown` now owns a process group on Unix or a Windows job.
Linux parent-death signalling closes nested WFL ownership chains after a hard
timeout. Normal direct-child exit closes remaining descendants before pipe EOF
is awaited. Windows job assignment occurs before the child is resumed.
`process-wrap` 10.0.0 supplies the platform wrappers with only the Tokio,
job-object, process-group and kill-on-drop features enabled; its Rust 1.87 MSRV
fits WFL's Rust 1.94 requirement and its MIT/Apache-2.0 license is compatible.

All new scenarios and fixtures are WFL. The existing release TestPrograms
harness discovers `process/lifecycle.test.wfl`, `process/ownership.test.wfl`,
and `process/failure-cleanup.test.wfl` on Linux and Windows. Boundary
coverage includes capacity exhaustion/release, invalid timeouts and exit codes,
literal argv and cwd, output truncation, timeout cleanup, and nested children
under success, runtime failure and explicit program status. See the
[acceptance evidence](../../../Engineering/evidence/2026-09-20-process-lifecycle.md)
for observed results and remaining platform checks.

The WFL-only runner also needs to select its own runtime without external
`which`/`where` programs. `call current_executable` is a read-only core builtin
backed by the operating system's executable path. It rejects non-Unicode paths
instead of guessing. Its WFL tests execute the returned program and verify the
same identity after changing a child's cwd. No environment enumeration or
mutation API was added.

Independent review of Scriptorium's WFL runner found that a timed-out owned wait
discarded diagnostics printed before the timeout. The WFL reproducer writes a
readiness marker, emits stdout and an intentional compiler stderr warning, then
waits. The test-first commit `012e7c89` fails on missing stdout in the error.
The wait now drains both bounded captures after termination, with a separate
one-second drain limit, and includes them in the same typed timeout error.
Handle consumption and idempotent cleanup are unchanged. No Python or Rust test
scenario was added.
