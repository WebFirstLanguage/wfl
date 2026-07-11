# Code Style Guide

Consistent style makes WFL programs easier to read, review, and maintain. This guide is the **canonical high-level overview** of WFL formatting style. It matches the defaults in `.wflcfg` and what `wfl --lint` checks for.

For naming detail (actions, containers, booleans, reserved words), see **[Naming Conventions](naming-conventions.md)**. For project layout, see **[Project Organization](project-organization.md)**.

## Quick Reference

| Topic | Canonical style | Default |
|-------|-----------------|---------|
| Indentation | Spaces only | **4 spaces** per level |
| Line length | Soft maximum | **100** characters |
| Nesting depth | Prefer shallow structure | **5** levels max |
| Keywords | Consistent case | **lowercase** (`check if`, `end check`) |
| Trailing whitespace | None | **disallowed** |
| Variables & actions | Preferred form | **snake_case** |
| Containers | Type-like names | **PascalCase** |
| Blocks | Explicit closers | Always use `end …` |
| Blank lines | Separate logical sections | Between config / actions / main |
| Comments | Explain *why* | `//` single-line |

These are the project defaults. Override them in `.wflcfg` only when the whole team agrees.

## Principles

1. **Read like English** — WFL’s natural-language syntax is the point; keep wording clear.
2. **Be consistent** — One style per project beats personal preference.
3. **Prefer shallow structure** — Deep nesting is harder to follow than small actions.
4. **Let tools help** — Lint and fix early; don’t rely on review alone for whitespace and indent.

## Configuration

Project style lives in `.wflcfg` (project directory). Defaults match this guide:

```ini
# Code style settings (WFL defaults)
max_line_length = 100
max_nesting_depth = 5
indent_size = 4
snake_case_variables = true
trailing_whitespace = false
consistent_keyword_case = true
```

| Setting | Meaning |
|---------|---------|
| `max_line_length` | Soft cap for a single line |
| `max_nesting_depth` | Max depth for nested control structures |
| `indent_size` | Spaces per indent level (canonical: 4) |
| `snake_case_variables` | Prefer snake_case for variables and actions |
| `trailing_whitespace` | `false` means trailing spaces are not allowed |
| `consistent_keyword_case` | Prefer consistent (lowercase) keywords |

Full option list: **[Configuration Reference](../reference/configuration-reference.md)**.

```bash
wfl --configCheck
wfl --configFix
```

## Indentation

**Use 4 spaces. Do not use tabs.**

```wfl
store condition as yes
store another_condition as yes

check if condition:
    display "Indented 4 spaces"
    check if another_condition:
        display "Indented 8 spaces"
    end check
end check
```

- Indent one level after a line that opens a block (typically ends with `:`).
- Dedent on `end …` and on `otherwise:`.
- Keep blank lines and pure comment lines without forcing indent noise.

## Line Length

**Prefer lines at or under 100 characters.**

WFL has no line-continuation marker. Split long work with extra statements or shorter strings:

```wfl
// Prefer breaking work across statements
store message_start as "This is a long message that has been "
store message as message_start with "broken across lines for readability"
display message
```

```wfl
// Or split sequential display lines when that still reads clearly
display "This is a long message that has been"
display "broken into multiple lines for readability"
```

## Nesting Depth

**Keep control-structure nesting at or under 5 levels.**

Deep trees of `check if` / loops are hard to follow. Prefer:

1. Combine conditions with `and` / `or` when the logic is still clear.
2. Extract inner logic into an **action**.

```wfl
// Prefer combining related conditions
check if a and b and c and d and e:
    display "All conditions met"
end check
```

```wfl
// Or extract nested decisions into an action
define action called all_conditions_met with parameters a and b and c:
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

**Write keywords in lowercase** and keep that style through a file:

```wfl
store value as 15

// Good
check if value is greater than 10:
    display "Large value"
end check

// Avoid mixed or all-caps keywords
// CHECK IF value IS GREATER THAN 10:
```

Common keywords include: `store`, `change`, `check`, `if`, `otherwise`, `end`, `define`, `action`, `count`, `for`, `each`, `while`, `display`, `yes`, `no`, `nothing`.

## Whitespace

### No trailing whitespace

Do not leave spaces or tabs at the end of a line.

### Blank lines

Use blank lines to separate logical sections — configuration, actions, main flow — not after every single statement:

```wfl
// Configuration
store max_users as 100
store timeout_seconds as 30

// Validation
store user_count as 42
check if user_count is less than max_users:
    display "Room for a new user"
end check

// Processing
store users as ["alice", "bob"]
for each user in users:
    display "Processing " with user
end for
```

## Comments

```wfl
// Section or intent comment
store value as 42  // short inline note when needed
```

**Good (why):**
```wfl
// Using 21 as minimum age for US alcohol laws
store legal_drinking_age as 21
```

**Poor (restates the code):**
```wfl
// Store legal drinking age as 21
store legal_drinking_age as 21
```

More detail: **[Comments and Documentation](../03-language-basics/comments-and-documentation.md)**.

## Naming (summary)

**Project default: snake_case** for variables and actions. The linter (`LINT-NAME`) expects that form.

```wfl
store user_name as "Alice"
store total_count as 0

