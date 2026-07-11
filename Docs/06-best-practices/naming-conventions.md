# Naming Conventions

Good names make WFL code self-documenting. This guide is the **canonical naming reference** for WFL programs. It pairs with the **[Code Style Guide](code-style-guide.md)**, which covers formatting (indent, line length, keywords, whitespace).

## Quick Reference

| Kind | Convention | Example |
|------|------------|---------|
| Variables | **snake_case** (project default) | `user_name`, `account_balance` |
| Actions | snake_case **verb phrases** | `calculate_total`, `validate_email` |
| Parameters | snake_case | `purchase_amount`, `new_email` |
| Containers | **PascalCase**, singular nouns | `Person`, `ShoppingCart` |
| Container methods | snake_case verb phrases | `set_email`, `get_info` |
| Constants (convention) | **SCREAMING_SNAKE_CASE** | `MAX_USERS`, `API_VERSION` |
| Booleans | affirmative prefixes | `is_active`, `has_permission` |
| Lists / collections | **plural** nouns | `users`, `error_messages` |
| Patterns | what they match | `email_address`, `iso_date_format` |
| Files / paths | lowercase + underscores | `app_config.txt`, `monthly_summary.pdf` |

**Linter:** `LINT-NAME` expects snake_case for variables and actions.  
**Config:** `snake_case_variables = true` is the default in `.wflcfg`.

## Principles

1. **Descriptive over short** — Prefer `customer_email` to `email` or `e` when context is ambiguous.
2. **Read like English** — Names should fit natural WFL phrasing.
3. **One style per project** — Default is snake_case; if you use spaced names, use them consistently.
4. **Avoid reserved keywords** — Prefer `is_valid`, `filename`, etc.
5. **Signal role in the name** — Verbs for actions, plurals for collections, `is_`/`has_` for booleans.

## Variable Names

### Preferred: snake_case (project default)

Use this form for shared, documented, and linted code:

```wfl
store user_name as "Alice"
store account_balance as 1000.00
store is_verified as yes
store total_count as 0
store retry_count as 3
```

### Alternative: spaced names

Spaced names are valid WFL and can feel natural in tutorials or personal scripts:

```wfl
store user name as "Alice"
store account balance as 1000.00
store account verified as yes
store item total as 0
```

Rules when using spaces:

- **Stay consistent** within the project.
- Avoid embedding reserved words (`is`, `count`, `file`, …) inside the name.
- Prefer snake_case for team and production code unless `.wflcfg` sets `snake_case_variables = false`.

### Descriptive over cryptic

**Good:**
```wfl
store customer_age as 25
store order_total as 199.99
store retry_count as 3
```

**Poor:**
```wfl
store ca as 25
store ot as 199.99
store rc as 3
```

Single letters are fine for pure math (`x`, `y`, `i`). Prefer real words everywhere else.

### What makes a valid name

Names **may**:

- Contain letters: `name`, `user_data`
- Contain digits (not first): `value1`, `item_2`
- Contain underscores: `user_name`, `total_count`
- Contain spaces (spaced style): `user name`, `total count`

Names **may not**:

- Start with a digit: `1value`
- Use most special characters: `user@email`
- Be a reserved keyword alone: `store`, `check`, `if`, `is`, `file`

## Reserved Keywords

WFL has **181** keywords and literals. Many cannot be used as bare variable names.

**Quick Reference:** [Keyword Reference](../reference/keyword-reference.md)  
**Complete details:** [Reserved Keywords](../reference/reserved-keywords.md)

**Common conflicts and safer names:**

| Avoid | Prefer |
|-------|--------|
| `is` | `is_valid`, `is_value` |
| `file` | `filename`, `file_handle`, `file_path` |
| `add` | `addition`, `add_result` |
| `current` | `current_value`, `current_item` |
| `list` (in some contexts) | `item_list`, `entries` |
| `count` (inside count loops) | loop variable is special; use `total_count` for your own counter outside |

```wfl
// Wrong — structural keywords as names fail to parse
// store is as yes
// store file as "data.txt"

// Right
store is_valid as yes
store filename as "data.txt"
```

## Action Names

### Verb phrases in snake_case

```wfl
define action called calculate_total with parameters items:
    return items
end action

define action called validate_email with parameters address:
    return yes
end action

define action called send_notification with parameters user:
    display "notifying " with user
end action

define action called format_order_date with parameters date_value:
    return date_value
end action
```

### Be specific

**Good:** `calculate_discount_for_member`, `send_welcome_email`, `validate_credit_card`  
**Poor:** `calc`, `proc`, `do_stuff`, `handle`

Actions name **behavior**. Prefer verbs (`calculate`, `validate`, `send`, `load`, `format`) over vague nouns.

### Parameters

Name parameters the same way as variables — snake_case and specific:

```wfl
define action called apply_discount with parameters purchase_amount and member_rate:
    return purchase_amount times member_rate
end action
```

Avoid single-letter parameters except in tiny math helpers.

## Container Names

### PascalCase, singular nouns

```wfl
create container Person:
end

create container ShoppingCart:
end

create container EmailValidator:
end

create container DatabaseConnection:
end
```

Prefer **singular** type names: `User`, `Product`, `Order` — not `Users`, `Products`, `Orders`.  
Collections of them are still plural variables: `store users as ...`.

