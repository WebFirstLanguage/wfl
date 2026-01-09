# Random Module

The Random module provides cryptographically secure random number generation. Perfect for games, simulations, security tokens, and randomized algorithms.

## Security Note

WFL's random functions use **cryptographically secure** random number generation (StdRng with entropy initialization), making them suitable for security-sensitive applications.

## Functions

### random

**Purpose:** Generate a random floating-point number between 0 and 1.

**Signature:**
```wfl
random
```

**Parameters:** None

**Returns:** Number - Random float in range [0.0, 1.0)

**Example:**
```wfl
store r1 as random
display r1                    // Output: 0.7234... (varies)

store r2 as random
display r2                    // Output: 0.3891... (different)

// Generate 5 random numbers
count from 1 to 5:
    display "Random " with count with ": " with random
end count
```

**Use Cases:**
- Probability calculations
- Random percentages
- Base for other random operations

---

### random_between

**Purpose:** Generate random number between min and max (inclusive).

**Signature:**
```wfl
random_between of <min> and <max>
```

**Alternative:**
```wfl
random_between between <min> and <max>
```

**Parameters:**
- `min` (Number): Minimum value
- `max` (Number): Maximum value

**Returns:** Number - Random number in range [min, max]

**Example:**
```wfl
store price as random_between of 10.0 and 100.0
display "Random price: $" with price

store percentage as random_between of 0 and 100
display "Random percentage: " with percentage with "%"
```

**Use Cases:**
- Random prices
- Random coordinates
- Simulation values

---

### random_int

**Purpose:** Generate random integer between min and max (inclusive).

**Signature:**
```wfl
random_int between <min> and <max>
```

**Parameters:**
- `min` (Number): Minimum value
- `max` (Number): Maximum value

**Returns:** Number - Random integer in range [min, max]

**Example:**
```wfl
store dice_roll as random_int between 1 and 6
display "Dice roll: " with dice_roll

store age as random_int between 18 and 65
display "Random age: " with age
```

**Use Cases:**
- Dice rolls
- Game mechanics
- Random selections
- Lottery numbers

**Example: Roll Two Dice**
```wfl
store die1 as random_int between 1 and 6
store die2 as random_int between 1 and 6
store total as die1 plus die2

display "Die 1: " with die1
display "Die 2: " with die2
display "Total: " with total
```

---

### random_boolean

**Purpose:** Generate a random boolean value (true/false).

**Signature:**
```wfl
random_boolean
```

**Parameters:** None

**Returns:** Boolean - Random `yes` or `no`

**Example:**
```wfl
store coin_flip as random_boolean

check if coin_flip is yes:
    display "Heads!"
otherwise:
    display "Tails!"
end check
```

**Use Cases:**
- Coin flips
- Random decisions
- 50/50 chances
- Feature flags (A/B testing)

**Example: Multiple Coin Flips**
```wfl
store heads_count as 0
store tails_count as 0

count from 1 to 100:
    store flip as random_boolean
    check if flip is yes:
        add 1 to heads_count
    otherwise:
        add 1 to tails_count
    end check
end count

display "Heads: " with heads_count
display "Tails: " with tails_count
```

---

### random_from

**Purpose:** Select a random element from a list.

**Signature:**
```wfl
random_from of <list>
```

**Alternative:**
```wfl
random_from <list>
```

**Parameters:**
- `list` (List): List to select from

**Returns:** Any - Random item from the list

**Example:**
```wfl
store colors as ["red", "green", "blue", "yellow"]
store random_color as random_from of colors
display "Random color: " with random_color

store dice_faces as [1, 2, 3, 4, 5, 6]
store roll as random_from of dice_faces
display "Dice roll: " with roll
```

**Use Cases:**
- Random selection from options
- Shuffling (with repeated random_from)
- Random name/item generation

**Example: Random Greeting**
```wfl
create list greetings:
    add "Hello"
    add "Hi"
    add "Hey"
    add "Greetings"
    add "Welcome"
end list

store greeting as random_from of greetings
display greeting with ", welcome to WFL!"
```

---

### random_seed

**Purpose:** Set the random number generator seed for reproducible randomness.

**Signature:**
```wfl
random_seed of <seed>
```

**Alternative:**
```wfl
random_seed <seed>
```

