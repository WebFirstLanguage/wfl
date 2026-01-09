# Operator Reference

Complete list of WFL operators with precedence and examples.

## Arithmetic Operators

| Operator | Natural Language | Symbol | Example | Result |
|----------|------------------|--------|---------|--------|
| Addition | `plus` | `+` | `5 plus 3` | 8 |
| Subtraction | `minus` | `-` | `10 minus 4` | 6 |
| Multiplication | `times` | `*` | `6 times 7` | 42 |
| Division | `divided by` | `/` | `20 divided by 4` | 5 |
| Modulo | `modulo` | `%` | `10 % 3` | 1 |

**Examples:**
```wfl
store sum as 5 plus 3                    // 8
store difference as 10 minus 4           // 6
store product as 6 times 7               // 42
store quotient as 20 divided by 4        // 5
store remainder as 10 % 3                // 1
```

## Comparison Operators

| Operator | Natural Language | Example |
|----------|------------------|---------|
| Equal | `is equal to`, `is` | `x is 5` |
| Not Equal | `is not equal to`, `is not` | `x is not 5` |
| Greater Than | `is greater than` | `x is greater than 5` |
| Greater or Equal | `is greater than or equal to` | `x is greater than or equal to 5` |
| Less Than | `is less than` | `x is less than 5` |
| Less or Equal | `is less than or equal to` | `x is less than or equal to 5` |
| Above | `is above` | `x is above 5` |
| Below | `is below` | `x is below 5` |

**Examples:**
```wfl
check if age is equal to 18:
check if age is 18:                      // Shorthand
check if age is not 21:
check if score is greater than 90:
check if temperature is above 30:
```

## Logical Operators

| Operator | Example | Description |
|----------|---------|-------------|
| `and` | `a and b` | Both must be true |
| `or` | `a or b` | At least one true |
| `not` | `not a` | Negation |

**Examples:**
```wfl
check if age is greater than 18 and has_license is yes:
check if is_weekend is yes or is_holiday is yes:
check if not is_raining:
```

**Precedence:**
1. `not` (highest)
2. `and`
3. `or` (lowest)

## String Operator

| Operator | Example | Result |
|----------|---------|--------|
| `with` | `"Hello" with " World"` | "Hello World" |

**Examples:**
```wfl
display "Name: " with name
store full_name as first_name with " " with last_name
```

## Order of Operations

**Precedence (highest to lowest):**

1. **Parentheses** (when supported)
2. **Multiplication, Division, Modulo** (left to right)
3. **Addition, Subtraction** (left to right)
4. **Comparison** (is, greater than, etc.)
5. **Logical NOT**
6. **Logical AND**
7. **Logical OR**

**Examples:**
```wfl
2 plus 3 times 4              // 14 (not 20)
// = 2 + (3 * 4) = 2 + 12 = 14

10 minus 2 times 3            // 4 (not 24)
// = 10 - (2 * 3) = 10 - 6 = 4
```

**Best practice:** Use intermediate variables for clarity:

```wfl
store step1 as 3 times 4      // 12
store result as 2 plus step1  // 14
```

## Associativity

All operators are **left-to-right**:

```wfl
10 minus 5 minus 2            // 3
// = (10 - 5) - 2 = 5 - 2 = 3

8 divided by 4 divided by 2   // 1
// = (8 / 4) / 2 = 2 / 2 = 1
```

## Special Operators

### Access Operators

```wfl
container.property            // Property access
container.action()            // Method call
list[0]                       // Index access
```

### Type Operators

```wfl
typeof of value               // Type checking
value matches pattern         // Pattern matching
```

---

**Previous:** [← Keyword Reference](keyword-reference.md) | **Next:** [Built-in Functions →](builtin-functions-reference.md)
