# Math Module

The Math module provides mathematical operations for numeric calculations. All functions work with WFL's Number type.

## Functions

### abs

**Purpose:** Get the absolute value of a number (removes negative sign).

**Signature:**
```wfl
abs of <number>
```

**Parameters:**
- `number` (Number): The number to process

**Returns:** Number - The absolute value

**Example:**
```wfl
store result1 as abs of -5
display result1                    // Output: 5

store result2 as abs of 10
display result2                    // Output: 10

store result3 as abs of -3.14
display result3                    // Output: 3.14
```

**Use Cases:**
- Distance calculations
- Difference without direction
- Always-positive values

---

### round

**Purpose:** Round a number to the nearest integer.

**Signature:**
```wfl
round of <number>
```

**Parameters:**
- `number` (Number): The number to round

**Returns:** Number - The rounded integer

**Rounding Rules:**
- 0.5 and above rounds up
- Below 0.5 rounds down

**Example:**
```wfl
display round of 3.2               // Output: 3
display round of 3.7               // Output: 4
display round of 3.5               // Output: 4
display round of -2.3              // Output: -2
display round of -2.7              // Output: -3
```

**Use Cases:**
- Currency calculations (round to dollars)
- Display rounded values
- Integer requirements

**Example: Temperature Display**
```wfl
store celsius as 23.8
store rounded as round of celsius
display "Temperature: " with rounded with "°C"
// Output: Temperature: 24°C
```

---

### floor

**Purpose:** Round down to the nearest integer (toward negative infinity).

**Signature:**
```wfl
floor of <number>
```

**Parameters:**
- `number` (Number): The number to round down

**Returns:** Number - The floor value

**Example:**
```wfl
display floor of 3.2               // Output: 3
display floor of 3.9               // Output: 3
display floor of -2.3              // Output: -3
display floor of -2.9              // Output: -3
```

**Use Cases:**
- Always round down
- Integer division results
- Pagination calculations

**Example: Page Numbers**
```wfl
store total_items as 47
store items_per_page as 10
store pages as floor of total_items divided by items_per_page
display "Total pages: " with pages
// Output: Total pages: 4
```

---

### ceil

**Purpose:** Round up to the nearest integer (toward positive infinity).

**Signature:**
```wfl
ceil of <number>
```

**Parameters:**
- `number` (Number): The number to round up

**Returns:** Number - The ceiling value

**Example:**
```wfl
display ceil of 3.1                // Output: 4
display ceil of 3.9                // Output: 4
display ceil of -2.1               // Output: -2
display ceil of -2.9               // Output: -2
```

**Use Cases:**
- Always round up
- Minimum requirements
- Resource allocation

**Example: Packing Boxes**
```wfl
store items as 47
store box_capacity as 10
store boxes_needed as ceil of items divided by box_capacity
display "Boxes needed: " with boxes_needed
// Output: Boxes needed: 5
```

---

### clamp

**Purpose:** Constrain a value between a minimum and maximum.

**Signature:**
```wfl
clamp of <value> between <min> and <max>
```

**Parameters:**
- `value` (Number): The value to clamp
- `min` (Number): Minimum allowed value
- `max` (Number): Maximum allowed value

**Returns:** Number - The clamped value

**Behavior:**
- If value < min, returns min
- If value > max, returns max
- Otherwise, returns value unchanged

**Example:**
```wfl
display clamp of 5 between 0 and 10       // Output: 5
display clamp of -5 between 0 and 10      // Output: 0
display clamp of 15 between 0 and 10      // Output: 10
display clamp of 7.5 between 0 and 10     // Output: 7.5
```

**Use Cases:**
- Limit user input to valid range
- Prevent values from exceeding bounds
- Volume/brightness controls
- Game score limits

**Example: Volume Control**
```wfl
define action called set volume with parameters level:
    store clamped_volume as clamp of level between 0 and 100
    display "Volume set to: " with clamped_volume with "%"
    return clamped_volume
end action

call set volume with 50    // Output: Volume set to: 50%
call set volume with 150   // Output: Volume set to: 100%
call set volume with -10   // Output: Volume set to: 0%
```

