# Standard Library Overview

WFL's standard library provides 181+ built-in functions organized into 11 modules. Everything you need is already included—no package managers, no external dependencies.

## Library Architecture

The standard library is organized by functionality:

```
Standard Library (181+ functions)
├── Core Module (3 functions)
│   └── Essential operations
├── Math Module (5 functions)
│   └── Mathematical operations
├── Text Module (8 functions)
│   └── String manipulation
├── List Module (5 functions)
│   └── List/array operations
├── Filesystem Module (20+ functions)
│   └── File and directory operations
├── Time Module (14 functions)
│   └── Date and time handling
├── Random Module (6 functions)
│   └── Random number generation
├── Crypto Module (4 functions)
│   └── Cryptographic hashing
├── Pattern Module (3 functions)
│   └── Pattern matching utilities
└── Typechecker Module
    └── Type checking utilities
```

## Design Philosophy

### Natural Language Names

Functions use descriptive, natural language names:

```wfl
// Traditional languages:
Math.abs(-5)
str.toUpperCase()
arr.indexOf(3)

// WFL:
abs of -5
touppercase of "hello"
indexof of list and 3
```

**Principle:** If you can say it in English, you can write it in WFL.

### Consistent Syntax

All functions follow consistent patterns:

**Single argument:**
```wfl
<function> of <value>
```

Examples:
```wfl
abs of -5
typeof of variable
touppercase of "text"
```

**Multiple arguments:**
```wfl
<function> of <arg1> and <arg2> [and <arg3>]
```

Examples:
```wfl
contains of "hello world" and "world"
substring of "hello" from 0 length 2
clamp of 15 between 0 and 10
```

**Special contexts:**
```wfl
file exists at "path"
length of list
random_int between 1 and 6
```

### Type Safety

Functions validate argument types:

```wfl
abs of "hello"  // ERROR: Expected Number, got Text
round of 3.7    // OK: Returns 4
```

Clear error messages help you fix problems quickly.

### No Imports Needed

All standard library functions are available automatically:

```wfl
// No need for:
// import math
// require('fs')
// using System;

// Just use functions:
display "Hello!"
store result as abs of -10
store upper as touppercase of "text"
```

## Function Categories

### Output & Debugging (Core)
- `display` - Output to console
- `typeof` - Get type information
- `isnothing` - Check for null

### Mathematics (Math)
- Basic: `abs`, `round`, `floor`, `ceil`
- Constraints: `clamp`

### String Operations (Text)
- Case: `touppercase`, `tolowercase`
- Information: `length`, `contains`, `starts_with`, `ends_with`
- Manipulation: `substring`, `trim`

### Collection Operations (List)
- Information: `length`, `contains`, `indexof`
- Modification: `push`, `pop`

### File System (Filesystem)
- Files: open, read, write, close
- Directories: list, create, check existence
- Paths: extension, basename, dirname, join
- Information: exists, size, type

### Temporal (Time)
- Current: `current time`, `current date`, `datetime_now`
- Formatting: `format_date`, `format_time`
- Math: `add_days`, `days_between`
- Creation: `create_date`, `create_time`

### Randomness (Random)
- Generation: `random`, `random_int`, `random_boolean`
- Ranges: `random_between`
- Selection: `random_from`
- Seeding: `random_seed`

### Security (Crypto)
- Hashing: `wflhash256`, `wflhash512`
- Salting: `wflhash256_with_salt`
- MAC: `wflmac256`

### Validation (Pattern)
- Matching: pattern matching, finding, replacing

## Using Functions

### Basic Usage

```wfl
// Call function with 'of'
store result as function of argument

// Examples:
store absolute as abs of -10
store uppercase as touppercase of "hello"
store type as typeof of value
```

### Multiple Arguments

```wfl
// Use 'and' to separate arguments
store result as function of arg1 and arg2

// Examples:
store has_world as contains of "hello world" and "world"
store sub as substring of "hello" from 0 length 2
store clamped as clamp of 15 between 0 and 10
```

### In Expressions

```wfl
// Use functions anywhere you'd use a value
display "Absolute: " with abs of -5

check if length of name is greater than 3:
    display "Name is long enough"
end check

count from 1 to random_int between 5 and 10:
    display count
end count
```

## Function Return Values

Functions return appropriate types:

```wfl
abs of -5              // Returns: Number (5)
typeof of 42           // Returns: Text ("Number")
isnothing of value     // Returns: Boolean (yes/no)
touppercase of "hi"    // Returns: Text ("HI")
length of [1, 2, 3]    // Returns: Number (3)
random_boolean         // Returns: Boolean (yes/no)
```

## Error Handling

Functions can fail. Always handle errors for risky operations:

```wfl
try:
    store size as file size at "missing.txt"
    display "Size: " with size
catch:
    display "File not found"
end try
```

**Functions that can fail:**
- File operations (file might not exist)
- Path operations (invalid paths)
- List operations (pop from empty list)

**Functions that won't fail:**
- Math operations (except divide by zero)
- Text operations (work on any text)
- Type checking (works on any value)

## Performance Characteristics

### Fast Functions
- Math operations (abs, round, etc.)
- Text case conversion
- Type checking
- Length calculations

### Moderate Functions
- String searching (contains, substring)
- List operations (push, pop)
- Random number generation

### Slow Functions
- File operations (I/O bound)
- Directory listing (many files)
- Cryptographic hashing (intentionally slow)

**Tip:** Cache results from slow functions when possible.

## Aliases and Alternative Names

Some functions have multiple names for convenience:

```wfl
// These are equivalent:
typeof of x
type_of of x

// These are equivalent:
isnothing of value
is_nothing of value

// These are equivalent:
touppercase of text
to_uppercase of text
```

Use whichever reads most naturally in your code!

## What You've Learned

In this overview, you learned:

✅ **Library organization** - 11 modules by functionality
✅ **Function count** - 181+ built-in functions
✅ **Naming conventions** - Natural language names
✅ **Syntax patterns** - Consistent `of` and `and` usage
✅ **No imports** - Everything available by default
✅ **Type safety** - Functions validate arguments
✅ **Error handling** - Some functions can fail
✅ **Performance** - Fast to slow operations

## Next Steps

Explore individual modules:

**Start with essentials:**
- **[Core Module →](core-module.md)** - display, typeof, isnothing

**For calculations:**
- **[Math Module →](math-module.md)** - Arithmetic functions

**For text:**
- **[Text Module →](text-module.md)** - String operations

**For collections:**
- **[List Module →](list-module.md)** - List functions

**Or browse all modules from the [Standard Library Index](index.md)**

---

**Previous:** [← Standard Library](index.md) | **Next:** [Core Module →](core-module.md)
