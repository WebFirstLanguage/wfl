# Code Style Guide

Consistent code style makes WFL programs easier to read, maintain, and collaborate on. This guide defines WFL's coding conventions.

## Configuration

WFL style is configured via `.wflcfg` in your project directory:

```ini
# Code style settings
max_line_length = 100
max_nesting_depth = 5
indent_size = 4
snake_case_variables = true
trailing_whitespace = false
consistent_keyword_case = true
```

## Indentation

**Use 4 spaces** (not tabs):

```wfl
check if condition:
    display "Indented 4 spaces"
    check if another_condition:
        display "Indented 8 spaces"
    end check
end check
```

## Line Length

**Maximum 100 characters per line.**

**Too long:**
```wfl
display "This is a very long message that exceeds 100 characters and should be broken into multiple lines for better readability"
```

**Good:**
```wfl
display "This is a long message that has been"
display "broken into multiple lines for readability"
```

**Or:**
```wfl
store message as "This is a long message that has been " with
    "broken across lines"
display message
```

## Nesting Depth

**Maximum 5 levels of nesting.**

**Too deep:**
```wfl
check if a:
    check if b:
        check if c:
            check if d:
                check if e:
                    check if f:  // 6 levels - too much!
                    end check
                end check
            end check
        end check
    end check
end check
```

**Better:**
```wfl
check if a and b and c and d and e:
    // Use logical operators
end check
```

**Or extract to action:**
```wfl
define action called check conditions with parameters a and b and c:
    check if a:
        check if b:
            check if c:
                return yes
            end check
        end check
    end check
    return no
end action
```

## Keyword Case

**Use lowercase keywords consistently:**

```wfl
// Good
check if value is greater than 10:
    display "Large value"
end check

// Avoid mixing cases
// CHECK IF value IS GREATER THAN 10:
```

## Whitespace

### No Trailing Whitespace

**Bad:**
```wfl
display "Hello"
store x as 5
```

**Good:**
```wfl
display "Hello"
store x as 5
```

### Blank Lines

Use blank lines to separate logical sections:

```wfl
// Configuration
store max_users as 100
store timeout as 30

// Validation
check if user_count is less than max_users:
    allow_new_user()
end check

// Processing
for each user in users:
    process_user(user)
end for
```

## Comments

### Single-Line Comments

```wfl
// This is a comment
store value as 42  // Inline comment
```

### Comment Style

**Good (explain why):**
```wfl
// Using 21 as minimum age for US alcohol laws
store legal_drinking_age as 21
```

**Poor (state the obvious):**
```wfl
// Store legal drinking age as 21
store legal_drinking_age as 21
```

**[More on comments →](../03-language-basics/comments-and-documentation.md)**

## Naming

**[See detailed naming guide →](naming-conventions.md)**

Quick rules:
- snake_case: `user_name`, `total_count`
- Descriptive: `customer_balance` not `cb`
- Avoid reserved keywords

## Block Structure

### Always Use `end` Keywords

```wfl
check if condition:
    code
end check  // Always close blocks

count from 1 to 10:
    code
end count  // Always close

for each item in list:
    code
end for  // Always close
```

### Consistent Block Style

**Good:**
```wfl
check if value is greater than 10:
    display "Large"
otherwise:
    display "Small"
end check
```

**Avoid one-liners for complex logic:**
```wfl
// Harder to read:
check if x is 5: display "Five" otherwise: display "Not five" end check
```

## Complete Example

Well-styled WFL code:

```wfl
// temperature_converter.wfl
// Converts temperatures between Celsius and Fahrenheit

// Configuration
store celsius_to_f_multiplier as 9 divided by 5
store celsius_to_f_offset as 32

// Functions
define action called celsius_to_fahrenheit with parameters celsius:
    store fahrenheit as celsius times celsius_to_f_multiplier plus celsius_to_f_offset
    return fahrenheit
end action

define action called fahrenheit_to_celsius with parameters fahrenheit:
    store celsius as fahrenheit minus celsius_to_f_offset divided by celsius_to_f_multiplier
    return celsius
end action

// Main program
display "=== Temperature Converter ==="
display ""

// Test conversions
store temp_c as 25
store temp_f as celsius_to_fahrenheit with temp_c
display temp_c with "°C = " with temp_f with "°F"

store temp_f2 as 77
store temp_c2 as fahrenheit_to_celsius with temp_f2
display temp_f2 with "°F = " with temp_c2 with "°C"
```

**Features:**
- Clear file header
- Grouped sections (config, functions, main)
- 4-space indentation
- Descriptive names
- Blank lines between sections
- Lowercase keywords
- Comments explain why, not what

## Enforcement

### Automatic Formatting

```bash
# Check style
wfl --lint your_program.wfl

# Auto-fix
wfl --fix your_program.wfl --in-place

# Preview changes
wfl --fix your_program.wfl --diff
```

### Configuration Check

```bash
# Validate .wflcfg
wfl --configCheck

# Auto-fix config
wfl --configFix
```

## Best Practices

✅ **Follow .wflcfg settings** - Consistency across projects
✅ **Use 4-space indentation** - Standard WFL style
✅ **Keep lines under 100 chars** - Easier to read
✅ **Limit nesting to 5 levels** - Extract to actions if deeper
✅ **Use lowercase keywords** - Consistent style
✅ **Remove trailing whitespace** - Clean code
✅ **Add blank lines** - Separate logical sections
✅ **Use `end` keywords** - Always close blocks
✅ **Run linter** - Catch style issues early

## What You've Learned

✅ Configuration via .wflcfg
✅ Indentation (4 spaces)
✅ Line length (100 chars max)
✅ Nesting depth (5 levels max)
✅ Keyword case (lowercase)
✅ Whitespace rules
✅ Comment style
✅ Block structure
✅ Automatic formatting tools

**Next:** [Naming Conventions →](naming-conventions.md)

---

**Previous:** [← Best Practices](index.md) | **Next:** [Naming Conventions →](naming-conventions.md)
