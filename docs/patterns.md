# Pattern Matching in WebFirst Language (WFL)

WFL provides a powerful, natural-language pattern matching system that makes it easy to work with text patterns without the complexity of traditional regular expressions.

## Overview

The WFL pattern matching system uses declarative `create pattern` blocks that compile to an efficient intermediate representation. This approach provides:

- **Natural language syntax** - Use words like "one or more", "optional", "between 1 and 5"
- **Type safety** - Capture groups are typed as `Option<Text>` with flow-sensitive analysis
- **Performance protection** - Built-in guards against catastrophic backtracking
- **Clear error messages** - Helpful diagnostics for both syntax and runtime errors

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

### Literals

Match exact text:

```wfl
create pattern greeting:
    "hello" or "hi" or "hey"
end pattern
```

### Character Classes

Built-in character classes for common patterns:

```wfl
create pattern phone_number:
    digit digit digit
    "-"
    digit digit digit
    "-" 
    digit digit digit digit
end pattern
```

Available character classes:
- `digit` - Matches 0-9
- `letter` - Matches a-z, A-Z
- `whitespace` - Matches space, tab, newline

### Quantifiers

Control how many times elements can repeat:

```wfl
create pattern flexible_number:
    one or more digit
    optional "."
    between 0 and 3 digit
end pattern
```

Quantifier options:
- `optional` - 0 or 1 occurrence
- `one or more` - 1 or more occurrences  
- `zero or more` - 0 or more occurrences
- `between N and M` - Between N and M occurrences
- `exactly N` - Exactly N occurrences

### Alternation

Match one of several alternatives:

```wfl
create pattern file_extension:
    "."
    "txt" or "doc" or "pdf" or "jpg"
end pattern
```

### Captures

Extract parts of the match:

```wfl
create pattern name_pattern:
    capture {
        one or more letter
    } as first_name
    whitespace
    capture {
        one or more letter  
    } as last_name
end pattern

store result as find name_pattern in "John Smith"
if result is not nothing:
    display "First: " with result["first_name"]
    display "Last: " with result["last_name"]
end if
```

### Anchors

Match at specific positions:

```wfl
create pattern line_start:
    at start of text
    "ERROR:"
    capture {
        one or more letter or digit or whitespace
    } as message
end pattern
```

Anchor options:
- `at start of text` - Match at beginning
- `at end of text` - Match at end

## Pattern Operations

### Pattern Matching (`matches`)

Test if text matches a pattern:

```wfl
if user_input matches email_pattern:
    display "Valid email format"
else:
    display "Invalid email format"
end if
```

### Pattern Finding (`find`)

Find the first match and extract captures:

```wfl
store match_result as find phone_pattern in contact_info
if match_result is not nothing:
    display "Phone: " with match_result["match"]
    display "Area code: " with match_result["area_code"]
end if
```

### Pattern Replacement (`replace`)

Replace matches with new text:

```wfl
store sanitized as replace email_pattern with "[EMAIL]" in user_message
display sanitized
```

### Pattern Splitting (`split`)

Split text on pattern matches:

```wfl
create pattern delimiter:
    "," or ";" or "|"
end pattern

store items as split csv_data on pattern delimiter
for each item in items:
    display "Item: " with item
end for each
```

## Type Safety and Flow Analysis

WFL's type system understands pattern captures:

```wfl
store result as find name_pattern in input_text

// result has type Map<Text, Option<Text>>
// Captures are Option<Text> because they might not match

if result is not nothing:
    // Flow-sensitive analysis knows result is not null here
    
    if result["first_name"] is not nothing:
        // Type checker knows first_name is Text here, not Option<Text>
        store greeting as "Hello, " with result["first_name"]
        display greeting
    end if
end if
```

## Error Handling

The pattern system provides clear error messages:

### Syntax Errors

```wfl
create pattern bad_range:
    between 5 and 2 digit  // Error: Invalid range
end pattern
// Error: PATTERN-SYNTAX-INVALID-RANGE
// Check that quantifier ranges are valid (e.g., 'between 1 and 5')
```

### Runtime Errors

```wfl
create pattern complex:
    // Pattern that could cause infinite backtracking
end pattern

// Runtime protection prevents hangs:
// Error: PATTERN-RUNTIME-DEPTH  
// Pattern matching was stopped to prevent infinite loops
```

## Performance Considerations

The WFL pattern engine includes several performance protections:

