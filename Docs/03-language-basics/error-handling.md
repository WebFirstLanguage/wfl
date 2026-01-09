# Error Handling

Programs encounter errors: files don't exist, networks fail, users enter invalid data. WFL's error handling helps you manage these situations gracefully.

## Try-Catch Basics

Use `try` to attempt risky operations and `catch` to handle errors:

```wfl
try:
    display "Attempting risky operation..."
    store result as 10 divided by 0
    display "This won't execute"
catch:
    display "An error occurred!"
end try
```

**Output:**
```
Attempting risky operation...
An error occurred!
```

**Syntax:**
```wfl
try:
    <statements that might fail>
catch:
    <error handling code>
end try
```

## Why Error Handling Matters

### Without Error Handling

```wfl
open file at "missing.txt" for reading as file
store content as read content from file
// CRASH! Program stops if file doesn't exist
```

**Result:** Your program crashes, user sees a scary error message.

### With Error Handling

```wfl
try:
    open file at "missing.txt" for reading as file
    store content as read content from file
    close file
    display "File content: " with content
catch:
    display "Could not read file. It might not exist."
end try
```

**Result:** User sees a friendly message, program continues running.

## Common Error Scenarios

### Division by Zero

```wfl
store x as 10
store y as 0

try:
    store result as x divided by y
    display "Result: " with result
catch:
    display "Error: Cannot divide by zero"
end try
```

### Undefined Variables

```wfl
try:
    display "Value: " with undefined variable
catch:
    display "Error: Variable not defined"
end try
```

### Type Mismatches

```wfl
try:
    store text as "hello"
    store result as text plus 5
catch:
    display "Error: Cannot add text and number"
end try
```

### Array Index Out of Bounds

```wfl
store items as [1 and 2 and 3]

try:
    store item as items[10]
catch:
    display "Error: Index out of bounds"
end try
```

### File Operations

```wfl
try:
    open file at "data.txt" for reading as file
    store content as read content from file
    close file
    display "File content: " with content
catch:
    display "Error: Could not read file"
end try
```

## Try-Catch-Finally

The `finally` block always executes, whether an error occurred or not:

```wfl
try:
    display "Opening file..."
    open file at "data.txt" for reading as file
    store content as read content from file
    display "File read successfully"
catch:
    display "Error reading file"
finally:
    display "Cleaning up..."
    // close file if it was opened
end try
```

**Syntax:**
```wfl
try:
    <statements that might fail>
catch:
    <error handling code>
finally:
    <cleanup code that always runs>
end try
```

**Use `finally` for:**
- Closing files
- Releasing resources
- Cleanup operations
- Logging

### Finally Example

```wfl
store file opened as no

try:
    open file at "data.txt" for reading as file
    change file opened to yes
    store content as read content from file
    display "Content: " with content
catch:
    display "Error: Could not read file"
finally:
    check if file opened is yes:
        close file
        display "File closed"
    end check
end try
```

## Accessing Error Information

Use `error message` to get error details (if supported):

```wfl
try:
    store result as 10 divided by 0
catch:
    display "An error occurred: " with error message
end try
```

**Output:**
```
An error occurred: Division by zero
```

## Nested Try-Catch

You can nest try-catch blocks:

```wfl
try:
    display "Outer try block"

    try:
        display "Inner try block"
        store result as risky operation()
        display "Inner success"
    catch:
        display "Inner catch: handled inner error"
    end try

    display "Back in outer try"
catch:
    display "Outer catch: handled outer error"
end try
```

**When to use:**
- Different error handling for different operations
- Isolating error handling to specific sections
- Complex error scenarios

### Nested Example

```wfl
try:
    display "Processing file..."

    try:
        open file at "data.txt" for reading as file
        store content as read content from file
        close file
    catch:
        display "Warning: Could not read data file, using defaults"
        store content as "default data"
    end try

    // Process content
    display "Processing: " with content

catch:
    display "Fatal error in processing"
end try
```

## Specific Error Types

WFL supports catching specific error types (if implemented):

