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
open file at "missing.txt" for reading as my_file
store file_content as read content from my_file
// CRASH! Program stops if file doesn't exist
```

**Result:** Your program crashes, user sees a scary error message.

### With Error Handling

```wfl
store file_handle as nothing

try:
    open file at "missing.txt" for reading as my_file
    change file_handle to my_file
    store file_content as read content from my_file
    display "File content: " with file_content
catch:
    display "Could not read file. It might not exist."
end try

check if file_handle is not nothing:
    close file_handle
end check
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
store file_handle as nothing

try:
    open file at "data.txt" for reading as my_file
    change file_handle to my_file
    store file_content as read content from my_file
    display "File content: " with file_content
catch:
    display "Error: Could not read file"
end try

check if file_handle is not nothing:
    close file_handle
end check
```

## Cleanup After the Try Block

WFL has no separate `finally` keyword. To run cleanup code whether or not an
error occurred, place it **right after `end try`**. Once a `catch:` (or a
`when` clause) has handled the error, execution continues past `end try`, so
that code always runs. Track any state you need to inspect during cleanup in a
variable that lives *outside* the try block:

```wfl
store outcome as "unknown"

try:
    display "Doing work"
    store result as 10 divided by 2
    display "Result: " with result
    change outcome to "succeeded"
catch:
    display "Something went wrong"
    change outcome to "failed"
end try

// This cleanup runs whether the work succeeded or failed
display "Cleanup: work " with outcome
```

**Syntax:**
```wfl
try:
    <statements that might fail>
catch:
    <error handling code>
end try
<cleanup code that always runs>
```

**Put cleanup after `end try` for:**
- Closing files
- Releasing resources
- Cleanup operations
- Logging

### Cleanup Example

Store the file handle in an outer variable so you can close it after the try
block finishes:

```wfl
store file_handle as nothing

try:
    open file at "log.txt" for writing as my_file
    change file_handle to my_file
    write content "log entry" into my_file
    display "File written successfully"
catch:
    display "Error: Could not write file"
end try

check if file_handle is not nothing:
    close file_handle
    display "File closed"
end check
```

## Accessing Error Information

Use `error_message` to get error details:

```wfl
try:
    store result as 10 divided by 0
catch:
    display "An error occurred: " with error_message
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
        store result as risky_operation
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
store file_content as "default data"

try:
    display "Processing file..."

    try:
        open file at "data.txt" for reading as my_file
        change file_content to read content from my_file
        close my_file
    catch:
        display "Warning: Could not read data file, using defaults"
    end try

    // Process content
    display "Processing: " with file_content

catch:
    display "Fatal error in processing"
end try
```

## Specific Error Types

WFL supports catching specific error types (if implemented):

```wfl
try:
    open file at "missing.txt" for reading as my_file
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
store file_handle as nothing

try:
    open file at "config.txt" for reading as my_file
    change file_handle to my_file
    change config to read content from my_file
catch:
    display "Config file not found, using defaults"
end try

check if file_handle is not nothing:
    close file_handle
end check

display "Configuration: " with config
```

### Validation

```wfl
define action called safe_divide with parameters a and b:
    try:
        return a divided by b
    catch:
        display "Warning: Division failed, returning 0"
        return 0
    end try
end action

store result1 as safe_divide of 10 and 2   // 5
store result2 as safe_divide of 10 and 0   // 0 (with warning)
```

### Multiple Operations

```wfl
store success_count as 0

try:
    operation1
    change success_count to success_count plus 1
catch:
    display "Operation 1 failed"
end try

try:
    operation2
    change success_count to success_count plus 1
catch:
    display "Operation 2 failed"
end try

display "Successful operations: " with success_count
```

### Resource Cleanup

```wfl
store file_handle as nothing

try:
    open file at "data.txt" for writing as my_file
    change file_handle to my_file
    write content "important data" into my_file
    display "File written successfully"
catch:
    display "Error writing to file"
end try

check if file_handle is not nothing:
    close file_handle
    display "File closed"
end check
```

## Real-World Examples

### Configuration Loader

```wfl
define action called load_config with parameters filename:
    try:
        open file at filename for reading as my_file
        store config_data as read content from my_file
        close my_file
        return config_data
    catch:
        display "Warning: Could not load " with filename
        return "default configuration"
    end try
end action

