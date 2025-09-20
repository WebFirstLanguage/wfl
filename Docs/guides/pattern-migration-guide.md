# WFL Pattern Migration Guide

This guide helps you migrate from the old WFL regex system to the new natural language pattern matching system.

## Overview

The new pattern system represents a major upgrade from the previous regex-based implementation, offering:
- **Natural Language Syntax**: English-like pattern definitions
- **Bytecode VM**: Efficient execution with step limits
- **Named Captures**: Extract data with meaningful names
- **Basic Lookaround**: Simple lookahead/lookbehind support (with braces)
- **Improved Safety**: Step limits prevent infinite loops

## Implementation Status

**✅ Currently Implemented:**
- Basic character classes (`digit`, `letter`, `whitespace`)
- Quantifiers (`one or more`, `zero or more`, `optional`, `exactly N`, `N to M`, `at least N`, `at most N`)
- Sequences and alternatives
- Named capture groups (with braces: `capture {pattern} as name`)
- Basic lookahead/lookbehind (with braces: `followed by {pattern}`)
- Start of text anchor (`start of text`)

**❌ Not Yet Implemented:**
- End of text anchor (`end of text`)
- Backreferences (`same as group N`)
- Character sets (`any of "chars"`)
- Unicode categories/scripts
- Line anchors, word boundaries
- Pattern replacement/split functions

## Migration Timeline

The old regex system has been **completely removed** as of version 25.8.3. All patterns must use the new syntax.

## Syntax Changes

### Pattern Definition

**Old Regex System:**
```wfl
store pattern as regex("hello world")
store result as match(pattern, text)
```

**New Pattern System:**
```wfl
create pattern greeting:
    "hello world"
end pattern

store result as text matches greeting
```

### Basic Character Matching

**Old:**
```wfl
store digit_pattern as regex("[0-9]")
store letter_pattern as regex("[a-zA-Z]")
store word_pattern as regex("\\w+")
```

**New:**
```wfl
create pattern single_digit:
    digit
end pattern

create pattern single_letter:  
    letter
end pattern

create pattern word:
    one or more letter
end pattern
```

### Quantifiers

**Old:**
```wfl
store optional_pattern as regex("colou?r")
store multiple_pattern as regex("\\d+")
store any_pattern as regex(".*")
```

**New:**
```wfl
create pattern color_spelling:
    "colo" optional "u" "r"
end pattern

create pattern multiple_digits:
    one or more digit
end pattern

create pattern any_characters:
    zero or more any
end pattern
```

### Character Classes

**Old:**
```wfl
store hex_pattern as regex("[0-9A-Fa-f]+")
store vowel_pattern as regex("[aeiouAEIOU]")
```

**New:**
```wfl
// ❌ Character sets not implemented - use alternatives instead
create pattern hex_digits:
    one or more (
        digit or "A" or "B" or "C" or "D" or "E" or "F" or
        "a" or "b" or "c" or "d" or "e" or "f"
    )
end pattern

create pattern vowels:
    "a" or "e" or "i" or "o" or "u" or
    "A" or "E" or "I" or "O" or "U"
end pattern
```

### Alternatives (OR)

**Old:**
```wfl
store choice_pattern as regex("cat|dog|bird")
```

**New:**
```wfl
create pattern pets:
    "cat" or "dog" or "bird"
end pattern
```

### Anchors

**Old:**
```wfl
store start_pattern as regex("^Hello")
store end_pattern as regex("world$")
store full_pattern as regex("^complete match$")
```

**New:**
```wfl
create pattern starts_with_hello:
    start of text then "Hello"
end pattern

// ❌ End of text anchor not yet implemented
// create pattern ends_with_world:
//     "world" then end of text
// end pattern

// ❌ Full match requires end anchor (not implemented)
// create pattern exact_match:
//     start of text then "complete match" then end of text
// end pattern
```

## Advanced Feature Migration

### Named Capture Groups

**Old:**
```wfl
store email_pattern as regex("(?P<user>[a-zA-Z0-9]+)@(?P<domain>[a-zA-Z0-9.]+)")
```

**New:**
```wfl
create pattern email:
    capture {one or more letter or digit} as user
    "@"
    capture {one or more letter or digit or "."} as domain
end pattern
```

### Backreferences

**Old:**
```wfl
store repeat_pattern as regex("(\\w+)\\s+\\1")
```

