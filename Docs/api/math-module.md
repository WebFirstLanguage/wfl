# WFL Math Module API Reference

## Overview

The Math module provides essential mathematical functions for numeric computations. All functions are optimized for performance and handle edge cases gracefully.

## Functions

### `abs(number)`

Returns the absolute value of a number.

**Parameters:**
- `number` (Number): The number to get the absolute value of

**Returns:** Number (always non-negative)

**Examples:**

```wfl
// Basic absolute values
store positive as abs of 5      // 5
store also_positive as abs of -5 // 5
store zero as abs of 0          // 0

// With variables
store temperature as -15
store temp_magnitude as abs of temperature
display "Temperature magnitude: " with temp_magnitude  // 15

// With expressions
store difference as abs of (x - y)
store distance as abs of (point1 - point2)
```

**Natural Language Variants:**
```wfl
// All equivalent ways to get absolute value
store result as abs of -5
store result as absolute of -5
store result as absolute value of -5
```

**Practical Use Cases:**

```wfl
// Distance calculation
action distance_between with point1 and point2:
    return abs of (point1 - point2)
end

// Error tolerance checking
action is_close with value1 and value2 and tolerance:
    store difference as abs of (value1 - value2)
    return difference < tolerance
end
```

---

### `round(number)`

Rounds a number to the nearest integer using standard rounding rules (0.5 rounds up).

**Parameters:**
- `number` (Number): The number to round

**Returns:** Number (integer)

**Examples:**

```wfl
// Basic rounding
store rounded1 as round of 3.2   // 3
store rounded2 as round of 3.7   // 4
store rounded3 as round of 3.5   // 4
store rounded4 as round of -3.5  // -3

// With variables
store price as 19.99
store rounded_price as round of price
display "Rounded price: $" with rounded_price  // 20

// Banking calculations
store total as 47.834
store bill_amount as round of total
display "Bill amount: $" with bill_amount  // 48
```

**Natural Language Variants:**
```wfl
// All equivalent ways to round
store result as round of 3.7
store result as rounded 3.7
store result as round up 3.7 to nearest integer
```

---

### `floor(number)`

Rounds a number down to the nearest integer (towards negative infinity).

**Parameters:**
- `number` (Number): The number to floor

**Returns:** Number (integer, always ≤ input)

**Examples:**

```wfl
// Basic floor operations
store floored1 as floor of 3.2   // 3
store floored2 as floor of 3.9   // 3
store floored3 as floor of -2.1  // -3
store floored4 as floor of -2.9  // -3

// Age calculation (always round down)
store age_in_years as floor of (days_alive / 365.25)

// Grid positioning
store grid_x as floor of (pixel_x / grid_size)
store grid_y as floor of (pixel_y / grid_size)
```

**Natural Language Variants:**
```wfl
// All equivalent ways to floor
store result as floor of 3.9
store result as rounded down 3.9
store result as floor value of 3.9
```

---

### `ceil(number)`

Rounds a number up to the nearest integer (towards positive infinity).

**Parameters:**
- `number` (Number): The number to ceiling

**Returns:** Number (integer, always ≥ input)

**Examples:**

```wfl
// Basic ceiling operations
store ceiled1 as ceil of 3.1   // 4
store ceiled2 as ceil of 3.9   // 4
store ceiled3 as ceil of -2.1  // -2
store ceiled4 as ceil of -2.9  // -2

// Calculate pages needed
store pages_needed as ceil of (total_items / items_per_page)

// Shipping boxes calculation
store boxes_needed as ceil of (total_weight / max_weight_per_box)
```

**Natural Language Variants:**
```wfl
// All equivalent ways to ceiling
store result as ceil of 3.1
store result as ceiling of 3.1
store result as rounded up 3.1
store result as ceil value of 3.1
```

---

### `random()`

Returns a pseudo-random number between 0 (inclusive) and 1 (exclusive).

**Parameters:** None

**Returns:** Number (0 ≤ result < 1)

**Examples:**

```wfl
// Basic random number
store rand as random
display "Random number: " with rand  // e.g., 0.7234567

// Random integer in range [1, 6] (dice roll)
store dice_roll as floor of (random * 6) + 1
display "Dice roll: " with dice_roll

// Random choice from list
store colors as ["red", "green", "blue", "yellow"]
store random_index as floor of (random * length of colors)
store random_color as colors[random_index]
display "Random color: " with random_color

// Random percentage
store percentage as round of (random * 100)
display "Random percentage: " with percentage with "%"
```

**Practical Use Cases:**

```wfl
// Random password character
action random_digit:
    return floor of (random * 10)
end

// Coin flip
action coin_flip:
    check if random < 0.5:
        return "heads"
    otherwise:
        return "tails"
    end
end

// Random wait time (between 1-5 seconds)
action random_delay:
    store delay as 1 + (random * 4)
    return delay
end
```

---

### `clamp(value, min, max)`

Constrains a value to be within the specified minimum and maximum range.

