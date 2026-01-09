# Comments and Documentation

Comments help you and others understand your code. WFL supports comments for documenting your programs.

## Single-Line Comments

Use `//` for single-line comments:

```wfl
// This is a comment
display "Hello, World!"

store age as 25  // This is also a comment
```

**Everything after `//` on that line is ignored by WFL.**

### Comment Placement

**Before a line:**
```wfl
// Calculate the total
store total as subtotal plus tax
```

**End of a line:**
```wfl
store tax rate as 0.08  // 8% tax
```

**Multiple comments:**
```wfl
// This program greets the user
// Written on 2026-01-09
// Author: Alice

store name as "World"
display "Hello, " with name
```

## When to Use Comments

### 1. Explain Why, Not What

**Bad (states the obvious):**
```wfl
// Store age as 25
store age as 25

// Check if age is greater than 18
check if age is greater than 18:
    display "Adult"
end check
```

The code already says what it does!

**Good (explains reasoning):**
```wfl
// Using 21 as minimum age for alcohol laws in US
store drinking age as 21

check if age is greater than or equal to drinking age:
    display "Can purchase alcohol"
end check
```

### 2. Explain Complex Logic

```wfl
// Calculate leap year using standard algorithm:
// Divisible by 4 AND (not divisible by 100 OR divisible by 400)
store year as 2024
store is leap as no

check if year modulo 4 is equal to 0:
    check if year modulo 100 is not equal to 0 or year modulo 400 is equal to 0:
        change is leap to yes
    end check
end check
```

### 3. Mark TODOs and FIXMEs

```wfl
// TODO: Add input validation
store user input as "sample"

// FIXME: This calculation is incorrect for negative numbers
store result as abs of value

// NOTE: This is a temporary workaround
store default value as 0
```

### 4. Explain Workarounds

```wfl
// Workaround for issue #123: Using slower algorithm
// until performance issue is fixed
store result as slow but reliable calculation()
```

### 5. Document Algorithms

```wfl
// Binary search implementation
// Requires sorted list
// Returns index if found, -1 if not found
define action called binary search with parameters list and target:
    // Implementation...
end action
```

## What NOT to Comment

### Don't State the Obvious

**Bad:**
```wfl
// Create a list called colors
create list colors:
    // Add red to the list
    add "red"
    // Add green to the list
    add "green"
end list

// Loop through each color
for each color in colors:
    // Display the color
    display color
end for
```

**This is over-commented!** The code is self-explanatory.

**Better:**
```wfl
// Primary colors for the color picker
create list colors:
    add "red"
    add "green"
    add "blue"
end list

for each color in colors:
    display color
end for
```

### Don't Comment Bad Code

**Bad:**
```wfl
// x is the user's age
store x as 25

// Check if x is greater than 18
check if x is greater than 18:
    display "y"  // y means yes
end check
```

**Good (fix the code, remove comments):**
```wfl
store user age as 25

check if user age is greater than or equal to 18:
    display "User is an adult"
end check
```

**Self-documenting code is better than comments!**

## Self-Documenting Code

WFL's natural language syntax often eliminates the need for comments:

### Before (Other Languages)

```javascript
// Check if user has permission and is active
if (u.p && u.a) {
    grant(u);
}
```

Needs a comment because the code is cryptic.

### After (WFL)

```wfl
check if user has permission is yes and user is active is yes:
    grant access to user
end check
```

No comment needed—the code explains itself!

### Examples of Self-Documenting WFL

**Variables:**
```wfl
// Don't need comments:
store customer total balance as 1000.00
store minimum account balance as 25.00
store is premium member as yes
```

**Actions:**
```wfl
// Action name explains what it does:
define action called calculate shipping cost with parameters weight and destination:
    // Implementation
end action
```

**Loops:**
```wfl
// Clear intent:
for each customer in premium customers:
    send loyalty reward to customer
end for
```

## Comment Styles

### Section Headers

```wfl
// ======================
// === Configuration ===
// ======================

store max users as 100
store timeout seconds as 30

// ==================
// === Main Logic ===
// ==================

display "Starting application..."
```

### Function Documentation

