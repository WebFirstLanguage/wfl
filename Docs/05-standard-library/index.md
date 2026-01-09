# Standard Library

WFL includes a comprehensive standard library with 181+ built-in functions across 11 modules. Everything you need is included.

## What's in the Standard Library?

WFL's standard library provides:

- **[Core Module](core-module.md)** - Essential functions (print, typeof, isnothing)
- **[Math Module](math-module.md)** - Mathematical operations (abs, round, floor, ceil, clamp)
- **[Text Module](text-module.md)** - String manipulation (uppercase, lowercase, substring, etc.)
- **[List Module](list-module.md)** - List operations (length, push, pop, indexof)
- **[Filesystem Module](filesystem-module.md)** - File and directory operations
- **[Time Module](time-module.md)** - Date and time handling
- **[Random Module](random-module.md)** - Random number generation
- **[Crypto Module](crypto-module.md)** - Cryptographic hashing (WFLHASH)
- **[Pattern Module](pattern-module.md)** - Pattern matching utilities
- **[Typechecker Module](typechecker-module.md)** - Type checking utilities

**No external dependencies required!** Everything is built-in.

## Quick Function Finder

### Core Functions
- `display` - Output text
- `typeof` - Get type of value
- `isnothing` - Check for null

### Math Functions
- `abs` - Absolute value
- `round` - Round to nearest integer
- `floor` - Round down
- `ceil` - Round up
- `clamp` - Constrain value between min and max

### Text Functions
- `touppercase` / `tolowercase` - Case conversion
- `length` - String length
- `contains` - Substring check
- `substring` - Extract portion of text
- `trim` - Remove whitespace
- `starts_with` / `ends_with` - Prefix/suffix checking

### List Functions
- `length` - List size
- `push` - Add to list
- `pop` - Remove from list
- `indexof` - Find item position

### File Functions
- `list files in` - Directory listing
- `file exists at` - Check existence
- `file size at` - Get file size
- `path extension of` - Get file extension
- `path basename of` - Get filename
- `copy_file` / `move_file` / `remove_file` - File operations

### Time Functions
- `current time in milliseconds` - Current timestamp
- `current date` - Current date
- `format date` / `format time` - Formatting

### Random Functions
- `random()` - Random float 0-1
- `random_between` - Random in range
- `random_int` - Random integer
- `random_boolean` - Random true/false
- `random_from` - Pick from list

### Crypto Functions
- `wflhash256` - 256-bit hash
- `wflhash512` - 512-bit hash
- `wflhash256_with_salt` - Salted hash
- `wflmac256` - Message authentication code

## Using Standard Library Functions

All functions use natural language syntax:

```wfl
// Math
store absolute as abs of -5
store rounded as round of 3.7

// Text
store upper as touppercase of "hello"
store len as length of "WFL"

// Lists
store items as [1, 2, 3, 4, 5]
store count as length of items
push with items and 6

// Files
store size as file size at "data.txt"
store ext as path extension of "document.pdf"

// Random
store dice_roll as random_int between 1 and 6
store coin_flip as random_boolean

// Crypto
store hash as wflhash256 of "sensitive data"
```

## Module Organization

Functions are organized by purpose:

**Data Operations:** Core, List, Text
**Mathematical:** Math, Random
**I/O Operations:** Filesystem
**Time & Date:** Time
**Security:** Crypto
**Validation:** Pattern, Typechecker

## Function Naming

WFL functions use natural language names:

- `touppercase` instead of `toUpper` or `upper()`
- `indexof` instead of `indexOf` or `find()`
- `file exists at` instead of `exists()` or `file_exists()`

Some functions have aliases:

- `typeof` = `type_of`
- `isnothing` = `is_nothing`
- `touppercase` = `to_uppercase`

Use whichever feels more natural!

## Learning Approach

### For Beginners

Start with these modules:
1. **[Core Module](core-module.md)** - Basic output and type checking
2. **[Math Module](math-module.md)** - Simple calculations
3. **[Text Module](text-module.md)** - String manipulation
4. **[List Module](list-module.md)** - Working with collections

Then explore others as needed.

### For Experienced Developers

Use as reference:
- **[Overview](overview.md)** - Architecture and design
- Jump to module you need
- Use Quick Function Finder above

### For Specific Tasks

**Building web apps?**
→ [Filesystem](filesystem-module.md), [Time](time-module.md), [Crypto](crypto-module.md)

**Data processing?**
→ [Text](text-module.md), [List](list-module.md), [Pattern](pattern-module.md)

**Game development?**
→ [Random](random-module.md), [Math](math-module.md)

**System automation?**
→ [Filesystem](filesystem-module.md), [Time](time-module.md)

## Testing Functions in REPL

The REPL is perfect for exploring the standard library:

```wfl
$ wfl
> abs of -10
10

> touppercase of "hello"
HELLO

> length of "WFL"
3

> random_int between 1 and 6
4

> typeof of 42
Number
```

Try every function interactively!

## Complete Function List

### Core Module (3 functions)
- display, typeof, isnothing

### Math Module (5 functions)
- abs, round, floor, ceil, clamp

### Text Module (8+ functions)
- touppercase, tolowercase, length, contains, substring, trim, starts_with, ends_with

### List Module (5 functions)
- length, push, pop, contains, indexof

### Filesystem Module (20+ functions)
- File operations, directory operations, path operations

### Time Module (14 functions)
- Current time, date formatting, time math

### Random Module (6 functions)
- random, random_between, random_int, random_boolean, random_from, random_seed

### Crypto Module (4 functions)
- wflhash256, wflhash512, wflhash256_with_salt, wflmac256

### Pattern Module (3 functions)
- Pattern creation, matching, extraction

---

## What's Next?

Choose a module to explore:

**Start with basics:**
- **[Core Module →](core-module.md)** - display, typeof, isnothing
- **[Math Module →](math-module.md)** - Mathematical operations

**For text processing:**
- **[Text Module →](text-module.md)** - String manipulation
- **[Pattern Module →](pattern-module.md)** - Pattern matching

**For file operations:**
- **[Filesystem Module →](filesystem-module.md)** - Complete file API

**For time operations:**
- **[Time Module →](time-module.md)** - Dates and times

**For randomness:**
- **[Random Module →](random-module.md)** - Random numbers

**For security:**
- **[Crypto Module →](crypto-module.md)** - Hashing and MACs

---

**Previous:** [← Interoperability](../04-advanced-features/interoperability.md) | **Next:** [Overview →](overview.md)
