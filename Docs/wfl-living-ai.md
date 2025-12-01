# WFL Living AI Document
## Constantly-Updated Cheat Sheet for AI Agents Building WFL Apps

> **🤖 PRIMARY AI REFERENCE**
> This is now the **primary reference document for AI agents** working with WFL.
> The previous 143KB WFL-AI-Reference.md has been retired (December 2025) as it duplicated content available in modular documentation.
>
> **Quick links for AI agents:**
> - **Language syntax:** See sections below and [wfldocs/](wfldocs/)
> - **API functions:** See [api/](api/) for detailed module documentation
> - **Examples:** See [guides/wfl-cookbook.md](guides/wfl-cookbook.md)

This living document serves as a comprehensive, constantly-updated reference for AI agents working with the WebFirst Language (WFL). It summarizes current language features, lists available modules, and provides guidance on composing WFL code using natural language syntax. This document is updated whenever the language or its specifications evolve.

## Table of Contents

1. [WFL Language Syntax Reference](#wfl-language-syntax-reference)
2. [Standard Library Documentation](#standard-library-documentation)
3. [Code Patterns and Best Practices](#code-patterns-and-best-practices)
4. [AI-Specific Guidance](#ai-specific-guidance)
5. [Integration Examples](#integration-examples)
6. [Troubleshooting Guide](#troubleshooting-guide)

---

## WFL Language Syntax Reference

### Core Principles
- **Natural Language Syntax**: Code reads like English sentences
- **Minimal Special Characters**: Uses words instead of symbols (`plus` instead of `+`)
- **Case Insensitive**: Keywords and identifiers are case-insensitive
- **Block Structure**: Uses `end` keywords to close blocks (no braces)

### Variable Declaration and Assignment

```wfl
// Variable declaration - use "store" or "create"
store user_name as "Alice"
store age as 28
store is_active as yes
store balance as 123.45
store nothing_value as nothing

// Variable assignment - use "change X to Y"
change age to 29
change user_name to "Bob"

// Arithmetic updates
add 10 to balance
subtract 5 from balance
multiply balance by 1.1
divide balance by 2
```

### Data Types

```wfl
// Text (strings)
store greeting as "Hello, World!"
store multiline as "Line 1
Line 2"

// Numbers (integers and floats)
store count as 42
store price as 19.99

// Booleans
store is_ready as yes    // or "true"
store is_done as no      // or "false"

// Lists
store numbers as [1, 2, 3, 4, 5]
store mixed as ["hello", 42, yes, nothing]
store empty_list as []

// Nothing (null/undefined)
store empty_value as nothing
```

### Control Flow

#### Conditional Statements
```wfl
// Basic if-then-else
check if age is greater than 18:
    display "Adult"
otherwise:
    display "Minor"
end check

// Multiple conditions
check if temperature is below 0:
    display "Freezing"
otherwise if temperature is below 20:
    display "Cold"
otherwise if temperature is below 30:
    display "Warm"
otherwise:
    display "Hot"
end check

// Logical operators
check if age is greater than 18 and is_active is yes:
    display "Eligible"
end check

check if name is "Alice" or name is "Bob":
    display "Recognized user"
end check
```

#### Loops
```wfl
// Count loop (for loop)
count from i as 1 to 10:
    display "Number: " with i
end count

// Count with step
count from i as 0 to 100 by 10:
    display i
end count

// For-each loop
store fruits as ["apple", "banana", "cherry"]
for each fruit in fruits:
    display "Fruit: " with fruit
end for

// While loop
store counter as 0
repeat while counter is less than 5:
    display "Counter: " with counter
    add 1 to counter
end repeat

// Until loop
repeat until counter is equal to 10:
    add 1 to counter
    display counter
end repeat

// Infinite loop with break
repeat forever:
    store input as read user input
    check if input is "quit":
        break
    end check
    display "You said: " with input
end repeat
```

### Functions (Actions)

```wfl
// Basic function definition
define action say_hello:
    display "Hello, World!"
end action

// Function with parameters
define action greet_user:
    needs:
        name as text
        age as number
    do:
        display "Hello, " with name with "! You are " with age with " years old."
end action

// Function with return value
define action calculate_area:
    needs:
        width as number
        height as number
    gives back:
        area as number
    do:
        store area as width times height
        give back area
end action

// Async function
define async action fetch_data:
    needs:
        url as text
    gives back:
        data as text
    do:
        wait for open url at url and read content as data
        give back data
end action

// Calling functions
perform say_hello
perform greet_user with name as "Alice" and age as 25
store result as perform calculate_area with width as 10 and height as 5
```

### String Operations

```wfl
// String concatenation
store full_name as first_name with " " with last_name
store message as "Hello, " with name with "!"

// String functions
store length as length of text
store upper as to_uppercase of text
store lower as to_lowercase of text
store trimmed as trim of text
store contains_result as contains of text and "substring"
store part as substring of text and 0 and 5
store words as split text by " "
```

### List Operations

```wfl
// Creating lists
store numbers as [1, 2, 3]
create list items:
    add "apple"
    add "banana"
    add "cherry"
end list

// List functions
store count as length of numbers
push of numbers and 4
store last as pop of numbers
store found as contains of numbers and 2
store position as index_of of numbers and 3

// List iteration
for each item in items:
    display item
end for

// List access
store first as numbers[0]
store second as numbers[1]
```

### Error Handling

```wfl
// Basic try-catch
try:
    store result as 10 divided by 0
    display "Result: " with result
catch:
    display "An error occurred"
end try

// Specific error handling
try:
    open file at "data.txt" and read content as data
    display "File content: " with data
when file_error:
    display "Could not read file"
when permission_error:
    display "Permission denied"
otherwise:
    display "Unknown error occurred"
end try
```

### Async Operations

```wfl
// File operations
wait for open file at "config.txt" and read content as config
wait for open file at "output.txt" and write data

// HTTP requests
wait for open url at "https://api.example.com/data" and read content as response

// Web server
listen on port 8080 as server
wait for request comes in on server as request
respond to request with "Hello, World!" and content_type "text/plain"
```

---

## Standard Library Documentation

### Core Module

#### `print(value, ...)`
Outputs values to console with automatic spacing.
```wfl
print "Hello, World!"
print "The answer is" 42
print name age balance  // Multiple values
```

#### `typeof(value)`
Returns the type of a value as text.
```wfl
store type as typeof of 42        // "Number"
store type as typeof of "hello"   // "Text"
store type as typeof of yes       // "Boolean"
```

#### `isnothing(value)` / `is_nothing(value)`
Checks if a value is nothing (null).
```wfl
check if isnothing of result:
    display "No result"
end check
```

### Math Module

#### `abs(number)`
Returns absolute value.
```wfl
store positive as abs of -5  // 5
```

#### `round(number)`, `floor(number)`, `ceil(number)`
Rounding functions.
```wfl
store rounded as round of 3.7   // 4
store down as floor of 3.9      // 3
store up as ceil of 3.1         // 4
```

#### `clamp(value, min, max)`
Constrains value between min and max.
```wfl
store limited as clamp of 150 and 0 and 100  // 100
```

### Random Module (Cryptographically Secure)

#### `random()`
Returns random number between 0 and 1.
```wfl
store chance as random  // 0.0 to 0.999...
```

#### `random_between(min, max)`
Random number in range.
```wfl
store temp as random_between of -10 and 35
```

#### `random_int(min, max)`
Random integer in range.
```wfl
store dice as random_int of 1 and 6
```

#### `random_boolean()`
Random true/false.
```wfl
store coin as random_boolean
```

#### `random_from(list)`
Random element from list.
```wfl
store color as random_from of ["red", "green", "blue"]
```

### Text Module

#### `length(text)`
Returns character count.
```wfl
store char_count as length of "Hello"  // 5
```

#### `to_uppercase(text)` / `touppercase(text)`
Converts to uppercase.
```wfl
store upper as to_uppercase of "hello"  // "HELLO"
```

#### `to_lowercase(text)` / `tolowercase(text)`
Converts to lowercase.
```wfl
store lower as to_lowercase of "HELLO"  // "hello"
```

#### `contains(text, substring)`
Checks if text contains substring.
```wfl
store has_hello as contains of "Hello World" and "Hello"  // yes
```

#### `substring(text, start, length)`
Extracts substring.
```wfl
store part as substring of "Hello World" and 0 and 5  // "Hello"
```

#### `string_split(text, delimiter)`
Splits text into list.
```wfl
store words as string_split of "a,b,c" and ","  // ["a", "b", "c"]
```

### List Module

#### `length(list)`
Returns element count.
```wfl
store count as length of [1, 2, 3]  // 3
```

#### `push(list, item)`
Adds item to end of list.
```wfl
push of numbers and 4
```

#### `pop(list)`
Removes and returns last item.
```wfl
store last as pop of numbers
```

#### `contains(list, item)`
Checks if list contains item.
```wfl
store found as contains of numbers and 5
```

#### `index_of(list, item)`
Returns index of item (-1 if not found).
```wfl
store position as index_of of numbers and 3
```

### Time Module

#### `current_time()`
Returns current timestamp.
```wfl
store now as current time
```

#### `current_time_formatted(format)`
Returns formatted current time.
```wfl
store timestamp as current time formatted as "yyyy-MM-dd HH:mm:ss"
```

#### `wait_duration(milliseconds)`
Pauses execution.
```wfl
wait for 1000 milliseconds  // Wait 1 second
```

### Filesystem Module

#### `list_dir(path)`
Lists directory contents.
```wfl
store files as list_dir of "/path/to/directory"
```

#### `path_join(parts...)`
Joins path components.
```wfl
store full_path as path_join of "/home" and "user" and "file.txt"
```

#### `makedirs(path)`
Creates directory and parents.
```wfl
makedirs of "/path/to/new/directory"
```

### Crypto Module

#### `wflhash256(data)`
Computes WFL hash (256-bit).
```wfl
store hash as wflhash256 of "Hello, World!"
```

#### `wflhash512(data)`
Computes WFL hash (512-bit).
```wfl
store hash as wflhash512 of data
```

---

## Code Patterns and Best Practices

### Variable Naming
```wfl
// Good: Descriptive names
store user_name as "Alice"
store total_price as 99.99
store is_authenticated as yes

// Avoid: Single letters or unclear names
store x as "Alice"        // Too short
store tp as 99.99         // Unclear
store flag as yes         // Generic
```

### Error Handling Patterns
```wfl
// Always handle potential errors
try:
    wait for open file at filename and read content as data
    // Process data
    display "Success: " with length of data with " characters read"
when file_error:
    display "Error: Could not read file " with filename
when permission_error:
    display "Error: Permission denied for " with filename
otherwise:
    display "Error: Unknown error reading " with filename
end try
```

### Async/Await Usage
```wfl
// Use 'wait for' for async operations
define async action process_urls:
    needs:
        urls as list
    do:
        for each url in urls:
            try:
                wait for open url at url and read content as response
                display "Fetched " with length of response with " bytes from " with url
            catch:
                display "Failed to fetch " with url
            end try
        end for
end action
```

### Function Design
```wfl
// Clear parameter and return types
define action calculate_discount:
    needs:
        original_price as number
        discount_percent as number
    gives back:
        final_price as number
    do:
        // Validate inputs
        check if original_price is less than 0:
            give back 0
        end check
        
        check if discount_percent is less than 0 or discount_percent is greater than 100:
            give back original_price
        end check
        
        // Calculate discount
        store discount_amount as original_price times discount_percent divided by 100
        store final_price as original_price minus discount_amount
        give back final_price
end action
```

### List Processing
```wfl
// Filter and transform lists
define action process_numbers:
    needs:
        numbers as list
    gives back:
        result as list
    do:
        store result as []
        
        for each num in numbers:
            // Filter: only positive numbers
            check if num is greater than 0:
                // Transform: square the number
                store squared as num times num
                push of result and squared
            end check
        end for
        
        give back result
end action
```

---

## AI-Specific Guidance

### Writing WFL Code
1. **Always use natural language constructs**: Prefer `store X as Y` over assignment operators
2. **Use descriptive variable names**: Multi-word names with spaces are allowed and encouraged
3. **Handle errors explicitly**: Use try/catch blocks for operations that might fail
4. **Follow TDD principles**: Write tests first, then implementation
5. **Use async/await properly**: Always use `wait for` with async operations

### Using WFL CLI Tools

#### Linting Code
```bash
# Check code style and potential issues
wfl --lint program.wfl

# Example output:
# Warning: Variable 'unused_var' is declared but never used
# Error: Missing 'end check' for conditional statement
```

#### Analyzing Code
```bash
# Perform static analysis
wfl --analyze program.wfl

# Example output:
# Info: Function 'calculate_total' has high complexity
# Warning: Potential null pointer access in line 45
```

#### Auto-fixing Code
```bash
# Show proposed fixes without applying
wfl --fix program.wfl --check

# Apply fixes in-place
wfl --fix program.wfl --in-place

# Show diff of proposed changes
wfl --fix program.wfl --diff
```

#### Debugging Code
```bash
# Run with debug output
wfl --debug program.wfl

# View tokens (lexer output)
wfl --lex program.wfl

# View AST (parser output)
wfl --parse program.wfl
```

### Interpreting Error Messages

WFL provides clear, actionable error messages:

#### Parse Errors
```
error: Expected 'as' after identifier(s), but found IntLiteral(42)
  --> example.wfl:3:14
   |
 3 | store greeting 42
   |              ^ Error occurred here
   |
   = Note: Did you forget to use 'as' before assigning a value? 
          For example: `store greeting as 42`
```

#### Type Errors
```
error: Cannot add number and text - Expected Number but found Text
  --> example.wfl:3:12
   |
 3 | display x plus y
   |            ^ Type error occurred here
   |
   = Note: Try converting the text to a number using 'convert to number'
```

#### Runtime Errors
```
error: Division by zero
  --> example.wfl:7:14
   |
 7 | display 10 divided by x
   |              ^ Runtime error occurred here
   |
   = Note: Check your divisor to ensure it's never zero
```

### TDD Best Practices

1. **Write failing tests first**:
```wfl
// test_calculator.wfl
define action test_addition:
    store result as perform add_numbers with a as 2 and b as 3
    check if result is equal to 5:
        display "✓ Addition test passed"
    otherwise:
        display "✗ Addition test failed: expected 5, got " with result
    end check
end action
```

2. **Run tests to confirm failure**:
```bash
wfl test_calculator.wfl
# Should show test failure initially
```

3. **Implement minimal code to pass**:
```wfl
define action add_numbers:
    needs:
        a as number
        b as number
    gives back:
        sum as number
    do:
        store sum as a plus b
        give back sum
end action
```

4. **Verify tests pass**:
```bash
wfl test_calculator.wfl
# Should show test success
```

---

## Integration Examples

### File I/O Operations
```wfl
// Reading configuration file
define action load_config:
    gives back:
        config as text
    do:
        try:
            wait for open file at "config.json" and read content as config
            display "Configuration loaded successfully"
            give back config
        when file_error:
            display "Warning: Config file not found, using defaults"
            give back "{\"default\": true}"
        end try
end action

// Writing log file
define action write_log:
    needs:
        message as text
    do:
        store timestamp as current time formatted as "yyyy-MM-dd HH:mm:ss"
        store log_entry as "[" with timestamp with "] " with message with "\n"
        
        try:
            wait for open file at "app.log" and write log_entry
        catch:
            display "Warning: Could not write to log file"
        end try
end action
```

### Web Requests
```wfl
// GET request with error handling
define async action fetch_user_data:
    needs:
        user_id as number
    gives back:
        user_data as text
    do:
        store api_url as "https://api.example.com/users/" with user_id
        
        try:
            wait for open url at api_url and read content as response
            display "User data fetched successfully"
            give back response
        when network_error:
            display "Error: Could not connect to API"
            give back "{\"error\": \"network_error\"}"
        when timeout_error:
            display "Error: Request timed out"
            give back "{\"error\": \"timeout\"}"
        otherwise:
            display "Error: Unknown error fetching user data"
            give back "{\"error\": \"unknown\"}"
        end try
end action

// POST request
define async action create_user:
    needs:
        user_data as text
    gives back:
        result as text
    do:
        try:
            wait for open url at "https://api.example.com/users" with method POST and write user_data and read content as result
            display "User created successfully"
            give back result
        catch:
            display "Error: Could not create user"
            give back "{\"error\": \"creation_failed\"}"
        end try
end action
```

### Web Server
```wfl
// Basic web server with routing
define action start_web_server:
    do:
        display "Starting web server on port 8080..."
        
        try:
            listen on port 8080 as web_server
            display "✓ Web server started successfully"
            
            // Main server loop
            repeat forever:
                try:
                    wait for request comes in on web_server as request
                    
                    store method as method of request
                    store path as path of request
                    store client_ip as client_ip of request
                    
                    display "📥 " with method with " " with path with " from " with client_ip
                    
                    // Route handling
                    check if path is equal to "/":
                        respond to request with "Welcome to WFL Web Server!" and content_type "text/plain"
                        
                    otherwise if path is equal to "/api/health":
                        store health_response as "{\"status\": \"healthy\", \"timestamp\": \"" with current time formatted as "yyyy-MM-dd HH:mm:ss" with "\"}"
                        respond to request with health_response and content_type "application/json"
                        
                    otherwise if path starts with "/api/":
                        respond to request with "{\"error\": \"API endpoint not found\"}" and status 404 and content_type "application/json"
                        
                    otherwise:
                        respond to request with "Page not found" and status 404 and content_type "text/plain"
                    end check
                    
                catch:
                    display "Error handling request"
                end try
            end repeat
            
        catch:
            display "Error: Could not start web server"
        end try
end action
```

### Database Operations
```wfl
// Note: Database operations are planned but not yet implemented
// This is an example of future functionality

define async action query_database:
    needs:
        query as text
        parameters as list
    gives back:
        results as list
    do:
        try:
            wait for open database at "sqlite:///app.db" as db
            wait for execute query on db with parameters as results
            close database db
            give back results
        when database_error:
            display "Database error: " with error_message
            give back []
        end try
end action
```

---

## Troubleshooting Guide

### Common Errors and Solutions

#### 1. Syntax Errors

**Error**: `Expected 'as' after identifier`
```wfl
// Wrong
store name "Alice"

// Correct
store name as "Alice"
```

**Error**: `Missing 'end' keyword`
```wfl
// Wrong
check if x is greater than 5:
    display "Greater"

// Correct
check if x is greater than 5:
    display "Greater"
end check
```

#### 2. Type Errors

**Error**: `Cannot add number and text`
```wfl
// Wrong
store result as 5 plus "hello"

// Correct - convert types first
store number_part as convert "5" to number
store result as number_part plus 10
```

**Error**: `Expected list but found text`
```wfl
// Wrong
store text as "hello"
store length as length of text  // This works for text too

// But for list operations:
store items as ["a", "b", "c"]
push of items and "d"  // Correct
```

#### 3. Runtime Errors

**Error**: `Division by zero`
```wfl
// Wrong
store result as 10 divided by 0

// Correct - check before dividing
check if divisor is not equal to 0:
    store result as 10 divided by divisor
otherwise:
    display "Error: Cannot divide by zero"
    store result as 0
end check
```

**Error**: `Variable not defined`
```wfl
// Wrong
display undefined_variable

// Correct - define variables before use
store my_variable as "Hello"
display my_variable
```

#### 4. Async/Await Issues

**Error**: `Async operation not awaited`
```wfl
// Wrong
open file at "data.txt" and read content as data

// Correct
wait for open file at "data.txt" and read content as data
```

**Error**: `Cannot use await in non-async function`
```wfl
// Wrong
define action read_file:
    wait for open file at "data.txt" and read content as data
end action

// Correct
define async action read_file:
    wait for open file at "data.txt" and read content as data
end action
```

#### 5. File and Network Issues

**Error**: `File not found`
```wfl
// Add error handling
try:
    wait for open file at "config.txt" and read content as config
when file_error:
    display "Config file not found, using defaults"
    store config as "{\"default\": true}"
end try
```

**Error**: `Network timeout`
```wfl
// Add timeout and retry logic
define async action fetch_with_retry:
    needs:
        url as text
        max_retries as number
    do:
        store attempts as 0
        
        repeat while attempts is less than max_retries:
            try:
                wait for open url at url and read content as response
                give back response
            when timeout_error:
                add 1 to attempts
                display "Attempt " with attempts with " failed, retrying..."
                wait for 1000 milliseconds
            end try
        end repeat
        
        display "All retry attempts failed"
        give back nothing
end action
```

### Performance Tips

1. **Use appropriate data structures**:
```wfl
// For frequent lookups, consider using contains() efficiently
store valid_users as ["alice", "bob", "charlie"]
check if contains of valid_users and username:
    // Process valid user
end check
```

2. **Minimize file I/O operations**:
```wfl
// Read file once, process in memory
wait for open file at "large_data.txt" and read content as data
store lines as string_split of data and "\n"
for each line in lines:
    // Process each line
end for
```

3. **Use async operations for I/O**:
```wfl
// Good - non-blocking
define async action process_urls:
    for each url in urls:
        wait for open url at url and read content as response
        // Process response
    end for
end action
```

### Debugging Strategies

1. **Use print statements for debugging**:
```wfl
store x as 10
print "Debug: x =", x
store y as x times 2
print "Debug: y =", y
```

2. **Check variable types**:
```wfl
store value as some_function_call
print "Type of value:", typeof of value
print "Value:", value
```

3. **Use try-catch to isolate issues**:
```wfl
try:
    // Problematic code here
    store result as risky_operation
    print "Success:", result
catch:
    print "Error occurred in risky_operation"
end try
```

4. **Test with simple inputs first**:
```wfl
// Test with known good data
store test_data as ["apple", "banana"]
store result as process_list with test_data
print "Test result:", result
```

### Getting Help

1. **Use WFL CLI tools**:
   - `wfl --lint` for style issues
   - `wfl --analyze` for potential problems
   - `wfl --debug` for execution tracing

2. **Check error messages carefully** - WFL provides detailed, actionable error messages

3. **Refer to test programs** in `TestPrograms/` directory for working examples

4. **Follow TDD practices** - write tests to verify expected behavior

### Container System (Object-Oriented Programming)

WFL supports containers (similar to classes) for object-oriented programming:

```wfl
// Define a container
create container Person:
    property name as text
    property age as number default 0
    property email as text

    action greet:
        display "Hello, I am " with this.name with " and I am " with this.age with " years old."
    end action

    action set_age:
        needs:
            new_age as number
        do:
            check if new_age is greater than 0 and new_age is less than 150:
                change this.age to new_age
            otherwise:
                display "Invalid age: " with new_age
            end check
    end action

    action get_info:
        gives back:
            info as text
        do:
            store info as "Name: " with this.name with ", Age: " with this.age with ", Email: " with this.email
            give back info
    end action
end container

// Create and use container instances
store person1 as create Person with name as "Alice" and age as 30 and email as "alice@example.com"
perform greet on person1
perform set_age on person1 with new_age as 31
store info as perform get_info on person1
display info
```

### Container Inheritance
```wfl
// Base container
create container Animal:
    property name as text
    property species as text

    action make_sound:
        display this.name with " makes a sound"
    end action
end container

// Derived container
create container Dog extends Animal:
    property breed as text

    action make_sound:
        display this.name with " barks!"
    end action

    action fetch:
        display this.name with " fetches the ball"
    end action
end container

// Usage
store my_dog as create Dog with name as "Buddy" and species as "Canine" and breed as "Golden Retriever"
perform make_sound on my_dog  // "Buddy barks!"
perform fetch on my_dog       // "Buddy fetches the ball"
```

### Advanced Error Handling Patterns

```wfl
// Custom error types and handling
define action safe_divide:
    needs:
        numerator as number
        denominator as number
    gives back:
        result as number
    do:
        try:
            check if denominator is equal to 0:
                throw error "Division by zero is not allowed"
            end check

            store result as numerator divided by denominator
            give back result

        when math_error:
            display "Mathematical error: " with error_message
            give back 0
        when validation_error:
            display "Validation error: " with error_message
            give back nothing
        otherwise:
            display "Unexpected error: " with error_message
            give back nothing
        end try
end action

// Nested error handling
define async action process_file_safely:
    needs:
        filename as text
    do:
        try:
            // Outer try for file operations
            wait for open file at filename and read content as content

            try:
                // Inner try for data processing
                store lines as string_split of content and "\n"
                store processed as []

                for each line in lines:
                    try:
                        // Process each line safely
                        store cleaned as trim of line
                        check if length of cleaned is greater than 0:
                            push of processed and cleaned
                        end check
                    catch:
                        display "Warning: Could not process line: " with line
                    end try
                end for

                display "Successfully processed " with length of processed with " lines"

            when processing_error:
                display "Error processing file content"
            end try

        when file_error:
            display "Error: Could not read file " with filename
        when permission_error:
            display "Error: Permission denied for " with filename
        end try
end action
```

### Command-Line Argument Handling

WFL automatically provides command-line arguments through built-in variables:

```wfl
// Access command-line arguments
display "Total arguments: " with arg_count
display "All arguments: " with args

// Process flags
check if flag_verbose:
    display "Verbose mode enabled"
    store verbose as yes
otherwise:
    store verbose as no
end check

check if flag_output:
    store output_file as flag_output
    display "Output file: " with output_file
otherwise:
    store output_file as "output.txt"
end check

// Process positional arguments
check if length of positional_args is equal to 0:
    display "Usage: wfl script.wfl [options] file1 file2 ..."
    display "Options:"
    display "  --verbose        Enable verbose output"
    display "  --output FILE    Specify output file"
otherwise:
    for each input_file in positional_args:
        check if verbose:
            display "Processing: " with input_file
        end check

        try:
            wait for open file at input_file and read content as data
            // Process file data
            display "Processed " with length of data with " characters from " with input_file
        catch:
            display "Error: Could not process " with input_file
        end try
    end for
end check
```

### Web Server Advanced Features

```wfl
// Advanced web server with middleware, sessions, and static files
define action start_advanced_server:
    do:
        store server_port as 8080
        store static_dir as "./static"
        store session_timeout as 3600000  // 1 hour in milliseconds
        store sessions as create empty map

        display "Starting advanced web server..."

        try:
            listen on port server_port as web_server
            display "✓ Server started on port " with server_port

            repeat forever:
                try:
                    wait for request comes in on web_server as request

                    // Extract request details
                    store method as method of request
                    store path as path of request
                    store headers as headers of request
                    store client_ip as client_ip of request
                    store body as body of request

                    // Middleware: Request logging
                    store timestamp as current time formatted as "yyyy-MM-dd HH:mm:ss"
                    display "📥 [" with timestamp with "] " with method with " " with path with " from " with client_ip

                    // Middleware: Session handling
                    store session_id as header "Cookie" of request
                    check if session_id and contains of sessions and session_id:
                        store session as get from sessions and session_id
                        display "Existing session: " with session_id
                    otherwise:
                        store session_id as generate_uuid
                        store session as create empty map
                        set in sessions and session_id to session
                        display "New session: " with session_id
                    end check

                    // Route handling
                    check if method is equal to "GET":
                        check if path is equal to "/":
                            // Serve home page
                            try:
                                wait for open file at static_dir with "/index.html" and read content as home_content
                                respond to request with home_content and content_type "text/html" and cookie "session_id=" with session_id
                            catch:
                                respond to request with "Welcome to WFL Server!" and content_type "text/plain"
                            end try

                        otherwise if path starts with "/static/":
                            // Serve static files
                            store file_path as substring of path and 8 and length of path  // Remove "/static/"
                            store full_path as static_dir with "/" with file_path

                            try:
                                wait for open file at full_path and read content as file_content

                                // Determine content type
                                check if file_path ends with ".html":
                                    store content_type as "text/html"
                                otherwise if file_path ends with ".css":
                                    store content_type as "text/css"
                                otherwise if file_path ends with ".js":
                                    store content_type as "application/javascript"
                                otherwise if file_path ends with ".json":
                                    store content_type as "application/json"
                                otherwise:
                                    store content_type as "text/plain"
                                end check

                                respond to request with file_content and content_type content_type

                            catch:
                                respond to request with "File not found" and status 404
                            end try

                        otherwise if path is equal to "/api/session":
                            // Session info endpoint
                            store session_info as "{\"session_id\": \"" with session_id with "\", \"timestamp\": \"" with timestamp with "\"}"
                            respond to request with session_info and content_type "application/json"

                        otherwise:
                            respond to request with "Page not found" and status 404
                        end check

                    otherwise if method is equal to "POST":
                        check if path is equal to "/api/data":
                            // Handle POST data
                            try:
                                display "Received POST data: " with body
                                store response as "{\"status\": \"success\", \"received\": " with length of body with " bytes}"
                                respond to request with response and content_type "application/json"
                            catch:
                                respond to request with "{\"error\": \"Invalid data\"}" and status 400 and content_type "application/json"
                            end try

                        otherwise:
                            respond to request with "{\"error\": \"Endpoint not found\"}" and status 404 and content_type "application/json"
                        end check

                    otherwise:
                        respond to request with "Method not allowed" and status 405
                    end check

                catch:
                    display "Error handling request"
                end try
            end repeat

        catch:
            display "Error: Could not start server"
        end try
end action

// Helper function for UUID generation (simplified)
define action generate_uuid:
    gives back:
        uuid as text
    do:
        store uuid as "session_" with random_int of 100000 and 999999
        give back uuid
end action
```

### Testing Patterns for AI Agents

```wfl
// Comprehensive test suite pattern
define action run_test_suite:
    do:
        store total_tests as 0
        store passed_tests as 0
        store failed_tests as 0

        display "=== Running WFL Test Suite ==="
        display ""

        // Test 1: Basic arithmetic
        add 1 to total_tests
        try:
            store result as 2 plus 3
            check if result is equal to 5:
                add 1 to passed_tests
                display "✓ Test 1: Basic arithmetic - PASSED"
            otherwise:
                add 1 to failed_tests
                display "✗ Test 1: Basic arithmetic - FAILED (expected 5, got " with result with ")"
            end check
        catch:
            add 1 to failed_tests
            display "✗ Test 1: Basic arithmetic - ERROR"
        end try

        // Test 2: String operations
        add 1 to total_tests
        try:
            store text as "Hello, World!"
            store length_result as length of text
            check if length_result is equal to 13:
                add 1 to passed_tests
                display "✓ Test 2: String length - PASSED"
            otherwise:
                add 1 to failed_tests
                display "✗ Test 2: String length - FAILED (expected 13, got " with length_result with ")"
            end check
        catch:
            add 1 to failed_tests
            display "✗ Test 2: String length - ERROR"
        end try

        // Test 3: List operations
        add 1 to total_tests
        try:
            store numbers as [1, 2, 3]
            push of numbers and 4
            store list_length as length of numbers
            check if list_length is equal to 4:
                add 1 to passed_tests
                display "✓ Test 3: List operations - PASSED"
            otherwise:
                add 1 to failed_tests
                display "✗ Test 3: List operations - FAILED (expected 4, got " with list_length with ")"
            end check
        catch:
            add 1 to failed_tests
            display "✗ Test 3: List operations - ERROR"
        end try

        // Test 4: Error handling
        add 1 to total_tests
        try:
            store error_caught as no
            try:
                store bad_result as 10 divided by 0
            catch:
                store error_caught as yes
            end try

            check if error_caught is yes:
                add 1 to passed_tests
                display "✓ Test 4: Error handling - PASSED"
            otherwise:
                add 1 to failed_tests
                display "✗ Test 4: Error handling - FAILED (error not caught)"
            end check
        catch:
            add 1 to failed_tests
            display "✗ Test 4: Error handling - ERROR"
        end try

        // Test 5: Function calls
        add 1 to total_tests
        try:
            store function_result as perform test_helper_function with input as "test"
            check if function_result is equal to "TEST":
                add 1 to passed_tests
                display "✓ Test 5: Function calls - PASSED"
            otherwise:
                add 1 to failed_tests
                display "✗ Test 5: Function calls - FAILED (expected 'TEST', got '" with function_result with "')"
            end check
        catch:
            add 1 to failed_tests
            display "✗ Test 5: Function calls - ERROR"
        end try

        // Summary
        display ""
        display "=== Test Results ==="
        display "Total tests: " with total_tests
        display "Passed: " with passed_tests
        display "Failed: " with failed_tests

        store success_rate as passed_tests divided by total_tests times 100
        display "Success rate: " with round of success_rate with "%"

        check if failed_tests is equal to 0:
            display "🎉 All tests passed!"
        otherwise:
            display "❌ Some tests failed"
        end check
end action

// Helper function for testing
define action test_helper_function:
    needs:
        input as text
    gives back:
        output as text
    do:
        store output as to_uppercase of input
        give back output
end action
```

### Configuration Management

```wfl
// Configuration loading and management
define action load_application_config:
    gives back:
        config as map
    do:
        store config as create empty map

        // Try to load from config file
        try:
            wait for open file at "app.config" and read content as config_text

            // Parse simple key=value format
            store lines as string_split of config_text and "\n"
            for each line in lines:
                store trimmed_line as trim of line

                // Skip empty lines and comments
                check if length of trimmed_line is greater than 0 and not starts with trimmed_line and "#":
                    check if contains of trimmed_line and "=":
                        store parts as string_split of trimmed_line and "="
                        check if length of parts is equal to 2:
                            store key as trim of parts[0]
                            store value as trim of parts[1]
                            set in config and key to value
                        end check
                    end check
                end check
            end for

            display "Configuration loaded from app.config"

        catch:
            display "No config file found, using defaults"
        end try

        // Set defaults for missing values
        check if not contains of config and "server_port":
            set in config and "server_port" to "8080"
        end check

        check if not contains of config and "debug_mode":
            set in config and "debug_mode" to "false"
        end check

        check if not contains of config and "log_level":
            set in config and "log_level" to "info"
        end check

        give back config
end action

// Environment variable support
define action get_env_var:
    needs:
        var_name as text
        default_value as text
    gives back:
        value as text
    do:
        // Note: This is conceptual - actual env var support would need implementation
        try:
            store value as environment variable var_name
            give back value
        catch:
            give back default_value
        end try
end action
```

---

This comprehensive guide provides everything an AI agent needs to write effective WFL code. Use it as a reference for syntax, patterns, best practices, and troubleshooting. The examples are practical and can be adapted for specific use cases while following WFL's natural language philosophy and TDD principles.
