# Subprocess Execution

WFL lets you run external commands and programs. Integrate with system tools, build automation scripts, and extend WFL's capabilities.

## Security: opt-in required

**By default, all subprocess execution is disabled.** Both
`execute command` and `spawn command` are blocked unless you enable them in
`.wflcfg`:

```ini
# .wflcfg — required before any execute/spawn will run
allow_shell_execution = true
shell_execution_mode = sanitized
# Tighter alternative:
# shell_execution_mode = allowlist_only
# allowed_shell_commands = echo, ls, git
```

- `allow_shell_execution = false` (default) blocks **every** process launch.
- `shell_execution_mode = forbidden` (default) also blocks every process launch
  when the master switch is on.
- Policy applies to **both** the shell form and the `with arguments` form.
  Passing arguments is safer against injection *after* a program is allowed;
  it is not a bypass of the policy.

See [Configuration Reference](../reference/configuration-reference.md#security-settings)
for full option details.

## Why Subprocess Execution?

Run external programs from WFL:
- Execute system commands (ls, git, npm)
- Run build tools
- Integrate with external utilities
- Automate system administration
- Call other programming languages

Shell and resource limits are controlled by `.wflcfg` (`allow_shell_execution`, `shell_execution_mode`, `allowed_shell_commands`, `max_concurrent_processes`, and related keys). Full reference: **[Configuration Reference](../reference/configuration-reference.md)**.

## Basic Command Execution

### Execute and Wait

Run a command and wait for it to complete (requires opt-in config above):

```wfl
wait for execute command "echo Hello from command line" as result
display "Command executed"
```

**Syntax:**
```wfl
wait for execute command "<command>" as <variable>
```

**Example:**
```wfl
wait for execute command "ls -la" as listing
display "Directory listing complete"
```

### Execute Without Storing

```wfl
wait for execute command "echo Simple execution"
display "Done"
```

## Spawning Background Processes

### Spawn a Process

Start a process in the background:

```wfl
wait for spawn command "echo Background task" as process_handle
display "Process spawned"
```

**Syntax:**
```wfl
wait for spawn command "<command>" as <process_variable>
```

### Wait for Completion

```wfl
wait for spawn command "echo Task" as proc
display "Process running..."

wait for process proc to complete as exit_status
display "Process completed with status: " with exit_status
```

## Process Control

### Check Process Status

```wfl
wait for spawn command "sleep 5" as long_proc

store is_running as process long_proc is running
check if is_running:
    display "Process is still running"
otherwise:
    display "Process has completed"
end check
```

### Kill a Process

```wfl
// "sleep" is a Unix command; on Windows use e.g. "timeout /t 5" instead.
wait for spawn command "sleep 5" as proc

// Wait a bit
wait for 1000 milliseconds

// Terminate it
kill process proc
display "Process terminated"
```

### Capture Output

```wfl
wait for spawn command "echo Captured output" as proc

wait for 100 milliseconds

wait for read output from process proc as output_data
display "Output: " with output_data
```

## Executing WFL Files In-Process

`execute command` starts a separate program. To run another **WFL file**
inside the current program — without spawning a process — use `execute file`:

```wfl
execute wfl file at "report.wfl" and read output as report_output
display "Captured: " with report_output
```

The file runs in a fresh, isolated environment with the full standard
library. With `and read output as`, everything it displays is captured into a
text variable instead of printed. Errors in the executed file are catchable
with `try`. This powers dynamic web pages — see
[Web Servers](web-servers.md#serving-dynamic-wfl-pages) for passing HTTP
request context with `with <request>`.

## Error Handling

Always handle subprocess errors:

```wfl
try:
    wait for execute command "nonexistent_command" as result
    display "Success"
when error:
    display "Command failed - does the command exist?"
end try
```

### Handling Exit Codes

```wfl
try:
    wait for spawn command "exit 1" as proc
    wait for process proc to complete as exit_code

    check if exit_code is equal to 0:
        display "Success"
    otherwise:
        display "Command failed with code: " with exit_code
    end check
catch:
    display "Error running command"
end try
```

## Common Patterns

### Running Git Commands

```wfl
define action called git_status:
    try:
        wait for execute command "git status" as command_output
        display "Git status executed"
        return yes
    catch:
        display "Git command failed - is this a git repository?"
        return no
    end try
end action

call git_status
```

### Build Automation

```wfl
display "=== Build Script ==="

// Clean
display "Cleaning..."
wait for execute command "rm -rf build"

// Build
display "Building..."
try:
    wait for execute command "cargo build --release" as build_result
    display "✓ Build succeeded"
catch:
    display "✗ Build failed"
    exit with code 1
end try

// Test
display "Testing..."
try:
    wait for execute command "cargo test" as test_result
    display "✓ Tests passed"
catch:
    display "✗ Tests failed"
    exit with code 1
end try

display "=== Build Complete ==="
```

### System Information

```wfl
// Get current user
wait for execute command "whoami" as username
display "User: " with username

// Get system info
wait for execute command "uname -a" as system_info
display "System: " with system_info

// Get current directory
wait for execute command "pwd" as current_dir
display "Directory: " with current_dir
```

### File Processing Pipeline

```wfl
display "Processing images..."

wait for store images as list files in "input" with pattern "*.png"

for each image in images:
    store cmd as "convert " with image with " -resize 50% output/" with image

    try:
        wait for execute command cmd
        display "✓ Processed: " with image
    catch:
        display "✗ Failed: " with image
    end try
end for

display "Image processing complete"
```

## Multiple Concurrent Processes

```wfl
display "Starting multiple processes..."

wait for spawn command "task1.sh" as proc1
wait for spawn command "task2.sh" as proc2
wait for spawn command "task3.sh" as proc3

display "All processes started"

// Wait for all to complete
wait for process proc1 to complete
display "Task 1 complete"

wait for process proc2 to complete
display "Task 2 complete"

wait for process proc3 to complete
display "Task 3 complete"

display "All tasks finished"
```

## Security Considerations

⚠️ **Important:** Subprocess execution can be dangerous. WFL disables it by
default; enable it only when needed, preferably with `allowlist_only`.

### Policy layers

1. **Config policy** (`.wflcfg`) — master switch + mode/allowlist, enforced on
   every launch (shell and direct-exec).
2. **Program design** — never splice untrusted input into a command string;
   pass values with `with arguments` and restrict which programs run.

### Command Injection

**Dangerous:**
```wfl
store user_input as "hello"  // Imagine this came from an untrusted user: "; rm -rf /"
store cmd as "echo " with user_input
wait for execute command cmd  // UNSAFE: user input goes straight into the command!
```

**Safer (after opt-in config allows `echo`):** Don't try to filter out
"dangerous" characters — blocklists are always incomplete. Restrict input to
approved values and pass them as arguments so they are never spliced into a
shell command string.

```wfl
store user_input as "hello"  // Untrusted input from a user or request

// Restrict input to an approved set of values (allowlist)
store allowed_values as ["hello", "status", "version"]
check if allowed_values contains user_input:
    // Pass the value as an argument (argv), never concatenated into a command
    wait for execute command "echo" with arguments [user_input]
otherwise:
    display "Invalid input - value is not on the allowlist"
end check
```

### Best Practices

✅ **Leave defaults off** for untrusted or public-facing hosts

✅ **Prefer `allowlist_only`** when you must enable subprocesses

✅ **Validate all input** - Never trust user data

✅ **Pass arguments, don't concatenate** - Avoid shell metacharacters

✅ **Limit permissions** - Run with minimal privileges

✅ **Log commands** - Track what's being executed

❌ **Don't enable `unrestricted` in production**

❌ **Don't assume `with arguments` bypasses policy** - it does not

❌ **Don't ignore exit codes** - Check for failures

## What You've Learned

In this section, you learned:

✅ **Executing commands** - `wait for execute command`
✅ **Spawning processes** - `wait for spawn command`
✅ **Process control** - Check status, kill processes
✅ **Capturing output** - `read output from process`
✅ **Error handling** - Try-catch for robust execution
✅ **Common patterns** - Git, builds, system info, pipelines
✅ **Security** - Command injection risks and prevention

## Next Steps

Complete your advanced features knowledge:

**[Interoperability →](interoperability.md)**
Learn how WFL works with other technologies.

**[Security Guidelines →](../06-best-practices/security-guidelines.md)**
Critical security practices for subprocess execution.

**[Best Practices →](../06-best-practices/index.md)**
Write better, safer WFL code.

---

**Previous:** [← Containers (OOP)](containers-oop.md) | **Next:** [Interoperability →](interoperability.md)
