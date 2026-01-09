# Subprocess Execution

WFL lets you run external commands and programs. Integrate with system tools, build automation scripts, and extend WFL's capabilities.

## Why Subprocess Execution?

Run external programs from WFL:
- Execute system commands (ls, git, npm)
- Run build tools
- Integrate with external utilities
- Automate system administration
- Call other programming languages

## Basic Command Execution

### Execute and Wait

Run a command and wait for it to complete:

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
wait for execute command "ls -la" as output
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
wait for spawn command "long_running_task" as proc

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
        wait for execute command "git status" as output
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

⚠️ **Important:** Subprocess execution can be dangerous!

### Command Injection

**Dangerous:**
```wfl
store user_input as get_user_input()  // User could input: "; rm -rf /"
store cmd as "echo " with user_input
wait for execute command cmd  // UNSAFE!
```

**Safe:**
```wfl
// Validate input first
check if contains ";" in user_input or contains "|" in user_input:
    display "Invalid input - special characters not allowed"
otherwise:
    store cmd as "echo " with user_input
    wait for execute command cmd
end check
```

### Best Practices

✅ **Validate all input** - Never trust user data

✅ **Use whitelists** - Only allow specific commands

✅ **Sanitize paths** - Prevent directory traversal

✅ **Limit permissions** - Run with minimal privileges

✅ **Log commands** - Track what's being executed

❌ **Don't use user input directly** - Always validate

❌ **Don't execute arbitrary commands** - Whitelist allowed commands

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
