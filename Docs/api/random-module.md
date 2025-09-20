# WFL Random Module API Reference

## Overview

The Random module provides cryptographically secure random number generation for all randomness needs in WFL applications. All functions use secure random number generators suitable for security-sensitive applications, games, simulations, and general-purpose randomness.

**Security Note**: This module replaces the previous time-based random implementation with cryptographically secure random number generation using the Rust `rand` crate with proper entropy sources.

## Functions

### `random()`

Returns a cryptographically secure random number between 0 (inclusive) and 1 (exclusive).

**Parameters:** None

**Returns:** Number (0 ≤ result < 1)

**Examples:**

```wfl
// Basic random number
store rand as random
display "Random number: " with rand  // e.g., 0.7234567

// Random percentage
store percentage as round of (random * 100)
display "Random percentage: " with percentage with "%"

// Probability check
check if random < 0.3:
    display "30% chance event occurred!"
end check
```

**Security Features:**
- Uses cryptographically secure random number generator
- Properly seeded from system entropy
- Suitable for security-sensitive applications

---

### `random_between(min, max)`

Returns a cryptographically secure random number between the specified minimum and maximum values (both inclusive).

**Parameters:**
- `min` (Number): The minimum value (inclusive)
- `max` (Number): The maximum value (inclusive)

**Returns:** Number (min ≤ result ≤ max)

**Examples:**

```wfl
// Random float in range
store temperature as random_between of -10 and 35
display "Random temperature: " with temperature with "°C"

// Random price
store price as random_between of 9.99 and 99.99
display "Random price: $" with price

// Random coordinate
store x as random_between of -100 and 100
store y as random_between of -100 and 100
display "Random position: (" with x with ", " with y with ")"
```

**Edge Cases:**
```wfl
// Same min and max returns exact value
store exact as random_between of 5 and 5  // Always returns 5

// Negative ranges work correctly
store negative as random_between of -50 and -10
```

---

### `random_int(min, max)`

Returns a cryptographically secure random integer between the specified minimum and maximum values (both inclusive).

**Parameters:**
- `min` (Number): The minimum integer value (inclusive)
- `max` (Number): The maximum integer value (inclusive)

**Returns:** Number (integer, min ≤ result ≤ max)

**Examples:**

```wfl
// Dice roll (1-6)
store dice as random_int of 1 and 6
display "Dice roll: " with dice

// Random age
store age as random_int of 18 and 65
display "Random age: " with age

// Array index selection
store list_size as length of my_list
store random_index as random_int of 0 and (list_size - 1)
store random_item as my_list[random_index]
```

**Practical Use Cases:**
```wfl
// Random ID generation
action generate_user_id:
    return random_int of 100000 and 999999
end

// Random delay (1-10 seconds)
action random_delay_seconds:
    return random_int of 1 and 10
end
```

---

### `random_boolean()`

Returns a cryptographically secure random boolean value (true or false with equal probability).

**Parameters:** None

**Returns:** Boolean (true or false)

**Examples:**

```wfl
// Coin flip
store coin as random_boolean
check if coin:
    display "Heads!"
otherwise:
    display "Tails!"
end check

// Random decision
store should_retry as random_boolean
check if should_retry:
    display "Retrying operation..."
end check

// Random feature flag
store enable_feature as random_boolean
```

**Practical Use Cases:**
```wfl
// Random game events
action random_critical_hit:
    return random_boolean
end

// A/B testing
action assign_test_group:
    check if random_boolean:
        return "group_a"
    otherwise:
        return "group_b"
    end check
end
```

---

### `random_from(list)`

Returns a cryptographically secure random element from the provided list.

**Parameters:**
- `list` (List): The list to select from (must not be empty)

**Returns:** Any (type matches the selected element)

**Examples:**

```wfl
// Random color selection
store colors as ["red" and "green" and "blue" and "yellow"]
store random_color as random_from of colors
display "Selected color: " with random_color

// Random name picker
store names as ["Alice" and "Bob" and "Charlie" and "Diana"]
store winner as random_from of names
display "Winner: " with winner

// Random number from predefined set
store lucky_numbers as [7 and 13 and 21 and 42]
store lucky as random_from of lucky_numbers
display "Your lucky number: " with lucky
```

**Error Handling:**
```wfl
// Empty list handling (will cause runtime error)
store empty_list as []
// store item as random_from of empty_list  // Error: cannot select from empty list
```

**Advanced Examples:**
```wfl
// Random menu item
store menu as [
    "Pizza" and "Burger" and "Salad" and "Pasta" and "Sushi"
]
store todays_special as random_from of menu
display "Today's special: " with todays_special

// Random difficulty level
store difficulties as ["Easy" and "Medium" and "Hard" and "Expert"]
store level as random_from of difficulties
display "Difficulty: " with level
```

---

### `random_seed(seed)`

Sets the random seed for reproducible random number generation. All subsequent random function calls will produce the same sequence when using the same seed.

