# Pattern Matching

WFL's pattern matching system provides regex-like capabilities with natural language syntax. Validate data, extract information, and search text without cryptic regular expressions.

## Why Pattern Matching?

Traditional regex is powerful but cryptic:

```javascript
// JavaScript regex - can you read this?
const emailRegex = /^[a-zA-Z0-9._-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,4}$/;
```

**WFL patterns use natural language:**

```wfl
create pattern email:
    one or more letter or digit or symbol from "._-"
    followed by "@"
    followed by one or more letter or digit or symbol from ".-"
    followed by "."
    followed by 2 to 4 letter
end pattern
```

**Which one is easier to understand?**

## Creating Patterns

Use `create pattern` to define reusable patterns:

```wfl
create pattern greeting:
    "hello"
end pattern
```

**Syntax:**
```wfl
create pattern <name>:
    <pattern expression>
end pattern
```

## Basic Pattern Matching

### Literal Matching

Match exact text:

```wfl
create pattern exact:
    "hello"
end pattern

check if "hello world" matches exact:
    display "Matches!"
end check
```

### Testing Matches

```wfl
store text as "hello world"

check if text matches greeting:
    display "Text contains greeting"
otherwise:
    display "No greeting found"
end check
```

## Character Classes

Match types of characters:

### Digit

```wfl
create pattern phone_simple:
    digit digit digit
end pattern

check if "555" matches phone_simple:
    display "Valid 3-digit code"
end check
```

### Letter

```wfl
create pattern word:
    one or more letter
end pattern

check if "hello" matches word:
    display "Valid word"
end check
```

### Whitespace

```wfl
create pattern spaces:
    one or more whitespace
end pattern

check if "   " matches spaces:
    display "Has whitespace"
end check
```

### Any Character

```wfl
create pattern wildcard:
    "test" then any character then "end"
end pattern

check if "test_end" matches wildcard:
    display "Matches with underscore"
end check
```

## Quantifiers

Control how many times a pattern should match:

### One or More

```wfl
create pattern multiple_digits:
    one or more digit
end pattern

check if "12345" matches multiple_digits:
    display "Multiple digits matched"
end check
```

### Zero or More

```wfl
create pattern optional_digits:
    zero or more digit
end pattern

check if "" matches optional_digits:
    display "Zero digits is valid"
end check

check if "123" matches optional_digits:
    display "Multiple digits also valid"
end check
```

### Optional (Zero or One)

```wfl
create pattern optional_space:
    "hello" then optional " " then "world"
end pattern

check if "hello world" matches optional_space:
    display "With space: matches"
end check

check if "helloworld" matches optional_space:
    display "Without space: also matches"
end check
```

### Exactly N

```wfl
create pattern zip_code:
    exactly 5 digit
end pattern

check if "12345" matches zip_code:
    display "Valid ZIP code"
end check

check if "123" matches zip_code:
    display "Too short"
otherwise:
    display "Not a valid ZIP code"
end check
```

### Between N and M

```wfl
create pattern username:
    3 to 16 letter or digit
end pattern

check if "alice" matches username:
    display "Valid username (5 chars)"
end check
```

### At Least N

```wfl
create pattern strong_password:
    at least 8 any character
end pattern

check if "password123" matches strong_password:
    display "Password long enough"
end check
```

### At Most N

```wfl
create pattern short_code:
    at most 4 digit
end pattern

check if "123" matches short_code:
    display "Valid short code"
end check
```

## Combining Patterns

### Sequences

Use `then` or `followed by` to chain patterns:

```wfl
create pattern phone_number:
    digit digit digit
    followed by "-"
    followed by digit digit digit
    followed by "-"
    followed by digit digit digit digit
end pattern

check if "555-123-4567" matches phone_number:
    display "Valid phone number format"
end check
```

### Alternatives (OR)

Use `or` to match one of several options:

```wfl
create pattern greeting:
    "hello" or "hi" or "hey"
end pattern

check if "hello" matches greeting:
    display "Greeting matched"
end check

check if "hey" matches greeting:
    display "Also matches"
end check
```

### Complex Combinations

```wfl
create pattern email_address:
    one or more letter or digit or symbol from "._-"
    followed by "@"
    followed by one or more letter or digit
    followed by "."
    followed by 2 to 4 letter
end pattern

check if "user@example.com" matches email_address:
    display "Valid email"
end check

check if "invalid-email" matches email_address:
    display "Valid email"
otherwise:
    display "Invalid email format"
end check
```

## Real-World Patterns

### Email Validation

```wfl
create pattern email:
    one or more letter or digit or symbol from "._-"
    followed by "@"
    followed by one or more letter or digit or symbol from ".-"
    followed by "."
    followed by 2 to 4 letter
end pattern

define action called validate email with parameters address:
    check if address matches email:
        return yes
    otherwise:
        return no
    end check
end action

check if validate email with "user@example.com":
    display "Valid email address"
end check
```

### Phone Number Validation

```wfl
create pattern us_phone:
    exactly 3 digit
    followed by "-"
    followed by exactly 3 digit
    followed by "-"
    followed by exactly 4 digit
end pattern

check if "555-123-4567" matches us_phone:
    display "Valid US phone number"
end check
```

### URL Validation