```wfl
try:
    open file at "missing.txt" for reading as file
when file not found:
    display "File doesn't exist"
when permission denied:
    display "Cannot access file"
catch:
    display "Other error occurred"
end try
```

**Syntax:**
```wfl
try:
    <statements>
when <error type>:
    <specific handling>
when <error type>:
    <specific handling>
catch:
    <general error handling>
end try
```

### Error Types

Common error types (check documentation for complete list):
- `file not found`
- `permission denied`
- `type mismatch`
- `index out of bounds`
- `division by zero`

## Common Error Handling Patterns

### File Reading with Fallback

```wfl
store config as "default config"

try:
    open file at "config.txt" for reading as file
    change config to read content from file
    close file
catch:
    display "Config file not found, using defaults"
end try

display "Configuration: " with config
```

### Validation

```wfl
define action called safe divide with parameters a and b:
    try:
        return a divided by b
    catch:
        display "Warning: Division failed, returning 0"
        return 0
    end try
end action

store result1 as safe divide with 10 and 2   // 5
store result2 as safe divide with 10 and 0   // 0 (with warning)
```

### Multiple Operations

```wfl
store success count as 0

try:
    operation1()
    change success count to success count plus 1
catch:
    display "Operation 1 failed"
end try

try:
    operation2()
    change success count to success count plus 1
catch:
    display "Operation 2 failed"
end try

display "Successful operations: " with success count
```

### Resource Cleanup

```wfl
store file handle as nothing

try:
    open file at "data.txt" for writing as file
    change file handle to file
    write content "important data" into file
    display "File written successfully"
catch:
    display "Error writing to file"
finally:
    check if file handle is not nothing:
        close file handle
        display "File closed"
    end check
end try
```

## Real-World Examples

### Configuration Loader

```wfl
define action called load config with parameters filename:
    try:
        open file at filename for reading as file
        store config data as read content from file
        close file
        return config data
    catch:
        display "Warning: Could not load " with filename
        return "default configuration"
    end try
end action

store app config as load config with "app.config"
display "Using config: " with app config
```

### Safe User Input Parser

```wfl
define action called parse number with parameters text:
    try:
        // Attempt to convert text to number (if supported)
        store number as convert text to number
        return number
    catch:
        display "Invalid number: " with text
        return 0
    end try
end action

store age as parse number with "25"      // 25
store invalid as parse number with "abc"  // 0 (with error message)
```

### Database Query with Retry

```wfl
define action called query database with parameters query:
    store attempts as 0
    store max attempts as 3

    repeat while attempts is less than max attempts:
        try:
            // Attempt database query
            store result as execute query
            return result
        catch:
            change attempts to attempts plus 1
            display "Query failed, attempt " with attempts with "/" with max attempts
            check if attempts is less than max attempts:
                display "Retrying..."
            end check
        end try
    end repeat

    display "All attempts failed"
    return nothing
end action
```

### Batch File Processing

```wfl
list files in "input" as file list

store processed count as 0
store error count as 0

for each filename in file list:
    try:
        open file at filename for reading as file
        store content as read content from file
        close file

        // Process content
        display "Processed: " with filename
        change processed count to processed count plus 1

    catch:
        display "Error processing: " with filename
        change error count to error count plus 1
    end try
end for

display ""
display "Processed: " with processed count
display "Errors: " with error count
```

## Otherwise Block

The `otherwise` block executes when NO error occurred:

```wfl
try:
    store result as 10 divided by 2
    display "Result: " with result
catch:
    display "Error occurred"
otherwise:
    display "Operation completed successfully"
end try
```

**Output:**
```
Result: 5
Operation completed successfully
```

**With error:**
```wfl
try:
    store result as 10 divided by 0
catch:
    display "Error occurred"
otherwise:
    display "This won't execute"
end try
```

**Output:**
```
Error occurred
```

**Syntax:**
```wfl
try:
    <statements>
catch:
    <error handling>
otherwise:
    <success handling>
end try
```

## Error Best Practices

### Don't Swallow Errors

