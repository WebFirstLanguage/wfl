# Bug Report: Incomplete Builtin Function Arity Definitions

## Bug Summary
The WFL typechecker has incomplete builtin function arity definitions, causing false argument-count errors. When builtin function names are converted to Function types, they use a small hardcoded match set to determine param_count, defaulting to 1 for unknown functions. This causes many builtin functions that require 2 or more arguments to incorrectly report type errors claiming they only expect 1 argument.

## Evidence Collected

### 1. Problematic Code Location
**File**: `C:\logbie\wfl\src\typechecker\mod.rs`
**Lines**: 1230-1237

```rust
let param_count = match name.as_str() {
    "substring" => 3, // text, start, length
    "replace" => 3,   // text, old, new
    "clamp" => 3,     // value, min, max
    "padleft" | "padright" => 2, // text, length
    "indexof" | "index_of" | "lastindexof" | "last_index_of" => 2, // text, substring
    _ => 1, // Most functions take 1 parameter
};
```

### 2. Current Match Cases and Their Param Counts
Only 6 functions have correct arity definitions:
- `substring`: 3 arguments (text, start, length)
- `replace`: 3 arguments (text, old, new)
- `clamp`: 3 arguments (value, min, max)
- `padleft`, `padright`: 2 arguments (text, length)
- `indexof`, `index_of`, `lastindexof`, `last_index_of`: 2 arguments (text, substring)
- **All others default to 1 argument**

### 3. Complete List of Builtin Functions by Module

**From `C:\logbie\wfl\src\builtins.rs` (Complete Registry)**:

#### Implemented Functions (with verified arities):

**Math Module** (`src/stdlib/math.rs`):
- `abs`: 1 argument ✓ (correctly defaulted)
- `round`: 1 argument ✓ (correctly defaulted)
- `floor`: 1 argument ✓ (correctly defaulted)
- `ceil`: 1 argument ✓ (correctly defaulted)
- `random`: 0 arguments ❌ (incorrectly defaulted to 1)
- `clamp`: 3 arguments ✓ (correctly defined)

**Text Module** (`src/stdlib/text.rs`):
- `touppercase`, `to_uppercase`: 1 argument ✓ (correctly defaulted)
- `tolowercase`, `to_lowercase`: 1 argument ✓ (correctly defaulted)
- `contains`: 2 arguments ❌ (incorrectly defaulted to 1)
- `substring`: 3 arguments ✓ (correctly defined)

**List Module** (`src/stdlib/list.rs`):
- `length`: 1 argument ✓ (correctly defaulted)
- `push`: 2 arguments ❌ (incorrectly defaulted to 1)
- `pop`: 1 argument ✓ (correctly defaulted)
- `contains`: 2 arguments ❌ (incorrectly defaulted to 1)
- `indexof`, `index_of`: 2 arguments ✓ (correctly defined)

#### Recognized but NOT Implemented Functions:

**Math Functions** (listed in builtins but not implemented):
- `min`: **2+ arguments** ❌ (incorrectly defaulted to 1)
- `max`: **2+ arguments** ❌ (incorrectly defaulted to 1)
- `power`: **2 arguments** ❌ (incorrectly defaulted to 1)
- `sqrt`, `sin`, `cos`, `tan`: 1 argument ✓ (correctly defaulted)

**Text Functions** (listed in builtins but not implemented):
- `replace`: 3 arguments ✓ (correctly defined)
- `trim`: 1 argument ✓ (correctly defaulted)
- `padleft`, `padright`: 2 arguments ✓ (correctly defined)
- `capitalize`, `reverse`: 1 argument ✓ (correctly defaulted)
- `startswith`, `starts_with`: **2 arguments** ❌ (incorrectly defaulted to 1)
- `endswith`, `ends_with`: **2 arguments** ❌ (incorrectly defaulted to 1)
- `split`: **2 arguments** ❌ (incorrectly defaulted to 1)
- `join`: **2 arguments** ❌ (incorrectly defaulted to 1)

**List Functions** (listed in builtins but not implemented):
- Most list operations: varying arities, mostly defaulted incorrectly

## Reproduction Steps

1. Create a simple WFL program with multi-argument builtin function calls:
```wfl
store a as 10
store b as 5
display min of a and b
```