define action called calculate_total with parameters items:
    return items
end action
```

Spaced names (`store user name as "Alice"`) are valid WFL and fit natural-language experiments, but **pick one style per project**. For shared or linted code, prefer snake_case.

| Kind | Convention | Example |
|------|------------|---------|
| Variables | snake_case | `account_balance` |
| Actions | snake_case verb phrases | `validate_email` |
| Containers | PascalCase singular nouns | `ShoppingCart` |
| Constants (convention) | SCREAMING_SNAKE_CASE | `MAX_RETRY_COUNT` |

Avoid reserved keywords as names (`is`, `file`, `add`, `current`, …). Prefer `is_valid`, `filename`, and similar.

Full guide: **[Naming Conventions](naming-conventions.md)**.  
Keyword lists: **[Keyword Reference](../reference/keyword-reference.md)** · **[Reserved Keywords](../reference/reserved-keywords.md)**.

## Block Structure

### Always close blocks with `end …`

```wfl
store condition as yes
check if condition:
    display "code"
end check

count from 1 to 10:
    display "code"
end count

store items as [1, 2, 3]
for each item in items:
    display "code"
end for
```

### Prefer multi-line blocks

```wfl
store value as 15

check if value is greater than 10:
    display "Large"
otherwise:
    display "Small"
end check
```

Avoid packing complex branches on one line:

```wfl
// Harder to scan — prefer multi-line form above
// check if x is 5: display "Five" otherwise: display "Not five" end check
```

### Nested conditionals

When chaining branches, nest under `otherwise:` (do not write `otherwise check if` as a single flat phrase):

```wfl
store score as 85

check if score is greater than 90:
    display "Excellent"
otherwise:
    check if score is greater than 70:
        display "Good"
    otherwise:
        display "Keep practicing"
    end check
end check
```

## Complete Example

```wfl
// temperature_converter.wfl
// Converts temperatures between Celsius and Fahrenheit

// Configuration
store celsius_to_f_multiplier as 9 divided by 5
store celsius_to_f_offset as 32

// Actions
define action called celsius_to_fahrenheit with parameters celsius:
    store fahrenheit as celsius times celsius_to_f_multiplier plus celsius_to_f_offset
    return fahrenheit
end action

define action called fahrenheit_to_celsius with parameters fahrenheit:
    store celsius as (fahrenheit minus celsius_to_f_offset) divided by celsius_to_f_multiplier
    return celsius
end action

// Main program
display "=== Temperature Converter ==="
display ""

store temp_c as 25
store temp_f as celsius_to_fahrenheit of temp_c
display temp_c with "°C = " with temp_f with "°F"

store temp_f2 as 77
store temp_c2 as fahrenheit_to_celsius of temp_f2
display temp_f2 with "°F = " with temp_c2 with "°C"
```

**What this demonstrates:**
- Short file header
- Grouped sections (config, actions, main)
- 4-space indentation
- Descriptive snake_case names
- Blank lines between sections
- Lowercase keywords
- Comments that explain purpose, not noise

## Enforcement

### Lint rule codes

| Code | What it checks |
|------|----------------|
| `LINT-INDENT` | Indentation (4-space levels) |
| `LINT-LENGTH` | Maximum line length |
| `LINT-COMPLEX` | Maximum nesting depth |
| `LINT-KEYWORD` | Lowercase keywords |
| `LINT-WHITESPACE` | No trailing whitespace |
| `LINT-NAME` | snake_case variables and actions |

### Commands

```bash
# Lint a program
wfl --lint your_program.wfl

# Lint and auto-fix (print fixed source)
wfl --lint --fix your_program.wfl

# Lint and overwrite the file
wfl --lint --fix your_program.wfl --in-place

# Lint and show a diff of proposed fixes
wfl --lint --fix your_program.wfl --diff
```

`--fix` must be used **with** `--lint`.

### Configuration check

```bash
wfl --configCheck
wfl --configFix
```

## Checklist

- [ ] 4-space indentation (no tabs)
- [ ] Lines generally ≤ 100 characters
- [ ] Nesting ≤ 5 levels (or extracted into actions)
- [ ] Lowercase keywords throughout
- [ ] No trailing whitespace
- [ ] snake_case for variables and actions in shared code
- [ ] Blank lines between logical sections
- [ ] Every block closed with the matching `end …`
- [ ] Comments explain *why* when intent is non-obvious
- [ ] `wfl --lint` is clean (or intentional, documented exceptions)

## What You've Learned

- Canonical WFL formatting defaults and `.wflcfg` keys
- Indentation, line length, nesting, keywords, and whitespace
- Naming summary and where full naming rules live
- Block layout, including nested `otherwise:` form
- How to lint and auto-fix with the CLI

**Next:** [Naming Conventions →](naming-conventions.md)

---

**Previous:** [← Best Practices](index.md) | **Next:** [Naming Conventions →](naming-conventions.md)