**Bad:**
```wfl
try:
    risky operation()
catch:
    // Silent failure - bad!
end try
```

**Good:**
```wfl
try:
    risky operation()
catch:
    display "Error in risky operation"
    // Log the error, inform the user, or take corrective action
end try
```

### Provide Context

**Bad:**
```wfl
catch:
    display "Error"  // Not helpful
end try
```

**Good:**
```wfl
catch:
    display "Error loading user profile from database"
    display "Please check your connection and try again"
end try
```

### Use Specific Error Types When Available

**Good:**
```wfl
try:
    open file at "data.txt"
when file not found:
    display "File doesn't exist, creating it..."
    create file at "data.txt"
when permission denied:
    display "Cannot access file, check permissions"
catch:
    display "Unknown file error"
end try
```

### Clean Up Resources

**Always close files, connections, etc.:**

```wfl
try:
    open file at "data.txt" for reading as file
    // ... operations
catch:
    display "Error"
finally:
    close file  // Ensure file is closed
end try
```

## Common Mistakes

### Forgetting `end try`

**Wrong:**
```wfl
try:
    risky operation()
catch:
    display "Error"
// Missing end try!
```

**Right:**
```wfl
try:
    risky operation()
catch:
    display "Error"
end try
```

### Catching Too Broadly

**Problematic:**
```wfl
try:
    // 100 lines of code
catch:
    display "Something went wrong somewhere"
    // Where did it fail? Hard to debug!
end try
```

**Better:**
```wfl
try:
    operation1()
catch:
    display "Error in operation1"
end try

try:
    operation2()
catch:
    display "Error in operation2"
end try
```

### Not Using Finally for Cleanup

**Risky:**
```wfl
try:
    open file at "data.txt"
    // operations
    close file  // What if error occurs before this?
catch:
    display "Error"
end try
```

**Safe:**
```wfl
try:
    open file at "data.txt"
    // operations
catch:
    display "Error"
finally:
    close file  // Always closes, even with error
end try
```

## Practice Exercises

### Exercise 1: Safe Division

Create an action called `safe divide with a and b` that:
- Returns the division result if successful
- Returns 0 and displays an error if division by zero

### Exercise 2: File Reader

Create a program that:
- Tries to read "input.txt"
- If successful, displays the content
- If file doesn't exist, displays "File not found"
- Always displays "Operation complete" at the end (use `finally`)

### Exercise 3: List Access

Create a program that:
- Creates a list with 5 items
- Asks for an index (or hardcode it)
- Tries to access that index
- Catches and handles out-of-bounds errors
- Displays the item if successful

### Exercise 4: Multiple Try-Catch

Create a program that attempts 3 operations:
1. Read file "config.txt"
2. Process data (fake operation)
3. Write result to "output.txt"

Each in its own try-catch. Count successes and failures.

### Exercise 5: Nested Error Handling

Create a program with nested try-catch where:
- Outer try handles file opening
- Inner try handles file reading
- Appropriate error messages for each level

## What You've Learned

In this section, you learned:

✅ **Basic try-catch** - `try:` ... `catch:` ... `end try`
✅ **Try-catch-finally** - `finally:` for cleanup code
✅ **Nested try-catch** - Error handling at multiple levels
✅ **Specific error types** - `when file not found`, etc.
✅ **Error messages** - Accessing error information
✅ **Otherwise block** - Code for success scenarios
✅ **Common patterns** - File handling, validation, resource cleanup
✅ **Best practices** - Don't swallow errors, provide context, clean up resources

## Next Steps

Now that you understand error handling:

**[Comments and Documentation →](comments-and-documentation.md)**
Learn how to document your code effectively.

Or explore related topics:
- [Actions (Functions) →](actions-functions.md) - Use error handling in actions
- [File I/O →](../04-advanced-features/file-io.md) - File operations that need error handling
- [Best Practices: Error Handling Patterns →](../06-best-practices/error-handling-patterns.md)

---

**Previous:** [← Lists and Collections](lists-and-collections.md) | **Next:** [Comments and Documentation →](comments-and-documentation.md)
