# WFL Pattern Matching Guide

## Table of Contents
- [Quick Start](#quick-start)
- [Pattern Syntax Reference](#pattern-syntax-reference)
- [Built-in Functions](#built-in-functions)
- [Common Patterns](#common-patterns)
- [Advanced Features](#advanced-features)
- [Performance & Optimization](#performance--optimization)
- [Implementation Details](#implementation-details)
- [Migration from Regex](#migration-from-regex)

## Quick Start

WFL's pattern matching system uses natural English syntax instead of traditional regex symbols, making it more readable and maintainable.

### Basic Pattern Matching

```wfl
// Simple string matching
check if "hello@example.com" matches pattern "email":
    display "Valid email!"
otherwise:
    display "Invalid email"
end check

// Custom pattern definition
create pattern greeting:
    "hello" or "hi" or "hey"
end pattern

check if "hello world" matches greeting:
    display "Found greeting!"
end check
```

### Finding and Extracting

```wfl
// Find first match
store first_match as pattern_find("Contact: user@example.com", email_pattern)

// Find all matches
store all_matches as pattern_find_all(text, email_pattern)

// Replace patterns
store cleaned as pattern_replace(text, phone_pattern, "XXX-XXX-XXXX")

// Split by pattern
store words as pattern_split(text, whitespace_pattern)
```

## Pattern Syntax Reference

### Character Classes

Character classes define what types of characters to match at a specific position in your pattern.

#### Basic Character Types

| Pattern | What it matches | Example matches | When to use |
|---------|----------------|-----------------|-------------|
| `any letter` | Any uppercase or lowercase letter (A-Z, a-z) | "a", "B", "z", "Q" | Matching names, words, or alphabetic content |
| `any digit` | Any numeric digit (0-9) | "0", "5", "9" | Matching numbers, IDs, or numeric codes |
| `any whitespace` | Spaces, tabs, newlines, carriage returns | " ", "\t", "\n" | Separating words or handling formatting |
| `any punctuation` | Common punctuation marks | ".", "!", "?", ",", ";" | Matching sentence endings or separators |
| `any character` | Literally any single character | "a", "7", "@", " ", "€" | Wildcard matching when you don't care what character appears |

**Examples:**
```wfl
// Match any single letter
check if "A" matches pattern "any letter":  // ✓ matches
check if "5" matches pattern "any letter":  // ✗ doesn't match

// Match any digit
check if "7" matches pattern "any digit":   // ✓ matches
check if "x" matches pattern "any digit":   // ✗ doesn't match
```

#### Combined Character Classes

These patterns match characters from multiple categories or specific sets.

| Pattern | What it matches | Example matches | When to use |
|---------|----------------|-----------------|-------------|
| `any letter or digit` | Letters (A-Z, a-z) OR digits (0-9) | "a", "5", "Z", "0" | Alphanumeric content like usernames or IDs |
| `any letter or digit or "_"` | Letters, digits, OR underscore | "a", "5", "_" | Variable names or identifiers in code |
| `any character not in "xyz"` | Any character EXCEPT x, y, or z | "a", "b", "1", "@" (but not "x", "y", "z") | Excluding specific characters |
| `any character from "a" to "z"` | Lowercase letters only | "a", "m", "z" (but not "A" or "Z") | Case-sensitive matching |

**Examples:**
```wfl
// Match alphanumeric characters
check if "user123" matches pattern "one or more of (any letter or digit)":  // ✓ matches
check if "user@123" matches pattern "one or more of (any letter or digit)": // ✗ contains @

// Exclude specific characters
check if "hello" matches pattern "one or more of (any character not in 'l')":  // ✗ contains 'l'
check if "world" matches pattern "one or more of (any character not in 'x')":  // ✓ no 'x' present
```

### Quantifiers

Quantifiers specify how many times a pattern should repeat. They control the "greediness" of your pattern matching.

#### Repetition Patterns

| Pattern | What it means | Example pattern | Matches | Doesn't match |
|---------|---------------|-----------------|---------|---------------|
| `zero or more` | Match 0 or unlimited times (optional and repeatable) | `zero or more letters` | "", "a", "abc", "hello" | "123", "a b" (has space) |
| `one or more` | Match at least once (required and repeatable) | `one or more digits` | "1", "42", "999999" | "", "abc", "12a" |
| `optional` | Match 0 or 1 time (may or may not exist) | `optional whitespace` | "", " " (single space) | "  " (multiple spaces) |
| `exactly N` | Match exactly N times | `exactly 3 digits` | "123", "000", "789" | "12", "1234", "abc" |
| `N to M` | Match between N and M times (inclusive) | `2 to 4 letters` | "ab", "abc", "abcd" | "a", "abcde" |
| `at least N` | Match N or more times | `at least 5 characters` | "hello", "hello world" | "hi", "test" |
| `at most N` | Match up to N times (0 to N) | `at most 10 digits` | "", "1", "1234567890" | "12345678901" |

**Practical Examples:**
```wfl
// Phone number with optional country code
create pattern phone:
    optional "+"              // Country code prefix (may or may not exist)
    one or more digits        // Required phone number digits
end pattern

// Password with length requirements
create pattern password:
    at least 8 of any character    // Minimum 8 characters
end pattern

// ZIP code (exactly 5 or 9 digits)
create pattern zip_code:
    exactly 5 digits
    optional (
        "-" then exactly 4 digits  // Optional +4 extension
    )
end pattern

// Username (3-20 alphanumeric characters)
create pattern username:
    3 to 20 of (any letter or digit or "_")
end pattern
```

**Understanding Greedy vs Non-Greedy:**
- By default, quantifiers are "greedy" - they match as much as possible
- Use `minimal` or `lazy` keywords for non-greedy matching when needed
```wfl
// Greedy: matches entire string between quotes
"one or more of any character" between quotes

// Non-greedy: matches shortest possible string
"minimal one or more of any character" between quotes
```

### Sequences and Alternatives

```wfl
// Sequence: patterns in order
"hello" then " " then "world"

// Alternatives: any of these patterns
"yes" or "no" or "maybe"

// Grouping with parentheses
("http" or "https") then "://"
```

### Anchors

```wfl
start of line     // ^
end of line       // $
start of text     // \A
end of text       // \z
word boundary     // \b
```

### Captures and Backreferences

```wfl
// Named capture groups
capture {one or more letter} as "name"
capture {digit digit digit} as "area_code"

// Backreferences
same as captured "name"

// Example: matching repeated words
create pattern duplicate_word:
    capture {one or more letter} as "word" " " same as captured "word"
end pattern
```

### Lookarounds

```wfl
// Lookahead (positive/negative)
digit check ahead for {letter}       // (?=letter) 
digit check not ahead for {letter}   // (?!letter)

// Lookbehind (positive/negative)  
digit check behind for {"$"}         // (?<=$)
digit check not behind for {"$"}     // (?<!$)
```

## Built-in Functions

### Pattern Matching Functions

| Function | Description | Example |
|----------|-------------|---------|
| `pattern_matches` | Check if text matches pattern | `pattern_matches(text, email_pattern)` |
| `pattern_find` | Find first match | `pattern_find(text, phone_pattern)` |
| `pattern_find_all` | Find all matches | `pattern_find_all(text, url_pattern)` |
| `pattern_replace` | Replace matches | `pattern_replace(text, phone_pattern, "XXX")` |
| `pattern_split` | Split by pattern | `pattern_split(text, whitespace_pattern)` |

### Pattern Creation

```wfl
// Define named patterns
create pattern email:
    one or more of (any letter or digit or "._-")
    then "@"
    then one or more of (any letter or digit or "-")
    then "."
    then 2 to 6 letters
end pattern

// Compile patterns for performance
compile pattern "email" as email_validator
```

### Standard Pattern Library

WFL includes built-in pattern functions for common use cases that can be accessed directly:

```wfl
// Available pattern functions in stdlib
pattern_matches(text, email_pattern)      // Email validation
pattern_matches(text, url_pattern)        // URL validation  
pattern_matches(text, ipv4_pattern)       // IPv4 address validation
pattern_matches(text, phone_pattern)      // Phone number validation
pattern_matches(text, uuid_pattern)       // UUID format validation
pattern_matches(text, date_pattern)       // Date format validation
```

## Common Patterns

### Email Validation

```wfl
// Basic email pattern
create pattern basic_email:
    one or more of (any letter or digit or "._-")
    then "@"
    then one or more of (any letter or digit or "-")
    then "."
    then 2 to 6 letters
end pattern

// Advanced email with full RFC compliance
create pattern rfc_email:
    one or more of (
        any letter or digit or 
        any of "!#$%&'*+-/=?^_`{|}~"
    )
    then "@"
    one or more of (any letter or digit or "-")
    one or more of (
        then "."
        one or more of (any letter or digit or "-")
    )
end pattern
```

### Phone Numbers

```wfl
// US phone number with flexible formatting
create pattern us_phone:
    optional (any of "+" or "1" then optional " ")
    optional "("
    capture 3 digits as "area"
    optional ")"
    optional any of " " or "-"
    capture 3 digits as "exchange"
    optional any of " " or "-"
    capture 4 digits as "line"
end pattern

// International phone
create pattern international_phone:
    optional "+"
    capture 1 to 3 digits as "country"
    one or more of (
        optional any of " " or "-" or "."
        then 1 to 4 digits
    )
end pattern
```

### URL Parsing

```wfl
create pattern full_url:
    // Protocol
    capture optional (
        any of "http" or "https" or "ftp"
        then "://"
    ) as "protocol"
    
    // Host
    capture (
        one or more of (any letter or digit or "-")
        one or more of (
            then "."
            one or more of (any letter or digit or "-")
        )
    ) as "host"
    
    // Port
    optional (
        then ":"
        capture one or more digits as "port"
    )
    
    // Path
    capture optional (
        then "/"
        zero or more of (any character not in "?#")
    ) as "path"
    
    // Query
    optional (
        then "?"
        capture zero or more of (any character not in "#") as "query"
    )
    
    // Fragment
    optional (
        then "#"
        capture zero or more of any character as "fragment"
    )
end pattern
```

### Date Patterns

```wfl
// ISO 8601 date
create pattern iso_date:
    capture 4 digits as "year"
    then "-"
    capture 2 digits as "month"
    then "-"
    capture 2 digits as "day"
    optional (
        then "T"
        capture 2 digits as "hour"
        then ":"
        capture 2 digits as "minute"
        optional (
            then ":"
            capture 2 digits as "second"
        )
        optional (
            any of "Z" or (
                any of "+" or "-"
                then 2 digits then ":" then 2 digits
            )
        ) as "timezone"
    )
end pattern

// Flexible date parser
create pattern flexible_date:
    // MM/DD/YYYY or MM-DD-YYYY
    capture 1 to 2 digits as "month"
    then any of "/" or "-"
    capture 1 to 2 digits as "day"
    then any of "/" or "-"
    capture 2 or 4 digits as "year"
    
    or
    
    // YYYY/MM/DD or YYYY-MM-DD
    capture 4 digits as "year"
    then any of "/" or "-"
    capture 1 to 2 digits as "month"
    then any of "/" or "-"
    capture 1 to 2 digits as "day"
end pattern
```

## Advanced Features

### Capture Groups and Extraction

```wfl
// Define pattern with captures
create pattern name_pattern:
    capture one or more letters as "first"
    then one or more whitespace
    capture one or more letters as "last"
end pattern

// Extract captured values
match "John Doe" with pattern name_pattern:
    store first_name as captured "first"
    store last_name as captured "last"
    display "First: " with first_name
    display "Last: " with last_name
end match
```

### Backreferences for Repeated Content

```wfl
// Match HTML/XML tags
create pattern html_tag:
    "<"
    capture one or more letters as "tag"
    ">"
    zero or more any character
    "</"
    same as captured "tag"
    ">"
end pattern

// Validate balanced quotes
create pattern quoted_string:
    capture any of "\"" or "'" as "quote"
    zero or more of (any character not in captured "quote")
    same as captured "quote"
end pattern
```

### Lookaround Assertions

```wfl
// Password validation with lookarounds
create pattern strong_password:
    // Must contain lowercase (positive lookahead)
    any position followed by (zero or more any character then any lowercase letter)
    
    // Must contain uppercase (positive lookahead)
    any position followed by (zero or more any character then any uppercase letter)
    
    // Must contain digit (positive lookahead)
    any position followed by (zero or more any character then any digit)
    
    // Must contain special char (positive lookahead)
    any position followed by (zero or more any character then any of "!@#$%^&*")
    
    // At least 8 characters
    at least 8 of any character
end pattern

// Find numbers not preceded by currency symbols
create pattern plain_number:
    any digit not preceded by any of "$£€¥"
    zero or more digits
end pattern
```

### Pattern Composition

```wfl
// Build complex patterns from simpler ones
create pattern word:
    one or more letters
end pattern

create pattern sentence:
    pattern "word"
    zero or more of (
        one or more whitespace
        then pattern "word"
    )
    then any of ".!?"
end pattern

// Dynamic pattern building
define action build_date_pattern:
    parameter separator as Text
    
    return pattern (
        1 to 2 digits 
        then separator 
        then 1 to 2 digits 
        then separator 
        then 2 or 4 digits
    )
end action
```

## Performance & Optimization

### Pattern Compilation and Caching

```wfl
// Compile frequently used patterns
compile pattern "email" as email_validator
compile pattern "url" as url_validator
compile pattern "phone" as phone_validator

// Use compiled patterns for better performance
for each contact in contacts:
    store valid_email as contact["email"] matches compiled email_validator
    store valid_phone as contact["phone"] matches compiled phone_validator
end for
```

### Optimization Guidelines

1. **Use atomic groups** for non-backtracking performance:
   ```wfl
   atomic group of (one or more letters)
   ```

2. **Anchor patterns** when possible:
   ```wfl
   start of line then pattern then end of line
   ```

3. **Use character classes** instead of alternatives:
   ```wfl
   // Better
   any letter or digit
   
   // Slower
   "a" or "b" or "c" or ... or "0" or "1" or "2" ...
   ```

4. **Quantify outer patterns** rather than inner:
   ```wfl
   // Better
   one or more of (letter then digit)
   
   // Slower  
   (one or more letter) then (one or more digit)
   ```

### Memory Management

The pattern engine automatically:
- Caches compiled patterns to avoid recompilation
- Limits backtracking to prevent ReDoS attacks
- Uses efficient NFA/DFA hybrid execution
- Manages memory pools for match results

## Implementation Details

### Architecture Overview

WFL's pattern system uses a bytecode virtual machine:

```
Pattern Source → Lexer → Parser → AST → Compiler → Bytecode → VM → Results
```

### Bytecode Instructions

The pattern compiler generates optimized bytecode:
- `Char(c)` - Match specific character
- `CharClass(set)` - Match character class
- `Split(a, b)` - Non-deterministic branch
- `Jump(addr)` - Unconditional jump
- `Match` - Success state
- `Capture(name)` - Start/end capture group
- `Backref(name)` - Match previous capture

### VM Execution

The pattern VM uses:
- NFA simulation with epsilon transitions
- Backtracking with step limits for safety
- Parallel thread execution for alternatives
- Capture group tracking with efficient storage

### Unicode Support

Full Unicode support includes:
- UTF-8 text processing
- Unicode character classes
- Normalization handling
- Multi-byte character matching

## Migration from Regex

### PCRE Compatibility Mode

For migration, WFL supports direct PCRE patterns:

```wfl
// Use existing regex directly
store regex as pcre pattern "/^[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}$/i"

check if email matches pcre regex:
    display "Valid email (PCRE mode)"
end check
```

### Conversion Examples

| PCRE Regex | WFL Pattern |
|------------|-------------|
| `\d+` | `one or more digit` |
| `[a-zA-Z]+` | `one or more letter` |
| `\w*` | `zero or more of (any letter or digit or "_")` |
| `^hello$` | `start of line then "hello" then end of line` |
| `(?=\d)` | `followed by digit` |
| `(?<=\$)` | `preceded by "$"` |
| `(.+)\1` | `capture {one or more any character} as x same as captured "x"` |

### Migration Strategy

1. **Start with simple patterns** - Convert basic character classes and quantifiers
2. **Use PCRE mode temporarily** - Keep complex patterns in PCRE while converting
3. **Test extensively** - Verify behavior matches expectations
4. **Leverage tools** - Use conversion utilities where available

### Conversion Tool

```wfl
// Convert PCRE to WFL pattern syntax
define action convert_pcre:
    parameter pcre_pattern as Text
    
    store wfl_pattern as convert pcre pcre_pattern to pattern
    return wfl_pattern
end action
```

## Error Handling and Debugging

### Pattern Compilation Errors

```wfl
try:
    create pattern invalid:
        one or more of (
        // Missing closing parenthesis
    end pattern
catch pattern error:
    display "Pattern error: " with pattern error message
end try
```

### Runtime Matching Errors

```wfl
try:
    store result as match text with pattern "complex_pattern"
catch match error:
    display "Match failed: " with match error reason
end try
```

### Pattern Debugging

```wfl
// Debug pattern execution
define action debug_pattern:
    parameter text as Text
    parameter pattern_name as Text
    
    display "Testing pattern: " with pattern_name
    display "Input: " with text
    
    check if text matches pattern pattern_name:
        display "✓ Pattern matched!"
        store captures as get all captures from last match
        for each name and value in captures:
            display "  " with name with ": " with value
        end for
    otherwise:
        display "✗ Pattern failed"
        store debug_info as debug match pattern_name against text
        display "  Failed at: " with debug_info["position"]
        display "  Expected: " with debug_info["expected"]
    end check
end action
```

## Best Practices

### Pattern Design
1. **Start simple** - Build complex patterns from simple components
2. **Use meaningful names** - Name capture groups descriptively
3. **Test incrementally** - Verify each part works before combining
4. **Document patterns** - Explain complex patterns with comments

### Performance
1. **Compile once, use many** - Cache compiled patterns
2. **Anchor when possible** - Use start/end anchors to reduce search space
3. **Avoid catastrophic backtracking** - Test with problematic inputs
4. **Profile pattern performance** - Measure and optimize hot patterns

### Maintainability  
1. **Use standard library patterns** - Leverage pre-built patterns
2. **Break up complex patterns** - Use pattern composition
3. **Add error handling** - Handle pattern compilation and matching errors
4. **Version control patterns** - Track pattern changes like code

This comprehensive guide covers WFL's powerful pattern matching system. The natural language syntax makes patterns more readable and maintainable while providing full regex functionality through an efficient bytecode VM implementation.