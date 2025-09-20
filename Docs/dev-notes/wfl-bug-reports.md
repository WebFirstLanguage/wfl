# WFL Bug Reports and Investigations

This document contains historical bug reports and investigations that have been resolved or are being tracked.

## Stack Overflow Bug (Resolved)

### Bug Summary

A stack overflow occurred when running WFL programs that process command line arguments containing flags starting with `--` during the "Flag/Option Parsing" section. The bug was specifically triggered when executing nested conditional logic that handled flag detection and list operations in a complex program structure.

**Command that triggered bug**: `cargo run -- args_comprehensive.wfl --azusa is cool`  
**Command that worked**: `cargo run -- args_comprehensive.wfl Azusa is cool`

### Root Cause

**Excessive Async Recursion in Complex Expression Evaluation**

The WFL interpreter's async architecture encountered a stack overflow when processing the specific combination of:

1. **Deep nested conditional structure** (4 levels of `check...otherwise...end check`)
2. **Complex expressions within loops** (`for each` with function calls and concatenation)
3. **Variable mutation within nested contexts** (`change` statements)
4. **String concatenation in display statements** (`display "..." with variable`)

The recursion occurred during expression evaluation where each `with` concatenation, `substring` function call, and variable access triggered additional async function calls, and the combination of all these in the specific nested structure exceeded the stack limit.

### Impact Assessment

- **Severity**: HIGH - Caused complete program termination with stack overflow
- **Scope**: MEDIUM - Specific to complex nested conditional structures
- **User Impact**: HIGH - Made WFL unsuitable for command-line tools

### Resolution

This bug was resolved through improvements to the async recursion handling in the interpreter, specifically in the `evaluate_expression()` and `execute_block()` functions. The fix involved better stack frame management and optimization of the async execution pipeline.

### Test Cases for Validation

1. **Basic reproduction**: `cargo run -- TestPrograms/args_comprehensive.wfl --azusa is cool`
2. **Simple flag**: `cargo run -- TestPrograms/args_comprehensive.wfl --help`
3. **Multiple flags**: `cargo run -- TestPrograms/args_comprehensive.wfl --verbose --test arg`
4. **No crash case**: `cargo run -- TestPrograms/args_comprehensive.wfl Azusa is cool`

---

## Memory Leak Investigation (Resolved)

### Root Cause Diagnosis

Heap profiling revealed extremely high memory usage (peaking around 10.7 GB) and excessive allocations (over 152 million) when running comprehensive test scripts.

### Primary Issues Identified

1. **Excessive Allocations in Parsing**: The parser's expression parsing routines caused disproportionate heap allocations
2. **Reference Cycle Between Environment and Function**: Rc reference cycle involving `Environment` and `FunctionValue`
3. **Unbounded Growth of Data Structures**: Collections growing without bound during execution

### Resolution Strategy

1. **Break reference cycles** by converting strong references to weak references
2. **Optimize parser memory usage** through better allocation patterns
3. **Limit debug output** to prevent memory explosion
4. **Implement proper cleanup** for temporary data structures

### Key Improvements Made

- Changed `FunctionValue::env` from `Rc<RefCell<Environment>>` to `Weak<RefCell<Environment>>`
- Implemented string interning for identifiers and keywords
- Added truncation limits for debug output
- Optimized vector allocations with proper capacity reservation

---

## Historical Context

These bug reports represent significant milestones in WFL's development and demonstrate the project's commitment to thorough investigation and resolution of complex issues. The detailed analysis and systematic approach to debugging have contributed to WFL's current stability and performance.

For current bug reports and issues, please use the [GitHub Issues](https://github.com/WebFirstLanguage/wfl/issues) system.
