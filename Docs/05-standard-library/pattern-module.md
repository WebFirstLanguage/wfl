# Pattern Module

The Pattern module provides pattern matching utilities for working with patterns created using `create pattern`. These functions complement the pattern matching syntax.

## Pattern Operations

Patterns are created using the `create pattern` syntax (covered in [Pattern Matching](../04-advanced-features/pattern-matching.md)). This module provides functions for using those patterns.

## Functions

### pattern_matches

**Purpose:** Test if text matches a pattern.

**Signature:**
```wfl
<text> matches <pattern>
```

**Alternative:**
```wfl
pattern_matches of <text> and <pattern>
```

**Parameters:**
- `text` (Text): Text to test
- `pattern` (Pattern): Pattern to match against

**Returns:** Boolean - `yes` if matches, `no` otherwise

**Example:**
```wfl
create pattern email:
    one or more letter or digit
    followed by "@"
    followed by one or more letter or digit
    followed by "."
    followed by 2 to 4 letter
end pattern

check if "user@example.com" matches email:
    display "Valid email"
end check

check if "invalid-email" matches email:
    display "Valid email"
otherwise:
    display "Invalid email"
end check
```

**Use Cases:**
- Validation
- Format checking
- Input verification

---

### pattern_find

**Purpose:** Find the first match of a pattern in text.

**Signature:**
```wfl
find <pattern> in <text>
```

**Alternative:**
```wfl
pattern_find of <text> and <pattern>
```

**Parameters:**
- `text` (Text): Text to search
- `pattern` (Pattern): Pattern to find

**Returns:** Match object or nothing if not found

**Match object contains:**
- `matched_text` - The matched string
- `start` - Start position
- `end` - End position
- `captures` - Named capture groups (if any)

**Example:**
```wfl
create pattern phone:
    digit digit digit
    followed by "-"
    followed by digit digit digit
    followed by "-"
    followed by digit digit digit digit
end pattern

store text as "Call me at 555-123-4567 anytime"
store match as find phone in text

check if isnothing of match:
    display "No phone number found"
otherwise:
    display "Found: " with match.matched_text
    display "At position: " with match.start
end check
```

**Use Cases:**
- Extract data from text
- Find specific patterns
- Parse structured text

---

### pattern_find_all

**Purpose:** Find all matches of a pattern in text.

**Signature:**
```wfl
find all <pattern> in <text>
```

**Alternative:**
```wfl
pattern_find_all of <text> and <pattern>
```

**Parameters:**
- `text` (Text): Text to search
- `pattern` (Pattern): Pattern to find

**Returns:** List - List of match objects

**Example:**
```wfl
create pattern word:
    one or more letter
end pattern

store text as "The quick brown fox"
store matches as find all word in text

display "Found " with length of matches with " words:"
for each match in matches:
    display "  - " with match.matched_text
end for
```

**Use Cases:**
- Extract all occurrences
- Count patterns
- Batch extraction

---

## Pattern in Conditions

The most common usage is in conditionals:

```wfl
create pattern valid_username:
    3 to 16 letter or digit
end pattern

store username as "alice123"

check if username matches valid_username:
    display "Username is valid"
otherwise:
    display "Username must be 3-16 alphanumeric characters"
end check
```

## Complete Example

```wfl
display "=== Pattern Module Demo ==="
display ""

// Create patterns
create pattern email:
    one or more letter or digit
    followed by "@"
    followed by one or more letter or digit
    followed by "."
    followed by 2 to 4 letter
end pattern

create pattern phone:
    exactly 3 digit
    followed by "-"
    followed by exactly 3 digit
    followed by "-"
    followed by exactly 4 digit
end pattern

create pattern word:
    one or more letter
end pattern

// Test matching
store email_addr as "user@example.com"
check if email_addr matches email:
    display "✓ Valid email: " with email_addr
end check

store phone_num as "555-123-4567"
check if phone_num matches phone:
    display "✓ Valid phone: " with phone_num
end check
display ""

// Find pattern in text
store contact_info as "Email: user@example.com, Phone: 555-123-4567"

store email_match as find email in contact_info
check if isnothing of email_match:
    display "No email found"
otherwise:
    display "Found email: " with email_match.matched_text
end check

store phone_match as find phone in contact_info
check if isnothing of phone_match:
    display "No phone found"
otherwise:
    display "Found phone: " with phone_match.matched_text
end check
display ""

// Find all matches
store sentence as "The quick brown fox jumps over the lazy dog"
store word_matches as find all word in sentence

display "Words found: " with length of word_matches
for each word_match in word_matches:
    display "  - " with word_match.matched_text
end for
display ""

display "=== Demo Complete ==="
```

## Common Patterns

### Extract Email from Text

```wfl
create pattern email:
    one or more letter or digit or symbol from "._-"
    followed by "@"
    followed by one or more letter or digit
    followed by "."
    followed by 2 to 4 letter
end pattern

define action called extract email with parameters text:
    store match as find email in text
    check if isnothing of match:
        return nothing
    otherwise:
        return match.matched_text
    end check
end action

store contact as "Contact Alice at alice@example.com for details"
store email_addr as extract email with contact
display "Email: " with email_addr
// Output: Email: alice@example.com
```

### Validate Multiple Fields

```wfl
create pattern email:
    one or more letter or digit
    followed by "@"
    followed by one or more letter or digit
    followed by "."
    followed by 2 to 4 letter
end pattern

create pattern phone:
    exactly 3 digit then "-" then exactly 3 digit then "-" then exactly 4 digit
end pattern

define action called validate contact with parameters email_val and phone_val:
    store email_ok as email_val matches email
    store phone_ok as phone_val matches phone

    check if email_ok is yes and phone_ok is yes:
        return yes
    otherwise:
        check if email_ok is no:
            display "Invalid email format"
        end check
        check if phone_ok is no:
            display "Invalid phone format"
        end check
        return no
    end check
end action

check if validate contact with "user@example.com" and "555-123-4567":
    display "Contact information is valid"
end check
```

## Best Practices

✅ **Define patterns once, use many times:** Create reusable patterns

✅ **Use matches for validation:** Simple yes/no checks

✅ **Use find for extraction:** Get the matched text

✅ **Use find all for multiple matches:** Extract all occurrences

✅ **Check for nothing:** find returns nothing if no match

❌ **Don't recreate patterns:** Define once, reference multiple times

❌ **Don't use patterns for simple checks:** Use `contains` instead

❌ **Don't forget isnothing check:** find can return nothing

## What You've Learned

In this module, you learned:

✅ **pattern_matches** - Test if text matches pattern
✅ **pattern_find** - Find first match
✅ **pattern_find_all** - Find all matches
✅ **Match objects** - matched_text, start, end, captures
✅ **Common patterns** - Email extraction, validation
✅ **Best practices** - Reuse patterns, check for nothing

## Next Steps

**[Pattern Matching Guide →](../04-advanced-features/pattern-matching.md)**
Complete guide to creating and using patterns.

**[Text Module →](text-module.md)**
String functions that work well with patterns.

Or return to:
**[Standard Library Index →](index.md)**

---

**Previous:** [← Crypto Module](crypto-module.md) | **Next:** [Typechecker Module →](typechecker-module.md)
