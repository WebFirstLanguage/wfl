# WFL Logging System

The WFL language includes a comprehensive logging system designed to provide insights into program execution, aid in debugging, and help developers understand the runtime behavior of their scripts.

## Overview

WFL provides two primary types of logging:

1. **General Logging** - Records key interpreter events, warnings, and errors to both the console and a log file.
2. **Execution Logging** - Provides detailed tracing of every execution action taken by the interpreter, including variable assignments, function calls, control flow decisions, and more.

## Configuration

Logging behavior is controlled via the `.wflcfg` configuration file. The following settings are available:

```
# Enable/disable general logging (default: false)
logging_enabled = true

# Set the log level (debug, info, warn, error - default: info)
log_level = debug

# Enable/disable detailed execution logging (default: true for debug builds, false for release)
execution_logging = true
```

These settings can be placed in a `.wflcfg` file in the script's directory to control logging behavior for that script. If no configuration is provided, the defaults are used.

## General Logging

When `logging_enabled` is set to `true`, WFL will create a log file (typically `wfl.log` in the script's directory) that records important events like:

- Program startup and shutdown
- Syntax and runtime errors
- Warnings about potential issues
- Informational messages about script execution

This log is useful for understanding the overall flow of your program and diagnosing issues. The `log_level` setting controls the verbosity of the general log.

## Execution Logging

Execution logging is a more detailed form of logging that records every significant action taken by the WFL interpreter during script execution. This feature is primarily intended for debugging and is only enabled in debug builds by default.

When `execution_logging` is set to `true`, WFL creates a separate log file (typically named `wfl_exec.log`) that includes:

### Variable Operations
- Variable declarations with initial values
- Variable assignments with new values
- Variable lookups with returned values

### Control Flow
- If/else condition evaluation and branch selection
- Loop iterations (count, while, for, etc.)
- Function entry and exit with parameter values

### Expression Evaluation
- Binary operations (arithmetic, comparison, etc.)
- Unary operations
- Function calls with arguments and return values
- Object and list access

### Statement Execution
- Entry and exit of each statement
- Block execution (entry, statements, exit)

### Sample Output

Here's an example of what execution logging output looks like:

```
[12:30:15.123] EXEC: Starting program execution
[12:30:15.124] EXEC: Declaration: 'x' = 10
[12:30:15.125] EXEC: Declaration: 'y' = 20
[12:30:15.126] EXEC: Expression: Binary(x + y) = 30
[12:30:15.127] EXEC: Declaration: 'sum' = 30
[12:30:15.128] EXEC: Control flow: if condition = true
[12:30:15.129] EXEC: ┌─ Block entry: if branch
[12:30:15.130] EXEC: │  Declaration: 'message' = "Sum is greater than 25"
[12:30:15.131] EXEC: │  Function call: display("Sum is greater than 25")
[12:30:15.132] EXEC: └─ Block exit: if branch
```

### Hierarchical Format

The execution log uses indentation to clearly show the nesting of blocks, functions, and control structures. This makes it easier to understand the flow of execution through nested constructs.

## Debug vs. Release Behavior

In debug builds (when compiled with debug assertions):
- Execution logging is enabled by default but can be disabled via configuration
- Execution logging macros compile to actual logging code

In release builds:
- Execution logging is disabled by default
- Execution logging macros are completely removed from the compiled code, ensuring zero runtime overhead

## Integration with Debug Reports

When an error occurs and debug reporting is enabled, the execution log can be a valuable companion to the debug report. While the debug report provides a snapshot of the program state at the moment of the error, the execution log shows the sequence of steps that led to that state.

## Implementation Details

The execution logging system is implemented using:

- Conditional compilation to eliminate logging code in release builds
- A separate logger initialized only when execution logging is enabled
- A set of macros that provide a consistent interface for logging different types of execution events
- An indentation system that visually represents the nesting of code blocks

## Best Practices

1. **Development and Testing**: Enable both general and execution logging during development and testing to gain insights into program behavior.

2. **Debugging**: When encountering issues, enable execution logging to trace the exact sequence of operations leading to a problem.

3. **Production**: Disable execution logging in production for optimal performance.

4. **Performance Testing**: When measuring performance, disable all logging to avoid I/O overhead affecting results.