```wfl
// calculate_discount
//
// Calculates discount based on customer tier and order amount
//
// Parameters:
//   - amount: Order total before discount
//   - tier: Customer tier ("bronze", "silver", "gold")
//
// Returns:
//   - Discounted amount
//
define action called calculate discount with parameters amount and tier:
    check if tier is "gold":
        return amount times 0.8  // 20% discount
    otherwise:
        check if tier is "silver":
            return amount times 0.9  // 10% discount
        otherwise:
            return amount  // No discount
        end check
    end check
end action
```

### File Headers

```wfl
// shopping_cart.wfl
//
// Shopping cart calculator for e-commerce platform
//
// Created: 2026-01-09
// Author: Alice Johnson
// Purpose: Calculate cart totals with tax and discounts
//

display "=== Shopping Cart ==="
// Program code...
```

## Multi-Line Comments

WFL currently supports only single-line comments with `//`.

For multi-line comments, use multiple `//` lines:

```wfl
// This is a longer explanation
// that spans multiple lines.
// Each line needs its own // prefix.
//
// You can add blank comment lines for readability.
```

## Commenting Best Practices

### DO

✅ **Explain complex algorithms**
```wfl
// Using Sieve of Eratosthenes for prime number generation
```

✅ **Document assumptions**
```wfl
// Assumes input is already sorted
```

✅ **Warn about limitations**
```wfl
// Note: This only works for positive numbers
```

✅ **Credit sources**
```wfl
// Algorithm from: https://example.com/article
```

✅ **Mark temporary code**
```wfl
// TODO: Replace with database query
store users as ["Alice" and "Bob"]
```

### DON'T

❌ **Explain what code does** (if code is clear)
```wfl
// Loop from 1 to 10  ← Unnecessary
count from 1 to 10:
```

❌ **Comment out code** (use version control instead)
```wfl
// store old value as 10
// display old value
```

❌ **Leave misleading comments**
```wfl
// Add 5 to x
store x as x plus 10  // Comment is wrong!
```

❌ **Over-comment simple code**

## Documentation Comments

For actions that will be used by others, use structured comments:

```wfl
// format_currency
//
// Formats a number as currency with dollar sign and 2 decimals
//
// Parameters:
//   amount (Number): The amount to format
//
// Returns:
//   Text: Formatted currency string (e.g., "$19.99")
//
// Example:
//   store price as format currency with 19.99
//   // Result: "$19.99"
//
define action called format currency with parameters amount:
    // Round to 2 decimal places (simplified)
    return "$" with amount
end action
```

## Special Comment Tags

Use standard tags for better organization:

### TODO

Mark incomplete work:

```wfl
// TODO: Add input validation
store user input as "sample"

// TODO: Implement error handling
store result as risky operation()
```

### FIXME

Mark broken code that needs fixing:

```wfl
// FIXME: This doesn't handle negative numbers correctly
store abs value as value times -1
```

### HACK

Mark non-ideal solutions:

```wfl
// HACK: Temporary workaround until API is fixed
store hardcoded value as 42
```

### NOTE

Important information:

```wfl
// NOTE: This must run before initialize()
store config as load configuration()
```

### OPTIMIZE

Mark performance improvements:

```wfl
// OPTIMIZE: This could be cached
store expensive result as complex calculation()
```

## Example: Well-Commented Program

```wfl
// temperature_converter.wfl
//
// Converts temperatures between Celsius and Fahrenheit
// Demonstrates proper commenting practices
//

// ===================
// === Constants ===
// ===================

// Conversion formula: F = C × 9/5 + 32
store celsius to fahrenheit multiplier as 9 divided by 5
store celsius to fahrenheit offset as 32

// ==================
// === Functions ===
// ==================

// Convert Celsius to Fahrenheit
// Uses standard conversion formula
define action called c to f with parameters celsius:
    store fahrenheit as celsius times celsius to fahrenheit multiplier plus celsius to fahrenheit offset
    return fahrenheit
end action

// Convert Fahrenheit to Celsius
// Inverse of c_to_f formula
define action called f to c with parameters fahrenheit:
    store celsius as fahrenheit minus celsius to fahrenheit offset divided by celsius to fahrenheit multiplier
    return celsius
end action

// ================
// === Main ===
// ================

display "=== Temperature Converter ==="
display ""

// Test conversions
store temp c as 25
store temp f as c to f with temp c
display temp c with "°C = " with temp f with "°F"

store temp f2 as 77
store temp c2 as f to c with temp f2
display temp f2 with "°F = " with temp c2 with "°C"

// TODO: Add user input support
// FIXME: Rounding is imprecise, consider formatting
```

