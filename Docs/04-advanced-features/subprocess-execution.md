# Subprocess Execution

WFL can launch a program, wait for its result, and close the processes it owns.
Use separate arguments so filenames and text are passed literally.

## Enable process execution

Process execution is disabled by default. A trusted project's `.wflcfg` can enable it:

```ini
allow_shell_execution = true
shell_execution_mode = sanitized
kill_on_shutdown = true
```

Both `execute command` and `spawn command` obey the same policy. For a restricted
project, use `shell_execution_mode = allowlist_only` and configure
`allowed_shell_commands` with the permitted executable names or exact paths.
Explicit paths require explicit path entries; a matching basename is insufficient.
Allowlisting an interpreter also permits the code passed to that interpreter.
See the [configuration reference](../reference/configuration-reference.md#security-settings).

## Launch, wait, read, and close

This complete example launches the same WFL executable and reads its version:

```wfl
store runtime_path as call current_executable
wait for spawn command runtime_path with arguments ["--version"] as child
try:
    wait for process child to complete with timeout 10 and read result as outcome
    display outcome["output"]
    check if outcome["success"] is no:
        display outcome["error"]
        exit program with code 1
    end check
finally:
    close process child
end try
```

`spawn command` returns immediately with a process handle. The wait includes
the direct child's exit and the draining of both output streams. On success it
returns the result and releases the handle together:

| Field | Value |
| --- | --- |
| `output` | Retained standard output text |
| `error` | Retained standard error text |
| `exit_code` | Numeric status; `-1` when the operating system reports no numeric status |
| `success` | `yes` when `exit_code` is zero |

A nonzero child exit is a result, not an exception. Launch, wait, and capture
failures are catchable runtime errors. `close process` terminates and reaps an
owned child and releases its readers and handle. It is safe after completion,
after a timeout, or more than once, so use it in `finally`.

The timeout is a finite number of seconds from one nanosecond through one year;
fractional seconds are supported. It covers process execution and pipe draining.
On timeout WFL closes the child, drains the bounded stdout/stderr captures for
at most one additional second, and includes their retained contents under
`Subprocess stdout` and `Subprocess stderr` labels in the `Timeout` error. This
preserves diagnostics emitted before a stalled child without leaving a handle
to manage afterward. A drain limit or read failure is stated explicitly. The run's
execution budget and cancellation still apply. Inside a long-lived `main loop`,
a wait also receives a finite `timeout_seconds` window.

## Choose the working directory

Add `in directory` after the arguments to set the child's directory:

```wfl
wait for execute command "wfl" with arguments ["--version"] in directory current_directory as outcome
display outcome["output"]
```

The same clause works with `spawn command`. It changes only that launch; the
parent's directory is unchanged. Explicit executable paths are resolved against
the parent's directory before applying the child's directory, preserving the
exact executable authorized by an allowlist. Missing or
inaccessible directories produce a launch error. Parenthesize a complex directory
expression, such as `in directory (path_join of workspace and "tests")`.

`execute command` waits immediately and returns the same four result fields.
Both forms accept the existing `using shell` clause after the directory. Shell
execution still obeys configuration; direct arguments do not bypass policy.

## Own the process lifetime

With `kill_on_shutdown = true`, a launch owns a Windows Job Object or a Unix
process group. Closing, timing out, dropping the interpreter, or observing the
direct child's exit terminates remaining processes in that owned group/job.
Linux also sets parent-death signalling before execution, so nested WFL drivers
that create their own groups are closed when their owning driver is killed.
Windows job assignment occurs while the child is suspended, before its code runs.
This is process ownership, not a security sandbox; programs that deliberately
escape a group are outside that group's ownership.

The historical default `kill_on_shutdown = false` keeps direct-child lifecycle
behavior and does not promise descendant cleanup. Enable ownership in test
runners and in their nested WFL fixtures. Each unconsumed handle counts against
`max_concurrent_processes`, including a completed child. Waiting or closing
releases capacity without discarding another child's result.

`process child is running` polls without consuming the result. Existing
`kill process child` remains available and reports an unknown handle as an error;
`close process child` is the convenient idempotent cleanup form.

## Output bounds and older programs

Standard output and standard error each retain at most `max_buffer_size_bytes`
raw bytes (10 MiB by default). WFL continuously drains both streams, retains the
most recent bytes, and warns if older bytes are discarded. Text replacement for
malformed UTF-8 may increase the returned character encoding size by a bounded
factor. The limit applies to foreground commands and background processes.

Existing numeric waits keep their result and release behavior:

```wfl
wait for spawn command "wfl" with arguments ["--version"] as child
wait for process child to complete as exit_status
display exit_status
```

`wait for read output from process child as text` consumes currently buffered
stdout while a handle is live. A later full-result wait returns stdout remaining
after those reads and retained stderr. Prefer one full-result wait when complete
diagnostics are needed. Output cannot be read after a numeric wait releases the
handle.

## Return a program status

`exit program with code 1` stops the WFL program after unwinding `finally`
blocks. Codes must be whole numbers from 0 through 255; zero means success.
`exit with code 1` is also accepted. Bare `exit`, `exit loop`, and
`exit program` retain their existing meanings. A failing test run still returns
status 1 even if code requested a different status.

## Execute a WFL file in the current process

For a fresh WFL environment without an operating-system process, use
`execute wfl file at "report.wfl" and read output as report_output`.
This captures `display` output and supports catchable runtime errors.
See [web servers](web-servers.md#serving-dynamic-wfl-pages) for passing request
context. Use subprocesses when the operating-system working directory, exit
status, or independent process lifetime matters.

The executable regression suite is
[`TestPrograms/process/lifecycle.test.wfl`](../../TestPrograms/process/lifecycle.test.wfl).
