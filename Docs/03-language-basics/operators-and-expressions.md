# Operators and Expressions

Operators let you combine values and perform calculations. WFL uses natural language operators that read like English.

## Arithmetic Operators

Perform mathematical operations on numbers.

### Addition

**Natural language:**
```wfl
store sum as 5 plus 3              // 8
store total as 10 plus 20 plus 30  // 60
```

**Symbol alternative:**
```wfl
store sum as 5 + 3                 // Also valid
```

### Subtraction

**Natural language:**
```wfl
store difference as 10 minus 3     // 7
store remaining as 100 minus 25    // 75
```

**Symbol alternative:**
```wfl
store difference as 10 - 3         // Also valid
```

### Multiplication

**Natural language:**
```wfl
store product as 5 times 4         // 20
store area as width times height
```

**Symbol alternative:**
```wfl
store product as 5 * 4             // Also valid
```

### Division

**Natural language:**
```wfl
store quotient as 20 divided by 4  // 5
store half as 10 divided by 2      // 5
```

**Symbol alternative:**
```wfl
store quotient as 20 / 4           // Also valid
```

### Modulo (Remainder)

Get the remainder after division:

```wfl
store remainder as 10 modulo 3     // 1 (10 ÷ 3 = 3 remainder 1)
store check as 15 modulo 5         // 0 (15 ÷ 5 = 3 remainder 0)
```

**Symbol alternative:**
```wfl
store remainder as 10 % 3          // Also valid
```

**Common use:** Check if a number is even:

```wfl
store number as 42
check if number modulo 2 is equal to 0:
    display "Even number"
otherwise:
    display "Odd number"
end check
```

### Combined Arithmetic

```wfl
store result as 2 plus 3 times 4   // Order of operations applies
store complex as 10 plus 5 times 2 minus 3 divided by 1
```

## Comparison Operators

Compare values and return yes/no (boolean).

### Equality

**Is Equal To:**
```wfl
check if 5 is equal to 5:          // yes
    display "Equal!"
end check

check if age is 25:                // Shorthand
    display "You are 25"
end check
```

**Is Not Equal:**
```wfl
check if 5 is not equal to 3:      // yes
    display "Not equal!"
end check

check if name is not "Alice":
    display "You're not Alice"
end check
```

### Greater Than

```wfl
check if 10 is greater than 5:     // yes
    display "10 > 5"
end check

// Shorthand: "is greater"
check if age is greater than 18:
    display "Adult"
end check
```

### Greater Than or Equal

```wfl
check if 5 is greater than or equal to 5:   // yes
    display "5 >= 5"
end check

check if score is greater than or equal to 90:
    display "A grade"
end check
```

### Less Than

```wfl
check if 3 is less than 10:        // yes
    display "3 < 10"
end check

check if temperature is less than 32:
    display "Freezing!"
end check
```

### Less Than or Equal

```wfl
check if 5 is less than or equal to 5:      // yes
    display "5 <= 5"
end check

check if age is less than or equal to 12:
    display "Child"
end check
```

### Alternative Comparison Syntax

**Above/Below (more natural):**
```wfl
check if temperature is above 30:
    display "Hot!"
end check

check if score is below 50:
    display "Failed"
end check
```

## Logical Operators

Combine multiple conditions.

### AND

Both conditions must be true:

```wfl
check if age is greater than 18 and has license is yes:
    display "Can drive"
end check

check if score is greater than or equal to 90 and attendance is good:
    display "Excellent student"
end check
```

### OR

At least one condition must be true:

```wfl
check if is weekend is yes or is holiday is yes:
    display "Day off!"
end check

check if payment method is "cash" or payment method is "card":
    display "Payment accepted"
end check
```

### NOT

Negates a condition:

```wfl
check if not is raining:
    display "Go outside!"
end check

check if not has access:
    display "Access denied"
end check
```

### Combining Logical Operators

```wfl
check if age is greater than or equal to 18 and is citizen is yes or has permit is yes:
    display "Can vote"
end check
```

**Note:** AND has higher precedence than OR. Use parentheses (when supported) or break into separate checks for clarity.

## String Concatenation

Join text and values together using `with`:

```wfl
store first name as "Alice"
store last name as "Smith"

store full name as first name with " " with last name
display full name                  // Output: "Alice Smith"
```

**With numbers:**
```wfl
store age as 25
display "Age: " with age          // Output: "Age: 25"
```