```wfl
create pattern simple_url:
    "http" then optional "s"
    followed by "://"
    followed by one or more any character
end pattern

check if "https://example.com" matches simple_url:
    display "Valid URL"
end check
```

### Date Format (YYYY-MM-DD)

```wfl
create pattern iso_date:
    exactly 4 digit
    followed by "-"
    followed by exactly 2 digit
    followed by "-"
    followed by exactly 2 digit
end pattern

check if "2026-01-09" matches iso_date:
    display "Valid ISO date format"
end check
```

### Credit Card (Last 4 Digits)

```wfl
create pattern card_last_four:
    "****-****-****-"
    followed by exactly 4 digit
end pattern

check if "****-****-****-1234" matches card_last_four:
    display "Valid masked card number"
end check
```

## Pattern Matching in Validation

### Input Validator

```wfl
define action called validate input with parameters data and pattern_name:
    check if data matches pattern_name:
        return yes
    otherwise:
        display "Validation failed for: " with data
        return no
    end check
end action

create pattern alphanumeric:
    one or more letter or digit
end pattern

check if validate input with "abc123" and alphanumeric:
    display "Input is valid"
end check
```

### Form Validation

```wfl
create pattern email:
    one or more letter or digit
    followed by "@"
    followed by one or more letter or digit
    followed by "."
    followed by 2 to 4 letter
end pattern

create pattern zip_code:
    exactly 5 digit
end pattern

store user_email as "test@example.com"
store user_zip as "12345"

store email_valid as no
store zip_valid as no

check if user_email matches email:
    change email_valid to yes
end check

check if user_zip matches zip_code:
    change zip_valid to yes
end check

check if email_valid is yes and zip_valid is yes:
    display "Form is valid - processing..."
otherwise:
    display "Form has errors:"
    check if email_valid is no:
        display "  - Invalid email"
    end check
    check if zip_valid is no:
        display "  - Invalid ZIP code"
    end check
end check
```

## Common Patterns Library

### Username

```wfl
create pattern username:
    3 to 16 letter or digit or symbol from "_-"
end pattern
```

### Password Strength

```wfl
create pattern strong_password:
    at least 8 any character
end pattern
```

### IP Address (Simplified)

```wfl
create pattern ip_address:
    one or more digit then "."
    then one or more digit then "."
    then one or more digit then "."
    then one or more digit
end pattern
```

### Hex Color

```wfl
create pattern hex_color:
    "#" followed by exactly 6 any character
end pattern
```

### Time Format (HH:MM)

```wfl
create pattern time_format:
    exactly 2 digit
    followed by ":"
    followed by exactly 2 digit
end pattern
```

## Best Practices

✅ **Use descriptive pattern names:** `email_address` not `pattern1`

✅ **Test patterns thoroughly:** Validate with multiple test cases

✅ **Keep patterns simple:** Break complex patterns into smaller ones

✅ **Document patterns:** Add comments explaining what they match

✅ **Reuse patterns:** Create once, use multiple times

❌ **Don't make patterns too permissive:** Validate strictly

❌ **Don't forget edge cases:** Test empty strings, special characters

❌ **Don't use patterns for simple checks:** Use `contains` or `starts with` for simple cases

## Pattern vs Simple Checks

### When to Use Patterns

✅ **Complex validation:** Email, phone, URL formats
✅ **Format enforcement:** Dates, times, codes
✅ **Data extraction:** Parsing structured text
✅ **Multiple variations:** Username rules, passwords

### When to Use Simple Functions

✅ **Simple contains:** Use `contains "word" in text`
✅ **Starts/ends with:** Use `starts with` or `ends with`
✅ **Length checks:** Use `length of text`
✅ **Equality:** Use `is equal to`

**Example:**

```wfl
// Don't use pattern for this:
create pattern has_hello:
    "hello"
end pattern

// Just use contains:
check if contains "hello" in text:
    display "Has hello"
end check
```

## Common Mistakes

### Forgetting `end pattern`

**Wrong:**
```wfl
create pattern test:
    "hello"
// Missing end pattern!
```

**Right:**
```wfl
create pattern test:
    "hello"
end pattern
```

### Pattern Too Permissive

**Wrong:**
```wfl
create pattern email:
    one or more any character
end pattern
// This matches EVERYTHING, not just emails!
```

**Right:**
```wfl
create pattern email:
    one or more letter or digit
    followed by "@"
    followed by one or more letter or digit
    followed by "."
    followed by 2 to 4 letter
end pattern
```

## What You've Learned

In this section, you learned:

✅ **Creating patterns** - `create pattern` block
✅ **Testing matches** - `matches` keyword
✅ **Character classes** - digit, letter, whitespace, any
✅ **Quantifiers** - one or more, zero or more, optional, exactly N, between N and M
✅ **Combining patterns** - Using `then` and `followed by`
✅ **Real-world patterns** - Email, phone, URL, date validation
✅ **Best practices** - When to use patterns vs simple checks

## Next Steps

Explore related topics:

**[Text Module →](../05-standard-library/text-module.md)**
Learn text manipulation functions for use with patterns.

**[Pattern Module →](../05-standard-library/pattern-module.md)**
Complete reference for pattern functions.

**[Containers (OOP) →](containers-oop.md)**
Organize validation logic into containers.

---

**Previous:** [← File I/O](file-io.md) | **Next:** [Async Programming →](async-programming.md)
