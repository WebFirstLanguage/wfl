# Pattern Matching in WebFirst Language (WFL)

WFL provides a powerful, natural-language pattern matching system that makes it easy to work with text patterns without the complexity of traditional regular expressions.

> **📋 Consolidated Documentation**
> This is the **authoritative reference** for WFL pattern matching. Content from the following guides has been consolidated here:
> - Pattern Migration Guide (from regex) - See [Migration from Regex](#migration-from-regex) section
> - Pattern Practical Examples - See [Common Patterns](#common-patterns) section
> - Pattern Error Guide - See error descriptions throughout and [Best Practices](#best-practices)
>
> Previous standalone guides archived to `archive/superseded/`

> **⚠️ Implementation Status Notice**
> This documentation covers both implemented and planned features. Features marked with ❌ are not yet implemented. See [Implementation Status](#implementation-status) for details.

## Table of Contents
- [Overview](#overview)
- [Implementation Status](#implementation-status)
- [Basic Syntax](#basic-syntax)
- [Pattern Elements](#pattern-elements)
- [Built-in Functions](#built-in-functions)
- [Common Patterns](#common-patterns)
- [Advanced Features](#advanced-features)
- [Unicode Support](#unicode-support) ❌
- [Performance & Optimization](#performance--optimization)
- [Migration from Regex](#migration-from-regex)
- [Design Philosophy](#design-philosophy)
- [Best Practices](#best-practices)

## Overview

The WFL pattern matching system uses declarative `create pattern` blocks that compile to an efficient bytecode virtual machine. This approach provides:

- **Natural language syntax** - Use words like "one or more", "optional", "2 to 5"
- **Type safety** - Capture groups are typed as `Option<Text>` with flow-sensitive analysis
- **Performance protection** - Built-in guards against catastrophic backtracking
- **Clear error messages** - Helpful diagnostics for both syntax and runtime errors
- **Bytecode compilation** - Optimized execution through VM

## Implementation Status

### ✅ Fully Implemented Features
- Basic literals and character classes (`digit`, `letter`, `whitespace`)
- Quantifiers (`one or more`, `zero or more`, `optional`, `exactly N`, `N to M`, `at least N`, `at most N`)
- Sequences and alternatives (`"a" then "b"`, `"a" or "b"`)
- Capture groups (`capture { pattern } as name`)
- Pattern operations (`matches`, `find`, `find_all`, `replace`, `split`)

### ❌ Not Yet Implemented
- Backreferences (`same as group N`)
- Character sets (`any of "!@#$%"`)
- Unicode categories and scripts
- Simple lookahead/lookbehind syntax (requires braces currently)

### ⚠️ Partially Implemented
- Lookahead/lookbehind (works with braces: `followed by { pattern }`)

For a complete analysis, see `Docs/dev-notes/pattern-implementation-analysis.md`.

## Basic Syntax

### Creating Patterns

```wfl
create pattern email_pattern:
    one or more letter or digit or "." or "_"
    "@"
    one or more letter or digit or "."
end pattern
```

### Using Patterns

```wfl
// Check if text matches a pattern
if "user@example.com" matches email_pattern:
    display "Valid email!"
end if

// Find matches and extract captures
store result as find email_pattern in "Contact: user@example.com"
if result is not nothing:
    display "Found email: " with result["match"]
end if

// Replace matches
store cleaned as replace email_pattern with "[EMAIL]" in "Send to user@example.com"

// Split text on pattern
store parts as split "one,two,three" on pattern comma_pattern
```

## Pattern Elements

### Character Classes

Character classes define what types of characters to match at a specific position:

| Pattern | What it matches | Example matches |
|---------|----------------|-----------------|
| `any letter` | Any uppercase or lowercase letter (A-Z, a-z) | "a", "B", "z", "Q" |
| `any digit` | Any numeric digit (0-9) | "0", "5", "9" |
| `any whitespace` | Spaces, tabs, newlines, carriage returns | " ", "\t", "\n" |
| `any punctuation` | Common punctuation marks | ".", "!", "?", ",", ";" |
| `any character` | Literally any single character | "a", "7", "@", " ", "€" |
| `letter` | Shorthand for `any letter` | "a", "Z" |
| `digit` | Shorthand for `any digit` | "0", "9" |
| `whitespace` | Shorthand for `any whitespace` | " ", "\t" |

**Combined Classes:** ✅ IMPLEMENTED
```wfl
any letter or digit          // Alphanumeric - works with "or"
any letter or digit or "_"   // Variable names - works with literals
```

**Character Sets:** ❌ NOT YET IMPLEMENTED
```wfl
// These don't work yet:
// any character not in "xyz"   // Exclusion
// any character from "a" to "z" // Range
// any of "!@#$%"              // Character set
```

### Quantifiers

Control how many times elements can repeat:

| Pattern | What it means | Example |
|---------|---------------|---------|
| `zero or more` | Match 0 or unlimited times | `zero or more letters` |
| `one or more` | Match at least once | `one or more digits` |
| `optional` | Match 0 or 1 time | `optional whitespace` |
| `exactly N` | Match exactly N times | `exactly 3 digits` |
| `N to M` | Match between N and M times | `2 to 4 letters` |
| `at least N` | Match N or more times | `at least 5 characters` |
| `at most N` | Match up to N times | `at most 10 digits` |

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

Match at specific positions in text:

```wfl
start of text        // ✅ Beginning of entire text
// end of text       // ❌ Not yet implemented
// at start of line  // ❌ Not yet implemented
// at end of line    // ❌ Not yet implemented
// word boundary     // ❌ Not yet implemented
```

**⚠️ Implementation Status:**
- Only `start of text` is currently implemented
- Other anchors are planned for future releases

### Captures and Backreferences

Extract parts of matches and reference them later:

```wfl
// Named capture groups ✅ IMPLEMENTED
create pattern name_pattern:
    capture {one or more letter} as first_name
    whitespace
    capture {one or more letter} as last_name
end pattern

// Backreferences - match same content ❌ NOT YET IMPLEMENTED
// create pattern duplicate_word:
//     capture {one or more letter} as word
//     whitespace
//     same as captured word
// end pattern
```

### Lookarounds ⚠️ PARTIALLY IMPLEMENTED

Zero-width assertions that check ahead or behind. **Note:** Current implementation requires braces around the lookaround pattern.

```wfl
// Positive lookahead - must be followed by ✅ WORKS WITH BRACES
create pattern digit_before_letter:
    digit followed by {letter}
end pattern

// Negative lookahead - must NOT be followed by ✅ WORKS WITH BRACES
create pattern digit_not_before_letter:
    digit not followed by {letter}
end pattern

// Positive lookbehind - must be preceded by ✅ WORKS WITH BRACES
create pattern digit_after_dollar:
    preceded by {"$"} then digit
end pattern

// Negative lookbehind - must NOT be preceded by ✅ WORKS WITH BRACES
create pattern digit_not_after_dollar:
    not preceded by {"$"} then digit
end pattern
```

❌ **Simple syntax not yet implemented:**
```wfl
// These don't work yet:
// digit followed by letter
// digit preceded by "$"
```

## Built-in Functions

### Pattern Operations

| Function | Description | Example |
|----------|-------------|---------|
| `matches` | Check if text matches pattern | `if text matches email_pattern:` |
| `find` | Find first match with captures | `store result as find pattern in text` |
| `find_all` | Find all matches | `store all as find_all pattern in text` |
| `replace` | Replace matches | `store new as replace pattern with "X" in text` |
| `split` | Split by pattern | `store parts as split text on pattern delimiter` |

#### find_all Function

The `find_all` function finds all non-overlapping matches of a pattern in text and returns a list containing all matches with their capture groups.

**Syntax:**
```wfl
store results as find_all pattern_name in text_value
```

**Return Type:** List of match objects, where each match contains:
- `match`: The full text that matched the pattern
- `captures`: Map of named capture groups to their values
- `position`: Starting position of the match in the original text

**Example:**
```wfl
create pattern phone_number:
    capture {exactly 3 digit} as area
    "-"
    capture {exactly 3 digit} as exchange
    "-"
    capture {exactly 4 digit} as number
end pattern

store text as "Call 555-123-4567 or 555-987-6543 for help"
store all_phones as find_all phone_number in text

count from each phone in all_phones:
    display "Found phone: " with phone.match
    display "Area code: " with phone.captures.area
    display "Exchange: " with phone.captures.exchange
    display "Number: " with phone.captures.number
end count

// Output:
// Found phone: 555-123-4567
// Area code: 555
// Exchange: 123
// Number: 4567
// Found phone: 555-987-6543
// Area code: 555
// Exchange: 987
// Number: 6543
```

**Behavior:**
- Returns an empty list if no matches are found
- Matches do not overlap - after finding a match, search continues after the end of that match
- Capture groups that don't participate in a match contain `nothing`
- Matches are returned in the order they appear in the text

### Standard Library Patterns

WFL includes pre-built patterns for common use cases:

```wfl
// Available in stdlib
email_pattern      // Email validation
url_pattern        // URL parsing
phone_pattern      // Phone numbers
ipv4_pattern       // IPv4 addresses
ipv6_pattern       // IPv6 addresses
date_pattern       // Date formats
time_pattern       // Time formats
uuid_pattern       // UUID format
```

## Common Patterns

### Email Validation

```wfl
create pattern email:
    capture {
        one or more letter or digit or "." or "_" or "%" or "+" or "-"
    } as username
    "@"
    capture {
        one or more letter or digit or "." or "-"
    } as domain
    "."
    capture {
        2 to 10 letter
    } as tld
end pattern
```

### Phone Numbers

```wfl
// US phone with flexible formatting
create pattern us_phone:
    optional "+" or "1" then optional " "
    optional "("
    capture {exactly 3 digit} as area_code
    optional ")"
    optional " " or "-"
    capture {exactly 3 digit} as exchange
    optional " " or "-"
    capture {exactly 4 digit} as line
end pattern
```

### URL Parsing

```wfl
create pattern url:
    // Protocol
    capture {optional ("http" or "https" or "ftp") then "://"} as "protocol"
    
    // Domain
    capture {
        one or more letter or digit or "-"
        zero or more ("." then one or more letter or digit or "-")
    } as "domain"
    
    // Port
    optional (":" then capture {one or more digit} as "port")
    
    // Path
    capture {optional ("/" then zero or more any character not in "?#")} as "path"
    
    // Query
    optional ("?" then capture {zero or more any character not in "#"} as "query")
    
    // Fragment
    optional ("#" then capture {zero or more any character} as "fragment")
end pattern
```

### Date Patterns

```wfl
// ISO 8601 date
create pattern iso_date:
    capture {exactly 4 digit} as "year"
    "-"
    capture {exactly 2 digit} as "month"
    "-"
    capture {exactly 2 digit} as "day"
    optional (
        "T"
        capture {exactly 2 digit} as "hour"
        ":"
        capture {exactly 2 digit} as "minute"
        optional (":" then capture {exactly 2 digit} as "second")
        optional ("Z" or ("+" or "-" then exactly 2 digit then ":" then exactly 2 digit))
    )
end pattern
```

### Log Parsing

```wfl
create pattern log_entry:
    at start of line
    capture {exactly 4 digit "-" exactly 2 digit "-" exactly 2 digit} as "date"
    whitespace
    capture {exactly 2 digit ":" exactly 2 digit ":" exactly 2 digit} as "time"
    whitespace
    "[" capture {one or more letter} as "level" "]"
    whitespace
    capture {one or more any character} as "message"
end pattern
```

## Advanced Features

### Capture Groups and Extraction

```wfl
create pattern name_pattern:
    capture {one or more letter} as "first"
    one or more whitespace
    capture {one or more letter} as "last"
end pattern

// Extract captured values
store result as find name_pattern in "John Doe"
if result is not nothing:
    display "First: " with result["first"]
    display "Last: " with result["last"]
end if
```

### Backreferences for Repeated Content

```wfl
// Match HTML/XML tags
create pattern html_tag:
    "<"
    capture {one or more letter} as "tag"
    ">"
    zero or more any character
    "</"
    same as captured "tag"
    ">"
end pattern

// Validate balanced quotes
create pattern quoted_string:
    capture {any of "\"" or "'"} as "quote"
    zero or more any character not in captured "quote"
    same as captured "quote"
end pattern
```

### Lookaround Assertions

```wfl
// Password validation with lookarounds
create pattern strong_password:
    // Must contain lowercase (positive lookahead)
    followed by (zero or more any character then any lowercase)
    
    // Must contain uppercase (positive lookahead)
    followed by (zero or more any character then any uppercase)
    
    // Must contain digit (positive lookahead)
    followed by (zero or more any character then any digit)
    
    // Must contain special char (positive lookahead)
    followed by (zero or more any character then any of "!@#$%^&*")
    
    // At least 8 characters
    at least 8 any character
end pattern
```

### Pattern Composition

```wfl
// Build complex patterns from simpler ones
create pattern word:
    one or more letter
end pattern

create pattern sentence:
    word
    zero or more (whitespace then word)
    any of ".!?"
end pattern

// Reuse patterns
create pattern paragraph:
    sentence
    zero or more (whitespace then sentence)
end pattern
```

## Unicode Support ❌ NOT YET IMPLEMENTED

> **⚠️ Note:** Unicode support is planned but not yet implemented. The examples below show the intended syntax for future implementation.

WFL will provide comprehensive Unicode support for international text processing.

### Unicode Categories

```wfl
// Match any Unicode letter
create pattern unicode_letters:
    unicode category "Letter"
end pattern

// Match specific categories
create pattern uppercase:
    unicode category "Uppercase_Letter"
end pattern

create pattern currency:
    unicode category "Currency_Symbol"  // $, €, ¥, £, etc.
end pattern

create pattern emoji:
    unicode category "Other_Symbol"
end pattern
```

### Supported Unicode Categories

**Letters:**
- `Letter` (L) - All letters
- `Uppercase_Letter` (Lu) - Uppercase letters
- `Lowercase_Letter` (Ll) - Lowercase letters
- `Titlecase_Letter` (Lt) - Titlecase letters
- `Modifier_Letter` (Lm) - Modifier letters
- `Other_Letter` (Lo) - Other letters (Chinese, Japanese, etc.)

**Numbers:**
- `Number` (N) - All numbers
- `Decimal_Number` (Nd) - Decimal digits (0-9, ٠-٩, etc.)
- `Letter_Number` (Nl) - Letter-like numbers (Ⅰ, Ⅱ, etc.)
- `Other_Number` (No) - Other numbers (½, ¼, etc.)

**Symbols:**
- `Symbol` (S) - All symbols
- `Math_Symbol` (Sm) - Math symbols (+, =, etc.)
- `Currency_Symbol` (Sc) - Currency symbols ($, €, ¥, etc.)
- `Modifier_Symbol` (Sk) - Modifier symbols
- `Other_Symbol` (So) - Other symbols (including emoji)

**Punctuation:**
- `Punctuation` (P) - All punctuation
- `Connector_Punctuation` (Pc) - Connectors (_, ‿, etc.)
- `Dash_Punctuation` (Pd) - Dashes (-, –, —, etc.)
- `Open_Punctuation` (Ps) - Opening punctuation ((, [, {, etc.)
- `Close_Punctuation` (Pe) - Closing punctuation (), ], }, etc.)

### Unicode Scripts

```wfl
// Match specific writing systems
create pattern chinese_text:
    unicode script "Han"
end pattern

create pattern arabic_text:
    unicode script "Arabic"
end pattern

create pattern mixed_japanese:
    unicode script "Hiragana" or "Katakana" or "Han"
end pattern
```

### International Examples

```wfl
// Japanese email pattern
create pattern japanese_email:
    one or more (unicode script "Hiragana" or "Katakana" or "Han" or letter or digit)
    "@"
    one or more (letter or digit or "." or "-")
    "."
    between 2 and 10 letter
end pattern

// Arabic phone number
create pattern arabic_phone:
    optional "+"
    // Allow Arabic-Indic digits (٠١٢٣٤٥٦٧٨٩) or ASCII digits
    exactly 3 (any of "٠١٢٣٤٥٦٧٨٩" or digit)
    "-"
    exactly 7 (any of "٠١٢٣٤٥٦٧٨٩" or digit)
end pattern
```

## Performance & Optimization

### Pattern Compilation and Caching

The WFL pattern engine automatically caches compiled patterns:

```wfl
// Patterns are compiled once and cached
for each email in email_list:
    if email matches email_pattern:  // Uses cached bytecode
        add email to valid_emails
    end if
end for each
```

### Optimization Guidelines

1. **Use specific patterns** rather than overly general ones
2. **Anchor patterns** when possible to reduce search space
3. **Use character classes** instead of long alternations
4. **Avoid nested quantifiers** that can cause backtracking
5. **Test with problematic inputs** to ensure performance

### Performance Protections

The WFL pattern engine includes:
- **Step counting** - Limits total matching operations
- **Recursion depth limits** - Prevents stack overflow
- **Backtracking guards** - Detects and prevents catastrophic backtracking
- **Memory pools** - Efficient memory management for captures

## Migration from Regex

### Conversion Guide

| PCRE Regex | WFL Pattern | Status |
|------------|-------------|--------|
| `\d+` | `one or more digit` | ✅ |
| `\w*` | `zero or more letter or digit or "_"` | ✅ |
| `[a-zA-Z]+` | `one or more letter` | ✅ |
| `\s+` | `one or more whitespace` | ✅ |
| `(pattern)` | `capture {pattern} as name` | ✅ |
| `pattern?` | `optional pattern` | ✅ |
| `pattern{2,5}` | `2 to 5 pattern` | ✅ |
| `pattern1\|pattern2` | `pattern1 or pattern2` | ✅ |
| `^pattern$` | `start of text then pattern` (end anchor not implemented) | ⚠️ |
| `(?=\d)` | `followed by {digit}` | ⚠️ (requires braces) |
| `(?<=\$)` | `preceded by {"$"}` | ⚠️ (requires braces) |
| `(.+)\1` | ❌ Not yet implemented | ❌ |

### Migration Strategy

1. **Start simple** - Convert basic character classes first
2. **Test incrementally** - Verify each pattern works correctly
3. **Use composition** - Build complex patterns from simple ones
4. **Leverage tools** - Use conversion utilities where available

## Design Philosophy

WFL's pattern system represents a fundamental reimagining of text pattern matching. Traditional regular expressions, while powerful, suffer from a terse, symbol-heavy syntax that makes them notoriously difficult to read and maintain.

### Why Natural Language Patterns?

1. **Readability Over Brevity**: While regex prioritizes compact notation, WFL patterns prioritize clarity. Compare `^\d{3}-[A-Za-z]{2}$` with "at start of text then exactly 3 digit then '-' then exactly 2 letter then at end of text" - the latter is instantly understandable.

2. **Self-Documenting Code**: WFL patterns serve as their own documentation. The pattern itself explains its purpose in plain English.

3. **Lower Barrier to Entry**: By using familiar words instead of cryptic symbols, WFL makes pattern matching accessible to beginners and non-programmers.

4. **Fewer Errors**: Natural language patterns eliminate common regex pitfalls like escaping issues, greedy vs. lazy quantifiers, and backreference confusion.

### Historical Context

WFL's pattern system draws inspiration from:
- **SNOBOL and Icon**: Languages that treated patterns as first-class objects
- **Raku (Perl 6)**: Introduced rules and grammars with more readable syntax
- **Parser Combinators**: Functional programming's composable parsers
- **Cucumber Expressions**: BDD tools using placeholders like `{int}` and `{string}`

### Implementation Architecture

WFL patterns compile to an efficient bytecode VM:

```
Pattern Source → Lexer → Parser → AST → Compiler → Bytecode → VM → Results
```

The VM uses:
- NFA simulation with epsilon transitions
- Backtracking with safety limits
- Parallel thread execution for alternatives
- Efficient capture group tracking

## Best Practices

### Pattern Design
1. **Use descriptive names** - `email_pattern` not `pattern1`
2. **Break complex patterns into parts** - Compose smaller patterns
3. **Test edge cases** - Empty strings, malformed input, boundaries
4. **Document intent** - Add comments explaining what patterns match

### Error Handling
```wfl
store result as find pattern in text
if result is not nothing:
    // Always check captures exist before using
    if result["capture_name"] is not nothing:
        display result["capture_name"]
    end if
end if
```

### Performance
1. **Compile once, use many** - Let the engine cache patterns
2. **Profile hot patterns** - Measure and optimize frequently used patterns
3. **Use standard library** - Leverage pre-built, optimized patterns
4. **Avoid catastrophic backtracking** - Test with adversarial inputs

### Maintainability
1. **Version control patterns** - Track changes like code
2. **Use pattern composition** - Build from reusable components
3. **Add unit tests** - Test patterns with known inputs/outputs
4. **Keep patterns simple** - Favor clarity over cleverness

## Troubleshooting

### Common Syntax Errors

**"Unexpected token in pattern" errors:**
```wfl
// ❌ Wrong: between N and M syntax
create pattern wrong:
    between 2 and 4 letter
end pattern

// ✅ Correct: N to M syntax
create pattern correct:
    2 to 4 letter
end pattern
```

**"Expected '{' after 'capture'" errors:**
```wfl
// ❌ Wrong: capture without braces
create pattern wrong:
    capture one or more letter as name
end pattern

// ✅ Correct: capture with braces
create pattern correct:
    capture {one or more letter} as name
end pattern
```

**Lookahead/lookbehind syntax:**
```wfl
// ❌ Wrong: simple syntax not implemented
// create pattern wrong:
//     digit followed by letter
// end pattern

// ✅ Correct: use braces
create pattern correct:
    digit followed by {letter}
end pattern
```

### Common Runtime Issues

**Pattern doesn't match expected input:**
- Check for missing `optional` quantifiers
- Verify character classes match your data
- Test with simpler patterns first

**Captures are empty:**
- Ensure capture groups actually match content
- Check quantifiers allow expected matches
- Verify capture names are used consistently

**Performance issues:**
- Simplify complex alternations
- Reduce nested optional groups
- Use more specific patterns

**Type errors with captures:**
- Always check `is not nothing` before using
- Remember captures are `Option<Text>`
- Use flow-sensitive analysis to narrow types

### Error Messages

WFL provides clear, actionable error messages:

```wfl
// Syntax error
create pattern bad_range:
    between 5 and 2 digit  // Error: Invalid range
end pattern
// Error: PATTERN-SYNTAX-INVALID-RANGE
// Quantifier range must have min <= max

// Runtime protection
// Error: PATTERN-RUNTIME-DEPTH
// Pattern matching stopped to prevent infinite loops
```

## See Also

- [WFL Language Specification](wfl-spec.md)
- [Pattern Module API](../api/pattern-module.md) - Legacy pattern API
- [Standard Library Reference](../api/wfl-standard-library.md)
- [WFL Async Programming](WFL-async.md) - Async pattern operations

## Archived Documentation

The following standalone pattern guides have been consolidated into this document:
- `pattern-migration-guide.md` → Migration content integrated throughout
- `pattern-practical-examples.md` → Examples integrated into Common Patterns section
- `pattern-error-guide.md` → Error information integrated into relevant sections

Archived files available in: `Docs/archive/superseded/`