**Multiple values:**
```wfl
store name as "Alice"
store age as 25
store city as "Portland"

display "Name: " with name with ", Age: " with age with ", City: " with city
// Output: "Name: Alice, Age: 25, City: Portland"
```

**NOT for math:**
```wfl
// Wrong way to add numbers:
store sum as 5 with 3             // This concatenates: "53"

// Right way:
store sum as 5 plus 3             // This adds: 8
```

## Order of Operations

WFL follows standard mathematical order of operations (PEMDAS):

1. **Parentheses** (when supported in future versions)
2. **Multiplication and Division** (left to right)
3. **Addition and Subtraction** (left to right)

**Examples:**
```wfl
store result1 as 2 plus 3 times 4
// = 2 + (3 × 4) = 2 + 12 = 14

store result2 as 10 minus 2 times 3
// = 10 - (2 × 3) = 10 - 6 = 4

store result3 as 20 divided by 4 plus 2
// = (20 ÷ 4) + 2 = 5 + 2 = 7
```

**Best practice:** Break complex expressions into steps for clarity:

```wfl
// Instead of:
store result as 10 plus 5 times 2 minus 3 divided by 1

// Do this:
store step1 as 5 times 2           // 10
store step2 as 3 divided by 1      // 3
store step3 as 10 plus step1       // 20
store result as step3 minus step2  // 17
```

## Type-Specific Operations

### Text Operations

```wfl
// Concatenation
store greeting as "Hello" with " " with "World"

// Comparison
check if "apple" is equal to "apple":   // yes
check if "Apple" is equal to "apple":   // no (case-sensitive)
```

### Number Operations

```wfl
// Arithmetic
store x as 10
store y as 3

store sum as x plus y              // 13
store difference as x minus y      // 7
store product as x times y         // 30
store quotient as x divided by y   // 3.333...
store remainder as x modulo y      // 1
```

### Boolean Operations

```wfl
store is active as yes
store is verified as no

check if is active is yes and is verified is yes:
    display "Fully active account"
otherwise:
    display "Account needs verification"
end check
```

## Expressions in Context

### In Display Statements

```wfl
display 5 plus 3                   // Output: 8
display "Result: " with 5 plus 3   // Output: "Result: 8"
```

### In Variable Assignments

```wfl
store result as 10 plus 20 divided by 2
// result = 10 + (20 ÷ 2) = 10 + 10 = 20
```

### In Conditionals

```wfl
check if 10 plus 5 is greater than 12:
    display "Condition is true"
end check
```

### In Loops

```wfl
count from 1 to 5 plus 5:          // Count from 1 to 10
    display the current count
end count
```

## Common Patterns

### Calculate Percentage

```wfl
store total as 100
store percentage as 25
store result as total times percentage divided by 100

display result with "% of " with total with " = " with result
// Output: "25% of 100 = 25"
```

### Temperature Conversion

```wfl
store celsius as 25
store fahrenheit as celsius times 9 divided by 5 plus 32

display celsius with "°C = " with fahrenheit with "°F"
// Output: "25°C = 77°F"
```

### Distance Formula

```wfl
store x1 as 0
store y1 as 0
store x2 as 3
store y2 as 4

store dx as x2 minus x1            // 3
store dy as y2 minus y1            // 4
store distance as dx times dx plus dy times dy
// For actual distance, you'd need sqrt (square root)

display "Distance squared: " with distance   // 25
```

### Age Calculation

```wfl
store current year as 2026
store birth year as 1995
store age as current year minus birth year

display "Age: " with age           // Output: "Age: 31"
```

### Discount Calculation

```wfl
store original price as 100.00
store discount percent as 20
store discount amount as original price times discount percent divided by 100
store final price as original price minus discount amount

display "Original: $" with original price
display "Discount: $" with discount amount
display "Final: $" with final price
```

**Output:**
```
Original: $100.0
Discount: $20.0
Final: $80.0
```

## Comparison Examples

### Range Checking

```wfl
store age as 25

check if age is greater than or equal to 13 and age is less than 20:
    display "Teenager"
end check
```

### Multiple Conditions

```wfl
store temperature as 75
store is raining as no

check if temperature is above 70 and not is raining:
    display "Perfect day for a picnic!"
end check
```

### Eligibility Check

```wfl
store age as 20
store is citizen as yes
store is registered as yes

check if age is greater than or equal to 18 and is citizen is yes and is registered is yes:
    display "Eligible to vote"
otherwise:
    display "Not eligible to vote"
end check
```