**New:**
```wfl
// ❌ Backreferences not yet implemented
// create pattern repeated_word:
//     capture {one or more letter} as word
//     whitespace
//     same as captured word
// end pattern

// Workaround: Use separate matching logic
create pattern word_pattern:
    capture {one or more letter} as word
end pattern

// Then check manually in code if words match
```

### Lookahead Assertions

**Old:**
```wfl
store positive_lookahead as regex("\\d(?=\\w)")
store negative_lookahead as regex("\\d(?!\\w)")
```

**New:**
```wfl
create pattern digit_before_letter:
    digit followed by {letter}
end pattern

create pattern digit_not_before_letter:
    digit not followed by {letter}
end pattern
```

### Lookbehind Assertions

**Old:**
```wfl
store positive_lookbehind as regex("(?<=\\w)\\d")
store negative_lookbehind as regex("(?<!\\w)\\d")
```

**New:**
```wfl
create pattern digit_after_letter:
    preceded by {letter} then digit
end pattern

create pattern digit_not_after_letter:
    not preceded by {letter} then digit
end pattern
```

## Function API Changes

### Matching Functions

**Old API:**
```wfl
store pattern as regex("hello")
store is_match as match(pattern, text)
store first_match as find(pattern, text)  
store all_matches as find_all(pattern, text)
```

**New API:**
```wfl
create pattern greeting:
    "hello"
end pattern

store is_match as text matches greeting
store first_match as find greeting in text
// store all_matches as find_all greeting in text  // ❌ Not yet implemented
```

### Replacement Functions

**Old API:**
```wfl
store pattern as regex("\\d+")
store result as replace(pattern, text, "NUMBER")
```

**New API:**
```wfl
create pattern numbers:
    one or more digit
end pattern

// ❌ Replace functionality not yet implemented
// Use string replace functions as workaround
store result as replace(text, "123", "NUMBER")  // Manual replacement
```

### Split Functions

**Old API:**
```wfl
store pattern as regex(",\\s*")
store parts as split(pattern, text)
```

**New API:**
```wfl
create pattern comma_separator:
    ","
    zero or more whitespace
end pattern

// ❌ Split functionality not yet implemented
// Use string split functions as workaround
store parts as split(text, ",")  // Manual splitting
```

## Unicode Migration

**❌ Unicode categories and scripts are not yet implemented.**

### Current ASCII-Only Approach

**Old:**
```wfl
store word_pattern as regex("[a-zA-Z]+")
store number_pattern as regex("[0-9]+")
```

**New (ASCII-Only for now):**
```wfl
create pattern ascii_word:
    one or more letter
end pattern

create pattern ascii_number:
    one or more digit
end pattern
```

### International Text Support

**Current Limitation:**
```wfl
// ❌ Unicode categories not implemented
// create pattern international_name:
//     one or more {unicode category "Letter"}
// end pattern

// ✅ Workaround: Use basic character classes
create pattern basic_name:
    one or more letter or whitespace or "'" or "-"
end pattern
```

## Error Handling Changes

### Old Error Handling

**Old:**
```wfl
try:
    store pattern as regex("invalid[")
catch error:
    display "Regex compilation failed: " with error
end try
```

**New Error Handling:**
```wfl
create pattern test:
    // Pattern syntax errors are caught at parse time
    // Runtime errors are handled by the VM
    one or more digit
end pattern

try:
    store result as find test in text
catch error:
    display "Pattern execution failed: " with error  
end try
```

## Performance Considerations

### Old System Performance Issues

The old regex system had several performance problems:
- Catastrophic backtracking (ReDoS vulnerabilities)
- No execution time limits
- Inefficient Unicode handling

### New System Improvements

The new pattern system addresses these issues:
- **Step Limits**: Prevents ReDoS attacks (100,000 step limit)
- **Bytecode VM**: More efficient execution
- **Unicode Optimization**: Proper Unicode character handling
- **Memory Safety**: Bounds checking prevents crashes

### Migration Performance Tips

1. **Replace Complex Regex with Natural Language:**
   ```wfl
   // Old: Complex and hard to read
   store pattern as regex("^(?=.*[a-z])(?=.*[A-Z])(?=.*\\d).{8,}$")
   
   // New: Clear and maintainable  
   create pattern strong_password:
       start of text
       check ahead for {any until {lowercase}}
       check ahead for {any until {uppercase}}  
       check ahead for {any until digit}
       at least 8 any
       end of text
   end pattern
   ```

