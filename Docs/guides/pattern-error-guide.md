# Pattern Matching Error Guide

This guide covers common pattern matching errors in WFL, their causes, and solutions. All examples are based on actual parser behavior and testing.

## Table of Contents
- [Syntax Errors](#syntax-errors)
- [Runtime Errors](#runtime-errors)
- [Logic Errors](#logic-errors)
- [Performance Issues](#performance-issues)
- [Debugging Tips](#debugging-tips)

## Syntax Errors

### "Unexpected token in pattern" Errors

#### Error: KeywordBetween
```
error[ERROR]: Unexpected token in pattern: KeywordBetween
```

**Cause:** Using `between N and M` syntax which is not implemented.

**❌ Wrong:**
```wfl
create pattern wrong:
    between 2 and 4 letter
end pattern
```

**✅ Correct:**
```wfl
create pattern correct:
    2 to 4 letter
end pattern
```

#### Error: KeywordNot (Lookahead)
```
error[ERROR]: Unexpected token in pattern: KeywordNot
```

**Cause:** Using simple negative lookahead syntax without braces.

**❌ Wrong:**
```wfl
create pattern wrong:
    "test" not followed by "456"
end pattern
```

**✅ Correct:**
```wfl
create pattern correct:
    "test" not followed by {"456"}
end pattern
```

#### Error: Identifier("preceded")
```
error[ERROR]: Unexpected token in pattern: Identifier("preceded")
```

**Cause:** Using simple lookbehind syntax without braces.

**❌ Wrong:**
```wfl
create pattern wrong:
    preceded by "pre" then "fix"
end pattern
```

**✅ Correct:**
```wfl
create pattern correct:
    preceded by {"pre"} then "fix"
end pattern
```

### "Expected '{' after 'capture'" Errors

```
error[ERROR]: Expected '{' after 'capture'
```

**Cause:** Capture groups must use brace syntax.

**❌ Wrong:**
```wfl
create pattern wrong:
    capture one or more letter as name
end pattern
```

**✅ Correct:**
```wfl
create pattern correct:
    capture {one or more letter} as name
end pattern
```

### "Expected 'letter', 'digit', 'whitespace', or 'character' after 'any'" Errors

```
error[ERROR]: Expected 'letter', 'digit', 'whitespace', or 'character' after 'any'
```

**Cause:** Character set syntax `any of "chars"` is not implemented.

**❌ Wrong:**
```wfl
create pattern wrong:
    any of "!@#$%"
end pattern
```

**✅ Workaround:**
```wfl
create pattern workaround:
    "!" or "@" or "#" or "$" or "%"
end pattern
```

### "Unexpected end of pattern" Errors

```
error[ERROR]: Unexpected end of pattern
```

**Cause:** Pattern definition is incomplete or has syntax errors.

**❌ Wrong:**
```wfl
create pattern incomplete:
    one or more
end pattern
```

**✅ Correct:**
```wfl
create pattern complete:
    one or more letter
end pattern
```

## Runtime Errors

### Pattern Matching Failures

**Issue:** Pattern doesn't match expected input.

**Example:**
```wfl
create pattern strict_email:
    one or more letter then "@" then one or more letter then ".com"
end pattern

store email as "user@example.org"  // Won't match - ends with .org, not .com
check if email matches strict_email:
    display "Matched"
otherwise:
    display "No match - check pattern specificity"
end check
```

**Solutions:**
1. Make patterns more flexible
2. Use alternatives for different cases
3. Test with actual data

**✅ Better:**
```wfl
create pattern flexible_email:
    one or more letter then "@" then one or more letter then "." then 2 to 4 letter
end pattern
```

### Capture Group Issues

**Issue:** Captures return `nothing` unexpectedly.

**Example:**
```wfl
create pattern optional_capture:
    capture {optional "Mr. " or "Ms. "} as title
    capture {one or more letter} as name
end pattern

store text as "John"  // No title present
store result as find optional_capture in text
check if result is not nothing:
    // title capture will be nothing because it's optional and not present
    check if result.captures.title is not nothing:
        display "Title: " with result.captures.title
    otherwise:
        display "No title found"
    end check
    display "Name: " with result.captures.name
end check
```

**Solution:** Always check captures for `nothing` before using.

## Logic Errors

### Greedy vs Non-Greedy Matching

**Issue:** Pattern matches more or less than expected.

**Example:**
```wfl
create pattern greedy_problem:
    one or more any character then ".txt"
end pattern

store filename as "document.txt.backup"
// This will match the entire string, not just "document.txt"
```

**Solution:** Be more specific about what you want to match.

**✅ Better:**
```wfl
create pattern specific_filename:
    one or more letter or digit or "_" or "-" then ".txt"
end pattern
```

### Anchor Misunderstanding

**Issue:** Forgetting that patterns match anywhere in text by default.

**Example:**
```wfl
create pattern phone_digits:
    exactly 10 digit
end pattern

store text as "Call me at 5551234567 today"
// This matches the 10 digits in the middle, not requiring the entire string
```

**Solution:** Use anchors when you need exact matches.

**✅ Better:**
```wfl
create pattern exact_phone:
    at start of text then exactly 10 digit then at end of text
end pattern
```

## Performance Issues

### Catastrophic Backtracking

**Issue:** Pattern takes too long to execute.

**❌ Problematic:**
```wfl
// This could be slow with certain inputs
create pattern nested_optional:
    optional optional optional "a" then "b"
end pattern
```

**✅ Better:**
```wfl
create pattern simpler:
    optional "a" then "b"
end pattern
```

### Overly General Patterns

**Issue:** Pattern matches too much, causing performance problems.

**❌ Slow:**
```wfl
create pattern too_general:
    zero or more any character then "end"
end pattern
```

**✅ Faster:**
```wfl
create pattern more_specific:
    zero or more letter or digit or " " then "end"
end pattern
```

## Debugging Tips

### 1. Test Patterns Incrementally

Start simple and add complexity:

```wfl
// Step 1: Test basic structure
create pattern email_step1:
    one or more letter
end pattern

// Step 2: Add @ symbol
create pattern email_step2:
    one or more letter then "@"
end pattern

// Step 3: Add domain
create pattern email_step3:
    one or more letter then "@" then one or more letter
end pattern

// Step 4: Add TLD
create pattern email_final:
    one or more letter then "@" then one or more letter then "." then 2 to 4 letter
end pattern
```

### 2. Use Multiple Test Cases

```wfl
create pattern test_pattern:
    exactly 3 digit
end pattern

// Test various inputs
store test_cases as create list:
    add "123"      // Should match
    add "12"       // Should not match
    add "1234"     // Should match (first 3 digits)
    add "abc"      // Should not match
    add ""         // Should not match
end list

count from each test_case in test_cases:
    check if test_case matches test_pattern:
        display test_case with " ✓ matches"
    otherwise:
        display test_case with " ✗ no match"
    end check
end count
```

### 3. Isolate Problem Areas

When a complex pattern fails, test each part separately:

```wfl
// Complex pattern that's failing
create pattern complex:
    capture {one or more letter} as first
    " "
    capture {one or more letter} as last
    " - "
    capture {exactly 3 digit} as id
end pattern

// Test each part individually
create pattern test_first:
    one or more letter
end pattern

create pattern test_space:
    " "
end pattern

create pattern test_last:
    one or more letter
end pattern

create pattern test_separator:
    " - "
end pattern

create pattern test_id:
    exactly 3 digit
end pattern
```

### 4. Check Implementation Status

Before debugging, verify the feature is implemented:

**✅ Implemented:**
- Basic character classes (`digit`, `letter`, `whitespace`)
- Quantifiers (`one or more`, `zero or more`, `optional`, `exactly N`, `N to M`)
- Sequences and alternatives
- Capture groups with braces
- Lookahead/lookbehind with braces

**❌ Not Implemented:**
- Backreferences (`same as group N`)
- Character sets (`any of "chars"`)
- Unicode categories/scripts
- Simple lookahead/lookbehind (without braces)

## Error Prevention Checklist

Before creating a pattern:

- [ ] Check if all features you need are implemented
- [ ] Use correct syntax (`N to M` not `between N and M`)
- [ ] Put capture groups in braces: `capture {pattern} as name`
- [ ] Use braces for lookahead/lookbehind: `followed by {pattern}`
- [ ] Test with multiple inputs including edge cases
- [ ] Check captures for `nothing` before using
- [ ] Consider using anchors for exact matches

## Getting Help

1. **Check Implementation Status**: See `Docs/dev-notes/pattern-implementation-analysis.md`
2. **Test Working Examples**: Run `TestPrograms/patterns_working_comprehensive.wfl`
3. **Review Practical Examples**: See `Docs/guides/pattern-practical-examples.md`
4. **Check Parser Errors**: Look at the exact error message and line number

## See Also

- [Pattern Matching Reference](../language-reference/wfl-patterns.md)
- [Practical Examples Guide](pattern-practical-examples.md)
- [Implementation Analysis](../dev-notes/pattern-implementation-analysis.md)