## Common Mistakes

### Mixing String Concatenation and Math

**Wrong:**
```wfl
store result as "5" with "3"       // "53" (string concatenation)
```

**Right:**
```wfl
store result as 5 plus 3           // 8 (addition)
```

### Type Mismatches

**Wrong:**
```wfl
store age as 25
store name as "Alice"
display age plus name              // ERROR: Cannot add Number and Text
```

**Right:**
```wfl
display "Name: " with name with ", Age: " with age
```

### Confusing Comparison Operators

**Wrong:**
```wfl
check if age is 18:                // Equality check
    display "Exactly 18"
end check
```

**If you meant "at least 18":**
```wfl
check if age is greater than or equal to 18:
    display "Adult"
end check
```

### Forgetting Order of Operations

**Wrong assumption:**
```wfl
store result as 2 plus 3 times 4   // You might think: (2+3)*4 = 20
// But actually: 2 + (3*4) = 14
```

**Clearer:**
```wfl
store step1 as 3 times 4           // 12
store result as 2 plus step1       // 14
```

## Practice Exercises

### Exercise 1: Calculator

Create a simple calculator that:
1. Stores two numbers
2. Calculates sum, difference, product, quotient
3. Displays all results

### Exercise 2: Circle Calculations

Given a radius of 5:
1. Calculate diameter (2 × radius)
2. Calculate circumference (2 × π × radius, use π = 3.14159)
3. Calculate area (π × radius²)
4. Display all results

### Exercise 3: Grade Calculator

Calculate a student's final grade:
- Homework: 85 (worth 20%)
- Midterm: 90 (worth 30%)
- Final: 88 (worth 50%)

Formula: (homework × 0.20) + (midterm × 0.30) + (final × 0.50)

### Exercise 4: Shopping Cart

Calculate total with tax:
- Item 1: $19.99
- Item 2: $34.50
- Item 3: $12.25
- Tax rate: 8%

Display subtotal, tax, and total.

### Exercise 5: Comparison Practice

Create variables for:
- score1: 85
- score2: 90
- passing_score: 70

Check if:
1. score1 is passing
2. score2 is higher than score1
3. Both scores are passing

## Best Practices

✅ **Use natural language operators:** `plus` is clearer than `+`

✅ **Use `with` for text:** Combine strings and numbers naturally

✅ **Break complex expressions into steps:** Easier to read and debug

✅ **Be explicit with comparisons:** `is greater than or equal to` is clearer than `>=`

✅ **Use descriptive variable names:** Makes expressions self-documenting

❌ **Don't mix types incorrectly:** Can't add text and numbers

❌ **Don't create overly complex expressions:** Break them down

❌ **Don't use `with` for math:** Use `plus`, `minus`, etc.

## Quick Reference

| Operation | Natural | Symbol | Example |
|-----------|---------|--------|---------|
| Addition | `plus` | `+` | `5 plus 3` |
| Subtraction | `minus` | `-` | `10 minus 4` |
| Multiplication | `times` | `*` | `6 times 7` |
| Division | `divided by` | `/` | `20 divided by 4` |
| Modulo | `modulo` | `%` | `10 modulo 3` |
| Equal | `is equal to` | `is` | `x is 5` |
| Not Equal | `is not equal to` | `is not` | `x is not 5` |
| Greater | `is greater than` | - | `x is greater than 5` |
| Less | `is less than` | - | `x is less than 5` |
| AND | `and` | - | `a and b` |
| OR | `or` | - | `a or b` |
| NOT | `not` | - | `not x` |
| Concatenate | `with` | - | `"Hi" with name` |

## What You've Learned

In this section, you learned:

✅ **Arithmetic operators** - plus, minus, times, divided by, modulo
✅ **Comparison operators** - is equal to, is greater than, is less than
✅ **Logical operators** - and, or, not
✅ **String concatenation** - with keyword
✅ **Order of operations** - How WFL evaluates expressions
✅ **Natural language alternatives** - Readable operator names

## Next Steps

Now that you understand operators and expressions:

**[Control Flow →](control-flow.md)**
Use expressions in conditional statements.

Or explore related topics:
- [Variables and Types →](variables-and-types.md) - Review data types
- [Loops and Iteration →](loops-and-iteration.md) - Use expressions in loops
- [Actions (Functions) →](actions-functions.md) - Return expression results

---

**Previous:** [← Variables and Types](variables-and-types.md) | **Next:** [Control Flow →](control-flow.md)