2. **Use Specific Character Classes:**
   ```wfl
   // More efficient
   create pattern specific:
       unicode category "Letter"
   end pattern
   
   // Less efficient  
   create pattern broad:
       any
   end pattern
   ```

## Common Migration Patterns

### Email Validation

**Old:**
```wfl
store email_pattern as regex("^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$")
```

**New:**
```wfl
create pattern email:
    start of text then
    one or more letter or digit or "." or "_" or "%" or "+" or "-"
    then "@" then
    one or more letter or digit or "." or "-"
    then "." then
    at least 2 letter
    // Note: end of text anchor not yet implemented
end pattern
```

### URL Validation

**Old:**
```wfl
store url_pattern as regex("^https?://[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}(/.*)?$")
```

**New:**
```wfl
create pattern url:
    start of text
    "http" optional "s" "://"
    one or more {letter or digit or "." or "-"}
    "."
    at least 2 letter
    optional {
        "/"
        zero or more any
    }
    end of text
end pattern
```

### Phone Number

**Old:**
```wfl
store phone_pattern as regex("^\\+?[1-9]\\d{1,14}$")
```

**New:**
```wfl
create pattern phone:
    start of text then
    optional "+" then
    ("1" or "2" or "3" or "4" or "5" or "6" or "7" or "8" or "9") then
    1 to 14 digit
    // Note: end of text anchor not yet implemented
end pattern
```

## Testing Migration

### Validation Approach

1. **Create Test Cases:**
   ```wfl
   store test_cases as [
       ["hello", true],
       ["world", false],
       ["hello world", true]
   ]
   
   create pattern greeting:
       "hello"
   end pattern
   
   for each test_case in test_cases:
       store input as test_case[0]
       store expected as test_case[1]
       store actual as input matches greeting
       
       check if actual equals expected:
           display "✓ " with input
       otherwise:
           display "✗ " with input with " (expected " with expected with ", got " with actual with ")"
       end check
   end for
   ```

2. **Compare Results:**
   ```wfl
   // Test that old and new patterns produce same results
   create pattern new_digit:
       digit
   end pattern
   
   store old_pattern as regex("\\d")
   
   store test_strings as ["1", "a", "9", "x"]
   for each test_string in test_strings:
       store old_result as match(old_pattern, test_string)  
       store new_result as test_string matches new_digit
       
       check if old_result equals new_result:
           display "✓ Results match for: " with test_string
       otherwise:
           display "✗ Results differ for: " with test_string
       end check
   end for
   ```

## Troubleshooting Common Issues

### Issue: Pattern Not Matching

**Problem:** Pattern that worked with regex doesn't match with new syntax.

**Solution:** Check syntax differences:
```wfl
// Wrong: Using regex escapes
create pattern wrong:
    "\\d+"
end pattern

// Correct: Using natural language
create pattern correct:
    one or more digit
end pattern
```

### Issue: Capture Group Names

**Problem:** Capture group access has changed.

**Solution:** Use new capture syntax:
```wfl
// Old style (no longer works)
store matches as find(pattern, text)
store user as matches.group("user")

// New style
store matches as find pattern in text  
store user as captured "user" from matches
```

### Issue: Performance Degradation

**Problem:** Patterns run slower than expected.

**Solution:** Optimize pattern structure:
```wfl
// Inefficient: Excessive backtracking
create pattern slow:
    zero or more {zero or more any "x"}
    "xyz"
end pattern

// Efficient: More direct approach
create pattern fast:
    zero or more {not "x"}
    "xyz"  
end pattern
```

## Getting Help

If you encounter migration issues:

1. **Check Documentation:** Review the [Pattern Reference](../language-reference/wfl-patterns.md) and [Standard Library Patterns](../api/wfl-standard-library.md)
2. **Test Incrementally:** Migrate patterns one at a time
3. **Use Debug Mode:** Run patterns with `--debug` flag to see execution traces
4. **Create Minimal Examples:** Isolate problematic patterns for testing

## Future Compatibility

The new pattern system is designed to be stable and backward-compatible. Future additions will not break existing patterns, ensuring your migration effort is a long-term investment.

### Planned Features

- Pattern replacement functions (`replace_pattern`)
- Pattern split functions (`split_pattern`)  
- Additional Unicode property support
- Performance optimizations
- More natural language constructs

Your migrated patterns will automatically benefit from these improvements without requiring changes.