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

## Time Module

### Date and Time Functions

#### `today()`
Returns the current date.
- **Parameters**: None
- **Returns**: Date
- **Example**: `store current_date as today`

#### `now()`
Returns the current time.
- **Parameters**: None
- **Returns**: Time
- **Example**: `store current_time as now`

#### `datetime_now()`
Returns the current date and time.
- **Parameters**: None
- **Returns**: DateTime
- **Example**: `store current_datetime as datetime_now`

#### `format_date(date, format)`
Formats a date according to a format string.
- **Parameters**: Date, Text (format string)
- **Returns**: Text
- **Format Options**: `%Y` (year), `%m` (month), `%d` (day), `%B` (month name)
- **Example**: `store formatted as format_date of today and "%Y-%m-%d"`

#### `format_time(time, format)`
Formats a time according to a format string.
- **Parameters**: Time, Text (format string)
- **Returns**: Text
- **Format Options**: `%H` (24-hour), `%I` (12-hour), `%M` (minutes), `%S` (seconds), `%p` (AM/PM)
- **Example**: `store formatted as format_time of now and "%H:%M:%S"`

#### `format_datetime(datetime, format)`
Formats a datetime according to a format string.
- **Parameters**: DateTime, Text (format string)
- **Returns**: Text
- **Example**: `store formatted as format_datetime of datetime_now and "%Y-%m-%d %H:%M:%S"`

#### `parse_date(text, format)`
Parses a date from a text string.
- **Parameters**: Text (date string), Text (format string)
- **Returns**: Date
- **Example**: `store birthday as parse_date of "1990-12-25" and "%Y-%m-%d"`

#### `parse_time(text, format)`
Parses a time from a text string.
- **Parameters**: Text (time string), Text (format string)
- **Returns**: Time
- **Example**: `store meeting_time as parse_time of "14:30" and "%H:%M"`

#### `create_date(year, month, day)`
Creates a date from year, month, and day values.
- **Parameters**: Number (year), Number (month 1-12), Number (day 1-31)
- **Returns**: Date
- **Example**: `store birthday as create_date of 1990 and 12 and 25`

#### `create_time(hour, minute, [second])`
Creates a time from hour, minute, and optional second values.
- **Parameters**: Number (hour 0-23), Number (minute 0-59), Number (second 0-59, optional)
- **Returns**: Time
- **Example**: `store lunch_time as create_time of 12 and 30`

#### `add_days(date, days)`
Adds a number of days to a date.
- **Parameters**: Date, Number (days to add, can be negative)
- **Returns**: Date
- **Example**: `store tomorrow as add_days of today and 1`

#### `days_between(date1, date2)`
Calculates the number of days between two dates.
- **Parameters**: Date, Date
- **Returns**: Number (positive if date2 is later, negative if earlier)
- **Example**: `store days_until as days_between of today and christmas`

#### `current_date()`
Returns the current date as a formatted string (YYYY-MM-DD).
- **Parameters**: None
- **Returns**: Text
- **Example**: `store date_string as current_date`

## Filesystem Module

### File and Directory Operations

#### `list_dir(path)`
Lists all files and directories in the specified path.
- **Parameters**: Text (directory path)
- **Returns**: List of Text (file/directory names)
- **Example**: `store files as list_dir of "."`

#### `glob(pattern, base_path)`
Finds files matching a glob pattern in the specified directory.
- **Parameters**: Text (glob pattern), Text (base directory path)
- **Returns**: List of Text (matching file paths)
- **Pattern Examples**: `"*.txt"`, `"test_*.wfl"`, `"[abc]*.log"`
- **Example**: `store wfl_files as glob of "*.wfl" and "TestPrograms"`

#### `rglob(pattern, base_path)`
Recursively finds files matching a glob pattern.
- **Parameters**: Text (glob pattern), Text (base directory path)
- **Returns**: List of Text (matching file paths)
- **Example**: `store all_rs_files as rglob of "*.rs" and "src"`

#### `path_join(component1, component2, ...)`
Joins path components into a single path.
- **Parameters**: Text (path components)
- **Returns**: Text (joined path)
- **Example**: `store full_path as path_join of "home" and "user" and "documents"`

#### `path_basename(path)`
Returns the filename portion of a path.
- **Parameters**: Text (file path)
- **Returns**: Text (filename)
- **Example**: `store filename as path_basename of "/home/user/test.txt"  // Returns "test.txt"`

#### `path_dirname(path)`
Returns the directory portion of a path.
- **Parameters**: Text (file path)
- **Returns**: Text (directory path)
- **Example**: `store directory as path_dirname of "/home/user/test.txt"  // Returns "/home/user"`

#### `makedirs(path)`
Creates a directory and all necessary parent directories.
- **Parameters**: Text (directory path)
- **Returns**: Nothing
- **Example**: `makedirs of "data/output/results"`

#### `path_exists(path)`
Checks if a file or directory exists.
- **Parameters**: Text (path)
- **Returns**: Boolean
- **Example**: `check if path_exists of "config.txt":`

#### `is_file(path)`
Checks if a path is a file.
- **Parameters**: Text (path)
- **Returns**: Boolean
- **Example**: `check if is_file of "README.md":`

#### `is_dir(path)`
Checks if a path is a directory.
- **Parameters**: Text (path)
- **Returns**: Boolean
- **Example**: `check if is_dir of "src":`

#### `file_mtime(path)`
Returns the modification time of a file as a timestamp.
- **Parameters**: Text (file path)
- **Returns**: Number (Unix timestamp)
- **Example**: `store last_modified as file_mtime of "data.txt"`

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
- **Database Module**: Database connectivity
- **Web Module**: HTTP requests and web APIs (partially available via async)
- **Crypto Module**: Encryption and hashing
- **JSON Module**: JSON parsing and generation

## Backward Compatibility

The standard library maintains backward compatibility:
- Function signatures remain stable
- Deprecated functions are kept as aliases
- New parameters are added as optional
- Breaking changes are avoided