1. **Step counting** - Limits total matching operations
2. **Recursion depth** - Prevents stack overflow
3. **Backtracking guards** - Detects and prevents catastrophic backtracking

For best performance:
- Use specific patterns rather than overly general ones
- Avoid deeply nested optional groups
- Test complex patterns on representative data

## Migration from Legacy Patterns

**⚠️ DEPRECATION NOTICE**: The legacy regex-based pattern system is deprecated and will be removed in a future version. Please migrate to the new `create pattern` syntax.

### Legacy Syntax (Deprecated)

```wfl
// Old way - deprecated
store email_regex as pattern "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"
if text matches email_regex:
    display "Valid email"
end if
```

### New Syntax (Recommended)

```wfl
// New way - recommended
create pattern email:
    one or more letter or digit or "." or "_" or "%" or "+" or "-"
    "@"
    one or more letter or digit or "." or "-"
    "."
    between 2 and 10 letter
end pattern

if text matches email:
    display "Valid email"
end if
```

### Migration Steps

1. **Identify legacy patterns** - Look for `pattern "regex"` syntax
2. **Convert to natural language** - Replace regex syntax with WFL pattern blocks
3. **Update pattern usage** - Ensure all `matches`, `find`, `replace`, `split` operations use new patterns
4. **Test thoroughly** - Verify behavior matches expectations
5. **Remove legacy patterns** - Clean up old pattern definitions

### Common Conversions

| Legacy Regex | New Pattern Syntax |
|--------------|-------------------|
| `\d+` | `one or more digit` |
| `\w*` | `zero or more letter or digit` |
| `[a-zA-Z]+` | `one or more letter` |
| `\s+` | `one or more whitespace` |
| `(pattern)` | `capture { pattern } as name` |
| `pattern?` | `optional pattern` |
| `pattern{2,5}` | `between 2 and 5 pattern` |
| `pattern1\|pattern2` | `pattern1 or pattern2` |

## Examples

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
        between 2 and 10 letter
    } as tld
end pattern

store result as find email in user_input
if result is not nothing:
    display "Username: " with result["username"]
    display "Domain: " with result["domain"]  
    display "TLD: " with result["tld"]
else:
    display "Invalid email format"
end if
```

### Log Parsing

```wfl
create pattern log_entry:
    at start of text
    capture {
        digit digit digit digit "-" digit digit "-" digit digit
    } as date
    whitespace
    capture {
        digit digit ":" digit digit ":" digit digit
    } as time
    whitespace
    "[" 
    capture {
        one or more letter
    } as level
    "]"
    whitespace
    capture {
        one or more letter or digit or whitespace or "." or ":"
    } as message
end pattern

for each line in log_lines:
    store parsed as find log_entry in line
    if parsed is not nothing:
        display parsed["date"] with " " with parsed["level"] with ": " with parsed["message"]
    end if
end for each
```

### Data Cleaning

```wfl
create pattern phone_number:
    optional "(" 
    capture {
        digit digit digit
    } as area_code
    optional ")"
    optional whitespace or "-"
    capture {
        digit digit digit
    } as exchange
    optional whitespace or "-"
    capture {
        digit digit digit digit
    } as number
end pattern

store cleaned_phones as list

for each contact in contacts:
    store match as find phone_number in contact["phone"]
    if match is not nothing:
        store formatted as match["area_code"] with "-" with match["exchange"] with "-" with match["number"]
        add formatted to cleaned_phones
    end if
end for each
```

## Best Practices

1. **Use descriptive pattern names** - `email_pattern` not `pattern1`
2. **Break complex patterns into parts** - Compose smaller patterns
3. **Test edge cases** - Empty strings, malformed input, boundary conditions
4. **Handle missing captures** - Always check `is not nothing` before using captures
5. **Document pattern intent** - Add comments explaining what patterns match
6. **Validate performance** - Test patterns on realistic data sizes

## Troubleshooting

### Common Issues

**Pattern doesn't match expected input**
- Check for missing `optional` quantifiers
- Verify character classes match your data
- Test with simpler patterns first

**Captures are empty**
- Ensure capture groups actually match content
- Check that quantifiers allow the expected number of matches
- Verify capture names are used consistently

**Performance issues**
- Simplify complex alternations
- Reduce nested optional groups
- Use more specific patterns when possible

**Type errors with captures**
- Always check `is not nothing` before using capture values
- Remember captures are `Option<Text>`, not `Text`
- Use flow-sensitive analysis to narrow types

For additional help, see the WFL documentation or community forums.