### Methods and properties

Use snake_case for methods and properties, consistent with actions and variables:

```wfl
create container Person:
    property first_name: Text
    property age: Number

    action greet:
        display "Hello, I am " with first_name
    end

    action set_email needs new_email: Text:
        store email as new_email
    end

    action get_info: Text
        return first_name with " (" with age with ")"
    end
end
```

## Constants

True immutability is limited today; **uppercase signals “do not reassign”** by convention:

```wfl
store MAX_USERS as 100
store DEFAULT_TIMEOUT as 30
store API_VERSION as "v1"
store PI as 3.14159
```

Use SCREAMING_SNAKE_CASE for values that act as fixed configuration or mathematical constants.  
See also: [Variables and Types — Constants](../03-language-basics/variables-and-types.md#constants).

## Boolean Names

### Affirmative prefixes

```wfl
store is_active as yes
store has_permission as no
store can_edit as yes
store should_retry as no
store will_expire as no
```

**Common prefixes:** `is_`, `has_`, `can_`, `should_`, `will_`

Prefer positive phrasing (`is_ready`) over double negatives (`is_not_failed`).

## List and Collection Names

### Plural nouns

```wfl
create list users:
end list

create list products:
end list

create list error_messages:
end list

create list pending_tasks:
end list

store colors as ["red", "green", "blue"]
```

Element variables in loops are usually singular:

```wfl
for each user in users:
    display user
end for
```

## File and Path Names

Prefer lowercase with underscores in path strings and WFL source filenames:

```wfl
store config_file as "app_config.txt"
store output_path as "reports/monthly_summary.pdf"
store log_file as "application.log"
```

Examples of program names: `temperature_converter.wfl`, `user_auth.wfl`.

## Pattern Names

Name patterns after **what they match**, not how they are implemented:

```wfl
create pattern email_address:
    one or more letter or digit
    followed by "@"
    one or more letter or digit
end pattern

create pattern us_phone_number:
    one or more digit
end pattern

create pattern iso_date_format:
    one or more digit
end pattern
```

## Complete Examples

### Good naming

```wfl
// Clear, self-documenting code
store customer_first_name as "Alice"
store customer_last_name as "Johnson"
store customer_age as 28
store is_premium_member as yes

define action called calculate_loyalty_discount with parameters purchase_amount:
    check if is_premium_member is yes:
        return purchase_amount times 0.9  // 10% member discount
    otherwise:
        return purchase_amount
    end check
end action

store discounted_total as calculate_loyalty_discount of 100.00
display "Total after discount: $" with discounted_total
```

### Poor naming

```wfl
// Cryptic abbreviations — hard to maintain
store cfn as "Alice"
store cln as "Johnson"
store ca as 28
store pm as yes

define action called calc_d with parameters amt:
    check if pm is yes:
        return amt times 0.9
    otherwise:
        return amt
    end check
end action

store dt as calc_d of 100.00
display "Total: $" with dt
```

## Checklist

- [ ] Variables and actions use **snake_case** (or a deliberate, project-wide spaced style)
- [ ] Names are **descriptive**; no cryptic abbreviations
- [ ] Actions are **verb phrases**
- [ ] Containers are **PascalCase** singular nouns
- [ ] Booleans use **is_ / has_ / can_** (or similar)
- [ ] Collections are **plural**
- [ ] No bare **reserved keywords** as names
- [ ] Style is **consistent** across the project
- [ ] `wfl --lint` is clean for `LINT-NAME` (or intentional exceptions)

## Do and Don't

✅ **Do**

- Use full words: `customer` not `cust`
- Be specific: `email_address` not `data`
- Prefer snake_case throughout a project
- Prefix booleans: `is_`, `has_`, `can_`
- Use plurals for collections: `users`, `items`
- Use verbs for actions: `calculate`, `validate`, `send`

❌ **Don't**

- Abbreviate unless universal (`HTTP`, `URL`, `API`, `ID`)
- Use vague names: `tmp`, `data`, `val`, `stuff`
- Mix snake_case and spaced names in the same project
- Use reserved keywords as bare names
- Name containers with plurals (`Users`) when you mean a type (`User`)

## Related Guides

- **[Code Style Guide](code-style-guide.md)** — Indentation, line length, keywords, blocks
- **[Keyword Reference](../reference/keyword-reference.md)** — Scannable keyword list
- **[Reserved Keywords](../reference/reserved-keywords.md)** — Full classification and pitfalls
- **[Variables and Types](../03-language-basics/variables-and-types.md)** — Scope, types, constants
- **[Project Organization](project-organization.md)** — File and module layout

## What You've Learned

- Project default: **snake_case** for variables, actions, and parameters
- Spaced names as a valid, must-be-consistent alternative
- Container **PascalCase**, method **snake_case**
- Constant, boolean, list, pattern, and path naming patterns
- How to avoid reserved-keyword collisions
- How naming pairs with lint (`LINT-NAME`) and `.wflcfg` — full config keys: **[Configuration Reference](../reference/configuration-reference.md)**

**Next:** [Error Handling Patterns →](error-handling-patterns.md)

---

**Previous:** [← Code Style Guide](code-style-guide.md) | **Next:** [Error Handling Patterns →](error-handling-patterns.md)
