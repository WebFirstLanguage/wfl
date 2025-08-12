# Bug Report: Array Indexing Parser Limitation

## Issue Summary
The WFL parser does not correctly handle array indexing syntax `array[index]`, causing variables used as array indices to be incorrectly flagged as unused by the static analyzer.

## Affected Code
**File**: `TestPrograms/args_comprehensive.wfl`  
**Lines**: 82-83
```wfl
store last_index as arg_count minus 1
display "  Last: " with args[last_index]
```

## Expected Behavior
- `args[last_index]` should access the element at the specified index
- Output should show: `Last: test` (the last command line argument)
- `last_index` should not be flagged as unused since it's used in the array access

## Actual Behavior
- `args[last_index]` is parsed as just `args` (entire array)
- Output shows: `Last: [This, is, a, cool, test]` (entire array)
- Static analyzer correctly reports `last_index` as unused (since it's never actually used in the parsed AST)

## Root Cause Analysis

### Parser Investigation
The WFL parser fails to generate `IndexAccess` AST nodes for array indexing syntax. Instead:

1. **Expected AST**: `Expression::IndexAccess { collection: args, index: last_index }`
2. **Actual AST**: `Expression::Variable(args)` 

### Evidence
Testing with a minimal case:
```wfl
store my_list as ["a" and "b" and "c"]
store index as 1
display "Element: " with my_list[index]
```

**Result**: Outputs `Element: [a, b, c]` instead of `Element: b`

### Static Analyzer Behavior
The static analyzer is working correctly:
- It properly tracks variable declarations and usage
- Since `last_index` never appears in an `IndexAccess` expression (due to parser limitation), it's correctly flagged as unused
- The analyzer's logic for `IndexAccess` expressions is correct and tested

## Technical Details

### Debug Process
1. **Initial hypothesis**: Analyzer bug in for-each loop variable tracking
2. **Investigation**: Added debug logging to trace variable usage detection
3. **Discovery**: `args[last_index]` is parsed as `Expression::Variable("args")` in concatenation
4. **Confirmation**: Created minimal test case demonstrating parser limitation

### Test Coverage
Added comprehensive test `test_variable_used_in_array_access` that verifies the analyzer correctly handles variables in `IndexAccess` expressions when properly parsed.

## Impact
- **Severity**: Medium - Functional limitation affecting array operations
- **Scope**: All array indexing operations in WFL
- **Workaround**: None available for array indexing syntax

## Resolution Required
This requires a **parser fix**, not an analyzer fix. The WFL parser needs to be updated to:

1. Correctly recognize array indexing syntax `array[index]`
2. Generate appropriate `IndexAccess` AST nodes
3. Ensure the AST structure matches the expected semantics

## Files Modified (During Investigation)
- `src/analyzer/static_analyzer.rs`: Added test case, temporary debug logging (removed)
- `TestPrograms/args_comprehensive.wfl`: No changes (issue confirmed in original file)

## Test Results
- All existing analyzer tests pass
- New test case passes (verifies analyzer works correctly with proper AST)
- No regressions introduced

## Recommendation
Prioritize fixing the parser to properly handle array indexing syntax, as this is a fundamental language feature that affects both functionality and developer experience.