**Example: Health Bar in Game**
```wfl
store health as 100
store damage as 30

change health to health minus damage
store health as clamp of health between 0 and 100

display "Health: " with health
// Ensures health stays between 0 and 100
```

---

## Complete Example

Using all math functions together:

```wfl
display "=== Math Module Demo ==="
display ""

// Absolute value
store negative as -42
store positive as abs of negative
display "Absolute value of -42: " with positive

// Rounding
store pi as 3.14159
display "Round: " with round of pi        // 3
display "Floor: " with floor of pi        // 3
display "Ceil: " with ceil of pi          // 4

// More rounding examples
store values as [2.1, 2.5, 2.9, -1.5]
for each value in values:
    display value with " → round: " with round of value with ", floor: " with floor of value with ", ceil: " with ceil of value
end for

display ""

// Clamping
store test_values as [-10, 0, 5, 10, 15, 20]
for each test in test_values:
    store clamped as clamp of test between 0 and 10
    display test with " clamped to [0,10]: " with clamped
end for

display ""

// Practical example: Temperature converter with rounding
store celsius as 23.7
store fahrenheit as celsius times 9 divided by 5 plus 32
store rounded_f as round of fahrenheit

display celsius with "°C = " with fahrenheit with "°F (raw)"
display celsius with "°C ≈ " with rounded_f with "°F (rounded)"

display ""
display "=== Demo Complete ==="
```

**Output:**
```
=== Math Module Demo ===

Absolute value of -42: 42
Round: 3
Floor: 3
Ceil: 4
2.1 → round: 2, floor: 2, ceil: 3
2.5 → round: 3, floor: 2, ceil: 3
2.9 → round: 3, floor: 2, ceil: 3
-1.5 → round: -2, floor: -2, ceil: -1

-10 clamped to [0,10]: 0
0 clamped to [0,10]: 0
5 clamped to [0,10]: 5
10 clamped to [0,10]: 10
15 clamped to [0,10]: 10
20 clamped to [0,10]: 10

23.7°C = 74.66°F (raw)
23.7°C ≈ 75°F (rounded)

=== Demo Complete ===
```

## Common Use Cases

### Currency Calculations

```wfl
store price as 19.99
store quantity as 3
store subtotal as price times quantity
store tax as subtotal times 0.0825
store total as subtotal plus tax
store rounded_total as round of total times 100 divided by 100

display "Total: $" with rounded_total
```

### Percentage Calculations

```wfl
store score as 47
store max_score as 50
store percentage as score divided by max_score times 100
store rounded_pct as round of percentage

display "Score: " with rounded_pct with "%"
```

### User Input Validation

```wfl
define action called validate age with parameters age:
    store clamped_age as clamp of age between 0 and 120
    check if age is not equal to clamped_age:
        display "Age adjusted from " with age with " to " with clamped_age
    end check
    return clamped_age
end action

store user_age as validate age with 150
// Output: Age adjusted from 150 to 120
display "Age: " with user_age
// Output: Age: 120
```

## Best Practices

✅ **Use abs for distances:** Distance is always positive

✅ **Use round for display:** Users don't need many decimals

✅ **Use floor for division:** Integer results from division

✅ **Use ceil for capacity:** Always round up for requirements

✅ **Use clamp for constraints:** Prevent out-of-range values

❌ **Don't round too early:** Do math first, round at the end

❌ **Don't assume integer results:** WFL uses floating-point

## What You've Learned

In this module, you learned:

✅ **abs** - Absolute value function
✅ **round** - Round to nearest integer
✅ **floor** - Round down
✅ **ceil** - Round up
✅ **clamp** - Constrain value to range
✅ **Use cases** - Currency, percentages, validation, games
✅ **Best practices** - When to use each function

## Next Steps

Continue exploring the standard library:

**[Text Module →](text-module.md)**
String manipulation functions.

**[List Module →](list-module.md)**
Working with collections.

**[Random Module →](random-module.md)**
Random number generation (includes more math functions).

Or return to:
**[Standard Library Index →](index.md)**

---

**Previous:** [← Core Module](core-module.md) | **Next:** [Text Module →](text-module.md)
