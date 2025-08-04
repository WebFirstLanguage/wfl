# WFL Standard Library Reference

## Overview

The WFL standard library provides essential built-in functions for common programming tasks. All functions are exposed as conventional function calls (e.g., `length(text)` or `random()`) and are implemented as Rust intrinsics for efficiency.

## Core Module

### Basic Utilities and System Functions

#### `print(value)`
Outputs the given value to the console or standard output.
- **Parameters**: One argument of any type (Text, Number, Boolean, List, etc.)
- **Returns**: Nothing
- **Example**: `print("Hello, World!")`

#### `typeof(value)`
Returns a text string describing the type of the given value.
- **Parameters**: One argument of any type
- **Returns**: Text (e.g., "Number", "Text", "List", "Boolean")
- **Example**: `store type as typeof(42)  // type is "Number"`

#### `isnothing(value)`
Checks if the given value is "nothing" (WFL's null/none equivalent).
- **Parameters**: One argument of any type
- **Returns**: Boolean (yes if the value is nothing, no otherwise)
- **Example**: `check if isnothing(result):`

## Math Module

### Numeric Functions

#### `abs(number)`
Returns the absolute value of a number.
- **Parameters**: Number
- **Returns**: Number
- **Example**: `store positive as abs(-5)  // positive is 5`

#### `round(number)`
Rounds a number to the nearest integer.
- **Parameters**: Number
- **Returns**: Number
- **Example**: `store rounded as round(3.7)  // rounded is 4`

#### `floor(number)`
Rounds a number down to the nearest integer.
- **Parameters**: Number
- **Returns**: Number
- **Example**: `store lower as floor(3.9)  // lower is 3`

#### `ceil(number)`
Rounds a number up to the nearest integer.
- **Parameters**: Number
- **Returns**: Number
- **Example**: `store upper as ceil(3.1)  // upper is 4`

#### `random()`
Returns a random number between 0 and 1.
- **Parameters**: None
- **Returns**: Number
- **Example**: `store chance as random()  // chance is between 0 and 1`

#### `clamp(value, min, max)`
Constrains a value between a minimum and maximum.
- **Parameters**: Number (value), Number (min), Number (max)
- **Returns**: Number
- **Example**: `store limited as clamp(150, 0, 100)  // limited is 100`

## Text Module

### String Manipulation Functions

#### `length(text)`
Returns the length of a text string.
- **Parameters**: Text
- **Returns**: Number
- **Example**: `store size as length("Hello")  // size is 5`

#### `touppercase(text)`
Converts text to uppercase.
- **Parameters**: Text
- **Returns**: Text
- **Example**: `store loud as touppercase("hello")  // loud is "HELLO"`

#### `tolowercase(text)`
Converts text to lowercase.
- **Parameters**: Text
- **Returns**: Text
- **Example**: `store quiet as tolowercase("HELLO")  // quiet is "hello"`

#### `contains(text, search)`
Checks if text contains a substring.
- **Parameters**: Text (to search in), Text (to search for)
- **Returns**: Boolean
- **Example**: `check if contains("Hello World", "World"):`

#### `substring(text, start, end)`
Extracts a portion of text.
- **Parameters**: Text, Number (start index), Number (end index)
- **Returns**: Text
- **Example**: `store part as substring("Hello", 0, 2)  // part is "He"`

## List Module

### Collection Functions

#### `length(list)`
Returns the number of elements in a list.
- **Parameters**: List
- **Returns**: Number
- **Example**: `store count as length([1, 2, 3])  // count is 3`

#### `push(list, item)`
Adds an item to the end of a list.
- **Parameters**: List, Any (item to add)
- **Returns**: Nothing (modifies list in place)
- **Example**: `push(mylist, "new item")`

#### `pop(list)`
Removes and returns the last item from a list.
- **Parameters**: List
- **Returns**: The removed item (or nothing if list is empty)
- **Example**: `store last as pop(mylist)`

#### `contains(list, item)`
Checks if a list contains a specific item.
- **Parameters**: List, Any (item to find)
- **Returns**: Boolean
- **Example**: `check if contains(mylist, "apple"):`

#### `indexof(list, item)`
Finds the index of an item in a list.
- **Parameters**: List, Any (item to find)
- **Returns**: Number (index, or -1 if not found)
- **Example**: `store position as indexof(mylist, "banana")`

## Time Module (Future)

### Date and Time Functions

The following functions are planned for future implementation:

#### `time.now()`
Returns the current timestamp.
- **Returns**: Timestamp

#### `time.sleep(seconds)`
Pauses execution for a specified duration.
- **Parameters**: Number (seconds)
- **Returns**: Nothing

#### `time.format(timestamp, format)`
Formats a timestamp as text.
- **Parameters**: Timestamp, Text (format string)
- **Returns**: Text

## Pattern Module

### Regular Expression Functions

#### `pattern.create(regex)`
Creates a compiled regular expression pattern.
- **Parameters**: Text (regex pattern)
- **Returns**: Pattern object
- **Example**: `store email_pattern as pattern.create("^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$")`

#### `pattern.test(pattern, text)`
Tests if text matches a pattern.
- **Parameters**: Pattern, Text
- **Returns**: Boolean
- **Example**: `check if pattern.test(email_pattern, user_input):`

#### `pattern.find(pattern, text)`
Finds the first match of a pattern in text.
- **Parameters**: Pattern, Text
- **Returns**: Match object or nothing
- **Example**: `store match as pattern.find(pattern, text)`

#### `pattern.find_all(pattern, text)`
Finds all matches of a pattern in text.
- **Parameters**: Pattern, Text
- **Returns**: List of Match objects
- **Example**: `store matches as pattern.find_all(pattern, text)`

#### `pattern.replace(pattern, text, replacement)`
Replaces pattern matches in text.
- **Parameters**: Pattern, Text, Text (replacement)
- **Returns**: Text
- **Example**: `store cleaned as pattern.replace(pattern, text, "")`

## Type System Integration

All standard library functions are integrated with WFL's type checker:
- Functions have defined type signatures
- Type mismatches are caught at compile time
- The type checker enforces correct argument types
- Return types are properly inferred

## Implementation Notes

### Naming Convention
Function names follow WFL's principle of minimizing special characters:
- No underscores in function names
- Clear, descriptive English names
- Consistent naming patterns across modules

### Error Handling
Standard library functions handle errors gracefully:
- Invalid inputs return sensible defaults or nothing
- Clear error messages explain what went wrong
- No crashes or undefined behavior

### Performance
- Functions are implemented in Rust for efficiency
- Operations are optimized for common use cases
- Memory usage is minimized where possible

## Future Expansions

The standard library will be expanded with:
- **I/O Module**: File and network operations
- **Database Module**: Database connectivity
- **Web Module**: HTTP requests and web APIs
- **Crypto Module**: Encryption and hashing
- **JSON Module**: JSON parsing and generation
- **Date Module**: Advanced date/time operations

## Backward Compatibility

The standard library maintains backward compatibility:
- Function signatures remain stable
- Deprecated functions are kept as aliases
- New parameters are added as optional
- Breaking changes are avoided