2. Run with: `./target/release/wfl.exe test.wfl`

3. Observe the type checking error:
```
Type checking warnings:
error: Function expects 1 arguments, but 2 were provided
```

## Analysis

### Root Cause Investigation

The issue stems from a **hardcoded lookup table** approach in the typechecker that only covers a small subset of builtin functions. The code at lines 1230-1237 in `src/typechecker/mod.rs` uses a match statement to determine parameter counts, but it only handles 6 specific cases and defaults everything else to 1 argument.

### Key Findings

1. **Incomplete Coverage**: Only 6 out of 80+ builtin functions have correct arity definitions
2. **Default is Wrong**: The default of 1 argument is incorrect for many functions
3. **Disconnect**: The arity definitions are disconnected from the actual function implementations
4. **Scale of Impact**: This affects most multi-argument builtin functions

### Impact Assessment

**Severity**: High - Prevents use of essential builtin functions

**Scope**: Affects all builtin functions requiring 2+ arguments that are not in the hardcoded list:

**Incorrectly Defaulted Functions (Major Impact)**:
- `min`, `max`: Basic math operations requiring 2+ arguments
- `power`: Exponentiation requiring 2 arguments  
- `contains`: Text/list searching requiring 2 arguments
- `push`: List manipulation requiring 2 arguments
- `starts_with`, `ends_with`: Text checking requiring 2 arguments
- `split`, `join`: String manipulation requiring 2 arguments
- `random`: Actually requires 0 arguments but defaults to 1

**Test Case Evidence**: The test program `TestPrograms/stdlib_comprehensive.wfl` contains multiple calls to these functions that would trigger this bug, including:
- `min of a and b`
- `max of a and b` 
- `power of 5 and 2`
- `contains of sample_text and search_text`

## Root Cause

The fundamental issue is that **parameter count determination is hardcoded and incomplete**. The typechecker attempts to create Function types for builtin functions using a small lookup table, but this table covers less than 10% of the declared builtin functions.

The problem occurs in the identifier type checking logic where builtin function names are converted to Function types. The param_count is determined by a match statement that only handles a few specific cases and defaults to 1 for everything else.

## Recommended Investigation Areas

### 1. Immediate Fix Areas
- **File**: `C:\logbie\wfl\src\typechecker\mod.rs`, lines 1230-1237
- **Issue**: Expand the match statement to include correct arities for all builtin functions
- **Priority**: High - This is the core bug location

### 2. Design Improvements
- **Synchronization**: Create a centralized arity registry that both the typechecker and interpreter can use
- **Validation**: Ensure arity definitions match actual function implementations
- **Testing**: Add comprehensive tests for builtin function arity validation

### 3. Implementation Gaps
- **Missing Functions**: Many builtin functions are declared but not implemented (`min`, `max`, `power`, etc.)
- **File**: Various stdlib modules need to implement missing functions
- **Priority**: Medium - These are functionality gaps beyond the arity bug

### 4. Test Coverage
- **File**: Need specific tests for builtin function arity validation
- **Focus**: Create test cases that specifically verify correct argument count handling
- **Location**: Add to existing test suites or create new arity-specific tests

## Suggested Fix Approach

### Minimal Expansion (Quick Fix)
Expand the match statement in `src/typechecker/mod.rs` to include the most commonly used multi-argument functions:

```rust
let param_count = match name.as_str() {
    // Existing entries...
    "substring" => 3,
    "replace" => 3,   
    "clamp" => 3,
    "padleft" | "padright" => 2,
    "indexof" | "index_of" | "lastindexof" | "last_index_of" => 2,
    
    // Critical additions:
    "min" | "max" => 2,  // Note: Actually variadic but minimum 2
    "power" => 2,
    "contains" => 2,
    "push" => 2,
    "starts_with" | "startswith" => 2,
    "ends_with" | "endswith" => 2,
    "split" | "join" => 2,
    "random" => 0,
    
    _ => 1,
};
```

### Comprehensive Solution (Recommended)
Create a centralized arity registry that can be shared between the typechecker and function implementations, eliminating the possibility of drift between the two systems.

This bug significantly impacts the usability of WFL's builtin functions and should be prioritized for fixing.