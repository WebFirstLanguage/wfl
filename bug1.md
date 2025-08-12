# WFL Stack Overflow Bug Report

## Bug Summary

A stack overflow occurs when running WFL programs that process command line arguments containing flags starting with `--` during the "Flag/Option Parsing" section. The bug is specifically triggered when executing the nested conditional logic that handles flag detection and list operations in a complex program structure.

**Command that triggers bug**: `cargo run -- args_comprehensive.wfl --azusa is cool`  
**Command that works**: `cargo run -- args_comprehensive.wfl Azusa is cool`

## Evidence Collected

### 1. Reproduction Steps
The stack overflow occurs consistently when:
1. Running `TestPrograms/args_comprehensive.wfl` with arguments starting with `--`
2. The program reaches section "7. Flag/Option Parsing" 
3. Execution begins processing the nested conditional logic
4. Stack overflow occurs before any output from the flag parsing section

### 2. Isolated Testing Results
Through systematic isolation testing, I found:

- **Simple conditional logic**: Works fine
- **Simple for loops over args**: Works fine  
- **Substring function calls**: Work fine
- **List operations (push)**: Work fine
- **Simple nested conditionals**: Work fine
- **Concatenation with 'with'**: Works fine

However, the **exact combination** from the original program triggers the stack overflow.

### 3. Exact Problematic Code Section
The issue occurs in this specific code block (lines 118-157 of `args_comprehensive.wfl`):

```wfl
display "7. Flag/Option Parsing"
check if arg_count is greater than 0:
    store has_help as no
    store has_version as no  
    store has_verbose as no
    store non_flag_args as []
    
    for each arg in args:
        check if arg is "--help" or arg is "-h":
            change has_help to yes
        otherwise:
            check if arg is "--version" or arg is "-v":
                change has_version to yes
            otherwise:
                check if arg is "--verbose":
                    change has_verbose to yes
                otherwise:
                    check if substring of arg and 0 and 1 is "-":
                        display "  Unknown flag: " with arg
                    otherwise:
                        push with non_flag_args and arg
                    end check
                end check
            end check
        end check
    end for
    
    display "Flags detected:"
    display "  Help flag: " with has_help
    display "  Version flag: " with has_version
    display "  Verbose flag: " with has_verbose
    
    display "Non-flag arguments:"
    for each non_flag in non_flag_args:
        display "  - " with non_flag
    end for
otherwise:
    display "No arguments for flag parsing"
end check
```

### 4. Stack Overflow Error Details
```
thread 'main' has overflowed its stack
error: process didn't exit successfully: `target\debug\wfl.exe args_comprehensive.wfl --azusa is cool` 
(exit code: 0xc00000fd, STATUS_STACK_OVERFLOW)
```

## Analysis

### Root Cause Investigation

#### Hypothesis 1: Deep Async Recursion
The WFL interpreter uses `Box::pin` for async recursion handling in:
- `evaluate_expression()` -> `_evaluate_expression()`
- `execute_block()` -> `_execute_block()`  
- `call_function()`

The exact combination of:
1. Nested conditional statements (4 levels deep)
2. Function calls (`substring`)
3. String concatenation with `with`
4. List operations (`push`)
5. Variable access and modification
6. Loop iteration over lists

...may be causing excessive stack frame allocation during async execution.

#### Hypothesis 2: Environment Chain Corruption
The problem occurs when combining:
- Multiple variable definitions in nested scopes
- For-each loop variable binding (`arg`, `non_flag`)
- Environment scope creation for loop iterations
- Complex expression evaluation within nested conditionals

This combination may lead to circular references or deep environment chain traversal.

#### Hypothesis 3: Async Context Explosion
The specific interaction between:
- String concatenation expressions (`"  - " with non_flag`)
- Function call expressions (`substring of arg and 0 and 1`)
- Conditional evaluation in nested structure
- Loop variable binding

...may create an exponential number of async contexts or cause recursive evaluation cycles.

## Root Cause

**Most Probable Root Cause**: **Excessive Async Recursion in Complex Expression Evaluation**

The WFL interpreter's async architecture, while designed to handle recursion with `Box::pin`, appears to encounter a stack overflow when processing the specific combination of:

1. **Deep nested conditional structure** (4 levels of `check...otherwise...end check`)
2. **Complex expressions within loops** (`for each` with function calls and concatenation)
3. **Variable mutation within nested contexts** (`change` statements)
4. **String concatenation in display statements** (`display "..." with variable`)

The recursion occurs during expression evaluation where each `with` concatenation, `substring` function call, and variable access triggers additional async function calls, and the combination of all these in the specific nested structure exceeds the stack limit.

## Impact Assessment

### Severity: **HIGH**
- Causes complete program termination with stack overflow
- Affects any WFL program processing command-line arguments with `--` flags
- No graceful error handling or recovery possible

### Scope: **MEDIUM**  
- Specific to complex nested conditional structures combined with:
  - Command-line argument processing
  - String concatenation operations
  - Function calls within loops
  - List manipulation

### User Impact: **HIGH**
- Any WFL program that processes command-line flags will crash
- Makes WFL unsuitable for command-line tools
- No workaround available for complex argument parsing

## Recommended Investigation Areas

### 1. Async Stack Frame Management
**File**: `C:\logbie\wfl\src\interpreter\mod.rs`
**Functions**: 
- `evaluate_expression()` (line 2883)
- `_evaluate_expression()` (line 2893)  
- `execute_block()` (line 2840)
- `call_function()` (line 3737)

**Investigation**: Examine if `Box::pin` usage is causing stack frame accumulation in complex nested scenarios.

### 2. Expression Concatenation Logic
**File**: `C:\logbie\wfl\src\interpreter\mod.rs`  
**Function**: Expression::Concatenation handler (line 3335)

**Investigation**: Check if recursive `evaluate_expression` calls in concatenation chains are properly tail-call optimized.

### 3. For-Each Loop Implementation  
**File**: `C:\logbie\wfl\src\interpreter\mod.rs`
**Lines**: 1248-1324 (for-each loop execution)

**Investigation**: Verify environment creation and cleanup in nested loop contexts.

### 4. Variable Environment Management
**File**: `C:\logbie\wfl\src\interpreter\mod.rs`
**Functions**: Environment creation and variable binding in loops

**Investigation**: Check for potential circular references or excessive environment chain depth.

### 5. Conditional Statement Execution
**File**: `C:\logbie\wfl\src\interpreter\mod.rs`  
**Function**: Check statement execution logic

**Investigation**: Examine if deeply nested conditionals cause stack frame accumulation.

## Reproduction Environment

- **OS**: Windows 10/11
- **WFL Version**: v25.8.26
- **Rust Version**: Latest stable
- **Build**: Debug mode (issue may not occur in release due to optimizations)

## Test Cases for Fix Validation

1. **Basic reproduction**: `cargo run -- TestPrograms/args_comprehensive.wfl --azusa is cool`
2. **Simple flag**: `cargo run -- TestPrograms/args_comprehensive.wfl --help`
3. **Multiple flags**: `cargo run -- TestPrograms/args_comprehensive.wfl --verbose --test arg`
4. **No crash case**: `cargo run -- TestPrograms/args_comprehensive.wfl Azusa is cool`

The fix should ensure all test cases execute successfully without stack overflow while maintaining identical program behavior.