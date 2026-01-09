# Naming Conventions

Good names make code self-documenting. WFL's natural language syntax encourages descriptive, readable names.

## General Principles

✅ **Use descriptive names** - Code should read like English
✅ **Be specific** - `customer_email` not `email` or `e`
✅ **Avoid abbreviations** - Unless universally understood
✅ **Use consistent style** - Pick snake_case or spaces

## Variable Names

### Preferred: snake_case

```wfl
store user_name as "Alice"
store account_balance as 1000.00
store is_verified as yes
store total_count as 0
```

### Alternative: Spaces (Natural Language)

```wfl
store user name as "Alice"
store account balance as 1000.00
store is verified as yes
store total count as 0
```

Both work. **Be consistent within a project.**

### Descriptive Over Cryptic

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

### Avoid Reserved Keywords

60+ keywords are reserved. **[See complete list →](../03-language-basics/variables-and-types.md#reserved-keywords)**

**Common conflicts:**
- `is` → Use `is_value` or `is_valid`
- `file` → Use `filename` or `file_handle`
- `add` → Use `addition` or `add_result`
- `current` → Use `current_value` or `current_item`

## Action Names

### Use Verb Phrases

```wfl
define action called calculate_total with parameters items:
define action called validate_email with parameters address:
define action called send_notification with parameters user:
define action called format_date with parameters date:
```

### Be Specific

**Good:**
```wfl
define action called calculate_discount_for_member
define action called send_welcome_email
define action called validate_credit_card
```

**Poor:**
```wfl
define action called calc
define action called process
define action called do_stuff
```

## Container Names

### Use PascalCase

```wfl
create container Person:
create container ShoppingCart:
create container EmailValidator:
create container DatabaseConnection:
```

### Singular Nouns

```wfl
create container User:        // Not Users
create container Product:     // Not Products
create container Order:       // Not Orders
```

## Constants

### Use SCREAMING_SNAKE_CASE (Convention)

```wfl
store MAX_USERS as 100
store DEFAULT_TIMEOUT as 30
store API_VERSION as "v1"
store PI as 3.14159
```

**Note:** WFL doesn't enforce immutability yet, but uppercase signals intent.

## Boolean Names

### Use Affirmative Prefixes

```wfl
store is_active as yes
store has_permission as no
store can_edit as yes
store should_retry as no
```

**Prefixes:** `is_`, `has_`, `can_`, `should_`, `will_`

## List Names

### Use Plural Nouns

```wfl
create list users
create list products
create list error_messages
create list pending_tasks
```

## File and Path Names

### Lowercase with Underscores

```wfl
store config_file as "app_config.txt"
store output_path as "reports/monthly_summary.pdf"
store log_file as "application.log"
```

## Pattern Names

### Descriptive of What They Match

```wfl
create pattern email_address:
create pattern us_phone_number:
create pattern credit_card_number:
create pattern iso_date_format:
```

## Examples

### Good Naming

```wfl
// Clear, self-documenting code
store customer_first_name as "Alice"
store customer_last_name as "Johnson"
store customer_age as 28
store is_premium_member as yes

define action called calculate_loyalty_discount with parameters purchase_amount:
    check if is_premium_member is yes:
        return purchase_amount times 0.9  // 10% discount
    otherwise:
        return purchase_amount
    end check
end action

store discounted_total as calculate_loyalty_discount with 100.00
display "Total after discount: $" with discounted_total
```

### Poor Naming

```wfl
// Cryptic, hard to understand
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

store dt as calc_d with 100.00
display "Total: $" with dt
```

## Best Practices

✅ **Use full words** - `customer` not `cust`
✅ **Be specific** - `email_address` not `data`
✅ **Use consistent style** - snake_case throughout project
✅ **Avoid single letters** - Except in math: `x`, `y`, `i`
✅ **Prefix booleans** - `is_`, `has_`, `can_`
✅ **Plural for collections** - `users`, `items`
✅ **Verbs for actions** - `calculate`, `validate`, `send`
✅ **Check reserved words** - Use underscores if conflict

❌ **Don't abbreviate** - Unless universal (HTTP, URL, API)
❌ **Don't use cryptic names** - `tmp`, `data`, `val`
❌ **Don't mix styles** - Pick snake_case OR spaces
❌ **Don't use reserved keywords** - Parser will reject them

## What You've Learned

✅ Variable naming (snake_case or spaces)
✅ Action naming (verb phrases)
✅ Container naming (PascalCase)
✅ Boolean naming (is_, has_, can_)
✅ List naming (plural nouns)
✅ Reserved keyword avoidance
✅ Descriptive over cryptic

**Next:** [Error Handling Patterns →](error-handling-patterns.md)

---

**Previous:** [← Code Style Guide](code-style-guide.md) | **Next:** [Error Handling Patterns →](error-handling-patterns.md)