**Parameters:**
- `seed` (Number): The seed value for the random number generator

**Returns:** Nothing

**Examples:**

```wfl
// Reproducible random sequence
random_seed of 42
store r1 as random
store r2 as random_int of 1 and 10
display "First sequence: " with r1 with ", " with r2

// Reset with same seed
random_seed of 42
store r3 as random
store r4 as random_int of 1 and 10
display "Second sequence: " with r3 with ", " with r4
// r1 == r3 and r2 == r4 (identical sequences)
```

**Use Cases:**
```wfl
// Deterministic testing
action test_random_behavior:
    random_seed of 12345
    // Now all random calls are predictable for testing
    store test_value as random_int of 1 and 100
    return test_value  // Always returns the same value
end

// Game level generation
action generate_level with level_number:
    random_seed of level_number
    // Generate consistent level layout based on level number
    store room_count as random_int of 5 and 15
    store enemy_count as random_int of 1 and room_count
    return [room_count, enemy_count]
end
```

**Important Notes:**
- Seeding affects ALL random functions globally
- Use different seeds for different purposes
- Time-based seeding for unpredictable results:
  ```wfl
  store current_time as timestamp of now
  random_seed of current_time
  ```

## Advanced Examples

### Game Development

```wfl
// Random loot generation
action generate_loot:
    store loot_types as ["gold" and "weapon" and "armor" and "potion"]
    store loot_type as random_from of loot_types
    
    check if loot_type is equal to "gold":
        store amount as random_int of 10 and 100
        return "Found " with amount with " gold coins!"
    check if loot_type is equal to "weapon":
        store weapons as ["sword" and "bow" and "staff"]
        store weapon as random_from of weapons
        return "Found a " with weapon with "!"
    otherwise:
        return "Found a " with loot_type with "!"
    end check
end

// Random enemy spawn
action spawn_enemy:
    store enemy_types as ["goblin" and "orc" and "skeleton"]
    store enemy as random_from of enemy_types
    store health as random_int of 50 and 150
    store x as random_between of -100 and 100
    store y as random_between of -100 and 100
    
    display "Spawned " with enemy with " with " with health with " HP at (" with x with ", " with y with ")"
end
```

### Simulation and Testing

```wfl
// Monte Carlo simulation
action estimate_pi with samples:
    store inside_circle as 0
    
    count from 1 to samples:
        store x as random_between of -1 and 1
        store y as random_between of -1 and 1
        store distance_squared as (x * x) + (y * y)
        
        check if distance_squared is less than or equal to 1:
            change inside_circle to inside_circle plus 1
        end check
    end count
    
    store pi_estimate as 4 * (inside_circle / samples)
    return pi_estimate
end

// Random data generation for testing
action generate_test_user:
    store first_names as ["John" and "Jane" and "Bob" and "Alice"]
    store last_names as ["Smith" and "Johnson" and "Brown" and "Davis"]
    
    store first_name as random_from of first_names
    store last_name as random_from of last_names
    store age as random_int of 18 and 80
    store is_premium as random_boolean
    
    return [first_name, last_name, age, is_premium]
end
```

## Security Considerations

### Cryptographic Security
- All random functions use cryptographically secure random number generators
- Suitable for generating passwords, tokens, and security-sensitive values
- Properly seeded from system entropy sources

### Best Practices
1. **Don't use predictable seeds** for security-sensitive applications
2. **Use time-based seeding** for unpredictable results
3. **Use fixed seeds** only for testing and reproducible simulations
4. **Avoid patterns** in seed selection

```wfl
// Good: Unpredictable seeding
store current_time as timestamp of now
random_seed of current_time

// Bad: Predictable seeding (for security applications)
random_seed of 12345  // Only use for testing/simulations
```

## Performance Notes

- All functions are implemented in Rust for optimal performance
- Random number generation is thread-safe within WFL's single-threaded execution
- Seeding operation is more expensive than generation, so seed once and generate many
- Large list selection with `random_from` is O(1) - very efficient

## Error Handling

The Random module handles errors gracefully:

- **Empty list**: `random_from` with empty list throws runtime error
- **Invalid ranges**: `random_between` and `random_int` with min > max throws error
- **Type errors**: All functions validate parameter types

```wfl
// Error handling example
action safe_random_from with list:
    check if length of list is 0:
        return nothing
    end check
    return random_from of list
end
```

## Migration from Old Random

If you're upgrading from the old time-based `random()` function:

**Old code still works:**
```wfl
store r as random  // Still works, now cryptographically secure
```

**New capabilities:**
```wfl
// Much more powerful and secure
store dice as random_int of 1 and 6
store color as random_from of ["red" and "green" and "blue"]
store coin as random_boolean
```

## See Also

- [Math Module](math-module.md) - Mathematical functions and calculations
- [Core Module](core-module.md) - Basic utilities and type checking
- [List Module](list-module.md) - Collection operations
- [WFL Language Reference](../language-reference/wfl-spec.md) - Complete language specification
