# Command-Line Arguments Implementation

**Date**: June 27, 2025
**Author**: Claude

## Summary

Implemented comprehensive command-line argument parsing for WFL scripts, allowing scripts to accept and process arguments passed after the script filename.

## Changes Made

### 1. Updated main.rs
- Modified argument parsing logic to separate WFL interpreter flags from script arguments
- Collect remaining arguments after the script filename as `script_args`
- Pass script arguments to the interpreter instance

### 2. Enhanced Interpreter
- Added `script_args` field to Interpreter struct
- Added `set_script_args()` method to set arguments
- Modified `interpret()` method to parse arguments and set up environment variables:
  - `arg_count`: Total number of arguments
  - `args`: List of all arguments
  - `positional_args`: List of non-flag arguments
  - `flag_*`: Individual flag variables with values

### 3. Argument Parsing Logic
- Long flags (--flag) and short flags (-f) are supported
- Flags can have values (e.g., --output file.txt)
- Boolean flags without values are set to `true`
- Positional arguments are collected separately
- Flag values are consumed greedily

### 4. Test Programs Created
- `args_test.wfl`: Comprehensive test showing all argument features
- `args_simple.wfl`: Simple test for basic argument access
- `args_example.wfl`: Practical example with flag handling
- `args_test_minimal.wfl`: Minimal test for debugging

### 5. Documentation
- Created `Docs/wfl-args.md` with complete documentation
- Includes usage examples and best practices

## Technical Details

### Argument Parsing Algorithm
```rust
// Parse flags and positional arguments
let mut flags = HashMap::new();
let mut positional_args = Vec::new();
let mut i = 0;

while i < self.script_args.len() {
    let arg = &self.script_args[i];
    if arg.starts_with("--") {
        // Long flag
        let flag_name = arg.trim_start_matches("--");
        if next_arg_exists && !next_arg.starts_with("-") {
            flags.insert(flag_name, next_arg_value);
            i += 2;
        } else {
            flags.insert(flag_name, true);
            i += 1;
        }
    } else if arg.starts_with("-") {
        // Short flag (similar logic)
    } else {
        // Positional argument
        positional_args.push(arg);
        i += 1;
    }
}
```

### Environment Setup
Arguments are made available as WFL variables:
- Direct access via predefined variables
- No need for special functions
- Undefined flags don't cause errors at runtime

## Challenges and Solutions

1. **Lexer Issues**: WFL doesn't support `<`, `>`, or `+` operators in strings
   - Solution: Modified test programs to use valid syntax

2. **Reserved Keywords**: `count` is a reserved keyword
   - Solution: Renamed variables (e.g., `repeat_count`)

3. **Comparison Syntax**: Must use `is` instead of `equals` or `=`
   - Solution: Updated all comparisons to use correct syntax

4. **String Concatenation**: Uses `with` keyword, not `+`
   - Solution: Updated all string concatenations

5. **Loop Syntax**: Must use `repeat while ... :` not just `while`
   - Solution: Fixed loop syntax in examples

6. **Semantic Analyzer Warnings**: Shows warnings for runtime-defined variables
   - Solution: Documented as expected behavior

## Backward Compatibility

- All existing programs continue to work unchanged
- Arguments are only parsed if provided
- No breaking changes to existing functionality

## Future Improvements

1. Add string-to-number conversion function for flag values
2. Support for `--` to separate flags from positional arguments
3. Add built-in help flag handling
4. Support for flag aliases (e.g., -v and --verbose)
5. Better handling of multiple values for same flag

## Testing

Tested with various argument combinations:
- No arguments
- Only flags
- Only positional arguments
- Mixed flags and positional arguments
- Flags with values
- Boolean flags

All tests pass and demonstrate correct functionality.