**Parameters:**
- `seed` (Number): Seed value

**Returns:** None

**Example:**
```wfl
// Set seed for reproducibility
random_seed of 12345

store r1 as random_int between 1 and 100
store r2 as random_int between 1 and 100
display r1 with ", " with r2

// Reset to same seed
random_seed of 12345

store r3 as random_int between 1 and 100
store r4 as random_int between 1 and 100
display r3 with ", " with r4

// r1 = r3, r2 = r4 (same sequence!)
```

**Use Cases:**
- Reproducible tests
- Debugging random code
- Procedural generation (games)
- Scientific simulations

---

## Complete Example

```wfl
display "=== Random Module Demo ==="
display ""

// Basic random
display "Random floats (0-1):"
count from 1 to 3:
    display "  " with random
end count
display ""

// Random integers
display "Dice rolls (1-6):"
count from 1 to 5:
    store roll as random_int between 1 and 6
    display "  Roll " with count with ": " with roll
end count
display ""

// Random in range
display "Random temperatures (60-80°F):"
count from 1 to 3:
    store temp as random_between of 60 and 80
    display "  " with round of temp with "°F"
end count
display ""

// Random boolean
display "Coin flips:"
count from 1 to 5:
    store flip as random_boolean
    check if flip is yes:
        display "  Flip " with count with ": Heads"
    otherwise:
        display "  Flip " with count with ": Tails"
    end check
end count
display ""

// Random from list
create list prizes:
    add "Car"
    add "Vacation"
    add "Gift card"
    add "T-shirt"
end list

display "Random prize draw:"
count from 1 to 3:
    store prize as random_from of prizes
    display "  Winner " with count with " gets: " with prize
end count
display ""

display "=== Demo Complete ==="
```

## Common Patterns

### Random Password Generator

```wfl
create list characters:
    add "A" add "B" add "C" add "D" add "E"
    add "F" add "G" add "H" add "I" add "J"
    add "1" add "2" add "3" add "4" add "5"
    add "!" add "@" add "#" add "$" add "%"
end list

store password as ""

count from 1 to 12:
    store char as random_from of characters
    change password to password with char
end count

display "Random password: " with password
```

### Random Color Generator

```wfl
store red as random_int between 0 and 255
store green as random_int between 0 and 255
store blue as random_int between 0 and 255

display "RGB(" with red with ", " with green with ", " with blue with ")"
```

### Weighted Random Selection

```wfl
// 70% common, 20% uncommon, 10% rare
store roll as random_int between 1 and 100

check if roll is less than or equal to 70:
    display "Common item"
check if roll is less than or equal to 90:
    display "Uncommon item"
otherwise:
    display "Rare item!"
end check
```

### Shuffle List (Simple)

```wfl
define action called simple shuffle with parameters list:
    create list shuffled
    end list

    store remaining_count as length of list

    count from 1 to remaining_count:
        store random_item as random_from of list
        push with shuffled and random_item
    end count

    return shuffled
end action

store deck as ["A", "K", "Q", "J"]
store shuffled_deck as simple shuffle with deck
display "Shuffled: " with shuffled_deck
```

## Best Practices

✅ **Use random_int for discrete values:** Dice, selections, indices

✅ **Use random_between for continuous values:** Prices, coordinates

✅ **Use random_boolean for 50/50:** Coin flips, binary choices

✅ **Use random_from for selection:** Choose from a list

✅ **Use random_seed for testing:** Reproducible randomness

❌ **Don't use random for integers:** Use random_int instead

❌ **Don't forget range limits:** random_int is inclusive

❌ **Don't reseed unnecessarily:** Only for reproducibility

## What You've Learned

In this module, you learned:

✅ **random** - Random float 0-1
✅ **random_between** - Random in range
✅ **random_int** - Random integer
✅ **random_boolean** - Random true/false
✅ **random_from** - Random item from list
✅ **random_seed** - Reproducible randomness
✅ **Security** - Cryptographically secure RNG
✅ **Common patterns** - Passwords, colors, shuffling, weighted selection

## Next Steps

**[Crypto Module →](crypto-module.md)**
Cryptographic hashing for security applications.

Or return to:
**[Standard Library Index →](index.md)**

---

**Previous:** [← Time Module](time-module.md) | **Next:** [Crypto Module →](crypto-module.md)