**Parameters:**
- `value` (Number): The value to constrain
- `min` (Number): The minimum allowed value
- `max` (Number): The maximum allowed value

**Returns:** Number (min ≤ result ≤ max)

**Examples:**

```wfl
// Basic clamping
store clamped1 as clamp of 15 and 0 and 10   // 10 (too high)
store clamped2 as clamp of -5 and 0 and 10   // 0 (too low)
store clamped3 as clamp of 7 and 0 and 10    // 7 (within range)

// Volume control (0-100)
store user_volume as 150
store safe_volume as clamp of user_volume and 0 and 100
display "Volume set to: " with safe_volume  // 100

// RGB color values (0-255)
store red_value as clamp of user_red and 0 and 255
store green_value as clamp of user_green and 0 and 255
store blue_value as clamp of user_blue and 0 and 255

// Progress bar (0-100%)
store progress as clamp of completion_ratio * 100 and 0 and 100
```

**Natural Language Variants:**
```wfl
// All equivalent ways to clamp
store result as clamp of value and min and max
store result as constrain value between min and max
store result as limit value to min and max
store result as bound value by min and max
```

**Practical Use Cases:**

```wfl
// Safe array indexing
action safe_get_item with list and index:
    store safe_index as clamp of index and 0 and (length of list - 1)
    return list[safe_index]
end

// Normalize percentage
action normalize_percentage with value:
    return clamp of value and 0 and 100
end

// Screen boundary collision
action keep_player_on_screen with x and y:
    store safe_x as clamp of x and 0 and screen_width
    store safe_y as clamp of y and 0 and screen_height
    return [safe_x, safe_y]
end
```

## Advanced Examples

### Mathematical Calculations

```wfl
// Distance formula using multiple math functions
action distance with x1 and y1 and x2 and y2:
    store dx as abs of (x2 - x1)
    store dy as abs of (y2 - y1)
    // Note: WFL doesn't have sqrt, so we approximate
    store distance_squared as (dx * dx) + (dy * dy)
    return distance_squared  # Could add sqrt when available
end

// Percentage calculation with rounding
action calculate_percentage with part and whole:
    store percentage as (part / whole) * 100
    return round of percentage
end

// Generate random number in range
action random_between with min and max:
    store range as max - min
    store random_value as min + (random * range)
    return random_value
end
```

### Game Development Examples

```wfl
// Dice rolling system
action roll_dice with sides:
    return floor of (random * sides) + 1
end

// Health calculation with clamping
action apply_damage with current_health and damage and max_health:
    store new_health as current_health - damage
    return clamp of new_health and 0 and max_health
end

// Random spawn position
action random_spawn_position:
    store x as random_between of -50 and 50
    store y as random_between of -50 and 50
    return [x, y]
end
```

### Statistical Functions

```wfl
// Calculate average and round to nearest integer
action average_score with scores:
    store total as 0
    count score in scores:
        store total as total + score
    end
    store average as total / length of scores
    return round of average
end

// Find range (max - min)
action calculate_range with numbers:
    store min_val as numbers[0]
    store max_val as numbers[0]
    
    count num in numbers:
        check if num < min_val:
            store min_val as num
        end
        check if num > max_val:
            store max_val as num
        end
    end
    
    return abs of (max_val - min_val)
end
```

## Error Handling

Math functions handle edge cases gracefully:

- **Division by zero**: Not handled by these functions (handled by language operators)
- **Invalid inputs**: Type checking ensures only numbers are accepted
- **Overflow/underflow**: Handled by underlying Rust f64 implementation
- **NaN/Infinity**: Results may include special float values

```wfl
// Safe division with math functions
action safe_divide with numerator and denominator:
    check if denominator is 0:
        return nothing
    end
    
    store result as numerator / denominator
    return round of result  // Example using math function
end
```

## Performance Notes

- All math functions are implemented in Rust for optimal performance
- Random number generation uses system time as seed (simple but effective)
- Floating-point operations follow IEEE 754 standard
- Functions are designed for single-threaded use

## Best Practices

1. **Use appropriate rounding**: Choose `round`, `floor`, or `ceil` based on your needs
2. **Clamp user inputs**: Always validate ranges for user-provided numbers
3. **Seed randomness appropriately**: For games, consider time-based or user-input seeding
4. **Handle edge cases**: Check for division by zero, negative inputs where inappropriate

```wfl
// Example of best practices
action process_user_score with raw_score:
    // Clamp to valid range
    store score as clamp of raw_score and 0 and 100
    
    // Round for display
    store display_score as round of score
    
    // Calculate grade with appropriate rounding
    check if score >= 90:
        return "A"
    check if score >= 80:
        return "B"  
    check if score >= 70:
        return "C"
    check if score >= 60:
        return "D"
    otherwise:
        return "F"
    end
end
```

## See Also

- [Core Module](core-module.md) - Basic utilities and type checking
- [Text Module](text-module.md) - String manipulation functions
- [List Module](list-module.md) - Collection operations
- [WFL Language Reference](../language-reference/wfl-spec.md) - Complete language specification