store app_config as load_config of "app.config"
display "Using config: " with app_config
```

### Safe User Input Parser

```wfl
define action called parse_number with parameters input_text:
    try:
        // Attempt to use the input as a number. WFL does not yet ship a
        // text-to-number builtin, so text input fails the numeric operation
        // and is handled by the catch block below.
        store parsed as input_text times 1
        return parsed
    catch:
        display "Invalid number: " with input_text
        return 0
    end try
end action

store age as parse_number of "25"
store fallback_value as parse_number of "abc"
display "Parsed results: " with age with ", " with fallback_value
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
store file_list as list files in "."

store processed_count as 0
store error_count as 0

for each filename in file_list:
    try:
        open file at filename for reading as my_file
        store file_content as read content from my_file
        close my_file

        // Process content
        display "Processed: " with filename
        change processed_count to processed_count plus 1

    catch:
        display "Error processing: " with filename
        change error_count to error_count plus 1
    end try
end for

display ""
display "Processed: " with processed_count
display "Errors: " with error_count
```

## Otherwise Block

The `otherwise` block is a **fallback for errors that no `when` clause
matched**. It runs only when an error occurred and none of the specific
`when` clauses handled it. It does **not** run when the try block succeeds,
and it does **not** run when a general `catch:` (or `when error:`) has already
handled the error.

```wfl
try:
    open file at "missing.txt" for reading as my_file
when permission denied:
    display "Permission denied"
otherwise:
    display "Some other error was handled"
end try
```

**Output:**
```
Some other error was handled
```

Opening a missing file raises a "file not found" error. The only specific
clause here is `when permission denied`, so the `otherwise` block catches it
instead.

**On success, `otherwise` does not run:**
```wfl
try:
    store result as 10 divided by 2
    display "Result: " with result
when file not found:
    display "File missing"
otherwise:
    display "This only runs on an unmatched error"
end try
```

**Output:**
```
Result: 5
```

**Combined with a general `catch:`, `otherwise` never runs** because `catch:`
already handles every error:
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
when <specific error type>:
    <specific handling>
otherwise:
    <fallback for any unmatched error>
end try
```

## Error Best Practices

### Don't Swallow Errors

**Bad:**
```wfl
try:
    risky_operation
catch:
    // Silent failure - bad!
end try
```

**Good:**
```wfl
try:
    risky_operation
catch:
    display "Error in risky operation"
    // Log the error, inform the user, or take corrective action
end try
```

### Provide Context

**Bad:**
```wfl
try:
    risky_operation
catch:
    display "Error"  // Not helpful
end try
```

**Good:**
```wfl
try:
    risky_operation
catch:
    display "Error loading user profile from database"
    display "Please check your connection and try again"
end try
```

### Use Specific Error Types When Available

**Good:**
```wfl
try:
    open file at "data.txt" for reading as my_file
when file not found:
    display "File doesn't exist, creating it..."
    create file at "data.txt" with ""
when permission denied:
    display "Cannot access file, check permissions"
catch:
    display "Unknown file error"
end try
```

### Clean Up Resources

**Always close files, connections, etc.:**

```wfl
store file_handle as nothing

try:
    open file at "data.txt" for reading as my_file
    change file_handle to my_file
    // ... operations
catch:
    display "Error"
end try

check if file_handle is not nothing:
    close file_handle  // Ensure the file is closed
end check
```

## Common Mistakes

### Forgetting `end try`

**Wrong:**
```wfl
try:
    risky_operation
catch:
    display "Error"
// Missing end try!
```

**Right:**
```wfl
try:
    risky_operation
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
    operation1
catch:
    display "Error in operation1"
end try

try:
    operation2
catch:
    display "Error in operation2"
end try
```

### Not Cleaning Up After the Try Block

**Risky:**
```wfl
try:
    open file at "data.txt" for reading as my_file
    // operations
    close my_file  // What if an error occurs before this line?
catch:
    display "Error"
end try
```

**Safe:**
```wfl
store file_handle as nothing

try:
    open file at "data.txt" for reading as my_file
    change file_handle to my_file
    // operations
catch:
    display "Error"
end try

check if file_handle is not nothing:
    close file_handle  // Always runs, even after an error
end check
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
- Always displays "Operation complete" at the end (place it after `end try`)

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
✅ **Cleanup after try** - Place cleanup code after `end try`
✅ **Nested try-catch** - Error handling at multiple levels
✅ **Specific error types** - `when file not found`, etc.
✅ **Error messages** - `error_message` for error details
✅ **Otherwise block** - Fallback for unmatched error types
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