**Balance:** This is well-commented without being over-commented.

## Practice Exercises

### Exercise 1: Comment Your Code

Take one of your previous programs and add appropriate comments:
- File header
- Action documentation
- Complex logic explanations
- TODO markers where applicable

### Exercise 2: Remove Bad Comments

Review this code and remove unnecessary comments:

```wfl
// Store name
store name as "Alice"

// Check if name is Alice
check if name is "Alice":
    // Display hello
    display "Hello, Alice!"
end check
```

What would you keep? What would you remove?

### Exercise 3: Document an Action

Write an action that calculates the area of a circle. Add proper documentation comments including:
- What it does
- Parameters
- Return value
- Example usage

### Exercise 4: Self-Documenting Code

Rewrite this poorly named code to be self-documenting (no comments needed):

```wfl
store x as 10
store y as 5
store z as x times y
display z
```

## Best Practices Summary

✅ **Comment the why, not the what**

✅ **Keep comments up-to-date** (outdated comments are worse than none)

✅ **Use self-documenting code** when possible

✅ **Document complex algorithms** and tricky logic

✅ **Use standard tags** (TODO, FIXME, NOTE)

✅ **Write documentation for public actions**

❌ **Don't over-comment** obvious code

❌ **Don't leave commented-out code** (use version control)

❌ **Don't write misleading comments**

❌ **Don't use comments as a crutch** for bad code

## Remember

> "Code is read 10 times more often than it's written."

Good comments make code:
- **Easier to maintain** - Future you will thank you
- **Easier to collaborate** - Teammates understand your intent
- **Easier to debug** - Comments provide context
- **Easier to review** - Reviewers see your reasoning

But **self-documenting code** (clear names, natural syntax) is even better than comments!

## WFL's Advantage

WFL's natural language syntax reduces the need for comments:

```wfl
// Traditional languages need this comment:
// "Calculate total price including discount and tax"

// WFL code explains itself:
store subtotal as item price times quantity
store discounted as subtotal times 1 minus discount rate
store tax as discounted times tax rate
store total as discounted plus tax
```

The code reads like the comment would!

## What You've Learned

In this section, you learned:

✅ **Single-line comments** - `// comment text`
✅ **When to comment** - Explain why, not what
✅ **What to comment** - Complex logic, TODOs, assumptions
✅ **What not to comment** - Obvious code, bad code
✅ **Self-documenting code** - Let clear code replace comments
✅ **Documentation comments** - Structured action documentation
✅ **Special tags** - TODO, FIXME, HACK, NOTE
✅ **Best practices** - Keep comments valuable and up-to-date

---

## Language Basics Complete!

Congratulations! You've completed the Language Basics section. You now understand:

✅ Variables and types
✅ Operators and expressions
✅ Control flow (conditionals)
✅ Loops and iteration
✅ Actions (functions)
✅ Lists and collections
✅ Error handling
✅ Comments and documentation

**You have the fundamentals to write real WFL programs!**

## What's Next?

### Build Something

Apply what you've learned:
- Create a calculator
- Build a todo list manager
- Make a file processor
- Write a simple game

**[Cookbook →](../guides/cookbook.md)** - Recipes for common tasks

### Learn Advanced Features

Ready for more power?

**[Advanced Features →](../04-advanced-features/index.md)**
- Async programming
- Web servers
- File I/O
- Pattern matching
- Object-oriented programming (containers)

### Explore the Standard Library

**[Standard Library →](../05-standard-library/index.md)**
- 181+ built-in functions
- Math, text, list, filesystem, time, random, crypto, and more

### Write Better Code

**[Best Practices →](../06-best-practices/index.md)**
- Code style guide
- Security guidelines
- Performance tips
- Testing strategies

---

**Keep practicing!** The more you code, the more natural WFL will feel.

---

**Previous:** [← Error Handling](error-handling.md) | **Next:** [Advanced Features →](../04-advanced-features/index.md)
