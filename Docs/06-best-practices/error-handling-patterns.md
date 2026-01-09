# Error Handling Patterns

Robust applications handle errors gracefully. This guide shows proven patterns for error handling in WFL.

## Golden Rules

1. **Always use try-catch for risky operations** (file I/O, network, user input)
2. **Provide context in error messages** (what failed, why, how to fix)
3. **Clean up resources** (use finally to close files)
4. **Fail fast** (validate early)
5. **Don't swallow errors** (log or display them)

## Pattern 1: File Operations

**Always wrap file operations in try-catch:**

```wfl
define action called safe_read_file with parameters filepath:
    try:
        open file at filepath for reading as myfile
        wait for store content as read content from myfile
        close file myfile
        return content
    catch:
        display "Error: Could not read file '" with filepath with "'"
        return nothing
    end try
end action

store data as safe_read_file with "config.txt"
check if isnothing of data:
    display "Using default configuration"
    store data as "default config"
end check
```

## Pattern 2: Resource Cleanup with Finally

**Always close resources, even when errors occur:**

```wfl
store file_handle as nothing

try:
    open file at "data.txt" for writing as myfile
    change file_handle to myfile
    wait for write content "important data" into myfile
    display "Write successful"
catch:
    display "Error: Failed to write file"
finally:
    check if file_handle is not nothing:
        close file file_handle
        display "File closed"
    end check
end try
```

## Pattern 3: Validation Before Processing

**Validate input early:**

```wfl
define action called process_user_age with parameters age:
    // Validate first
    check if typeof of age is not "Number":
        display "Error: Age must be a number"
        return nothing
    end check

    check if age is less than 0 or age is greater than 120:
        display "Error: Age must be between 0 and 120"
        return nothing
    end check

    // Now safe to process
    return age
end action
```

## Pattern 4: Multiple Operations

**Separate try-catch for each operation:**

```wfl
store step1_success as no
store step2_success as no

try:
    perform_step1()
    change step1_success to yes
catch:
    display "Step 1 failed"
end try

try:
    perform_step2()
    change step2_success to yes
catch:
    display "Step 2 failed"
end try

check if step1_success is yes and step2_success is yes:
    display "All steps completed"
otherwise:
    display "Some steps failed"
end check
```

## Pattern 5: Retry Logic

**Retry transient failures:**

```wfl
define action called retry_operation with parameters max_attempts:
    store attempts as 0

    repeat while attempts is less than max_attempts:
        try:
            store result as risky_operation()
            display "Success after " with attempts plus 1 with " attempt(s)"
            return result
        catch:
            add 1 to attempts
            display "Attempt " with attempts with " failed"
            check if attempts is less than max_attempts:
                display "Retrying..."
                wait for 1000 milliseconds
            end check
        end try
    end repeat

    display "All attempts failed"
    return nothing
end action
```

## Pattern 6: Graceful Degradation

**Provide fallbacks when operations fail:**

```wfl
define action called load_config_with_fallback with parameters filename:
    try:
        open file at filename for reading as myfile
        wait for store config as read content from myfile
        close file myfile
        return config
    catch:
        display "Warning: Could not load config, using defaults"
        return "default configuration"
    end try
end action

store app_config as load_config_with_fallback with "app.config"
// Always has a value, even if file doesn't exist
```

## Pattern 7: Specific Error Types

**Handle different errors differently:**

```wfl
try:
    open file at "data.txt" for reading as myfile
    wait for store content as read content from myfile
    close file myfile
when file not found:
    display "File doesn't exist - creating it"
    create file at "data.txt"
when permission denied:
    display "Cannot access file - check permissions"
catch:
    display "Unknown error occurred"
end try
```

## Pattern 8: Error Accumulation

**Collect multiple errors before failing:**

```wfl
create list errors
end list

check if length of username is less than 3:
    push with errors and "Username too short"
end check

check if length of password is less than 8:
    push with errors and "Password too short"
end check

check if not contains "@" in email:
    push with errors and "Invalid email format"
end check

check if length of errors is greater than 0:
    display "Validation errors:"
    for each error in errors:
        display "  - " with error
    end for
    return no
otherwise:
    return yes
end check
```

## Best Practices

✅ **Always try-catch risky operations** - File I/O, network, subprocess
✅ **Use finally for cleanup** - Ensure resources are freed
✅ **Provide helpful error messages** - Include context
✅ **Log errors** - For debugging
✅ **Validate early** - Fail fast on bad input
✅ **Return nothing on error** - Clear signal of failure
✅ **Use specific error types** - When available
✅ **Implement retries** - For transient failures
✅ **Provide fallbacks** - Graceful degradation

❌ **Don't swallow errors** - Always log or display
❌ **Don't leave resources open** - Use finally
❌ **Don't give vague errors** - "Error" is not helpful
❌ **Don't assume success** - Always handle failure cases

## What You've Learned

✅ File operation error handling
✅ Resource cleanup with finally
✅ Input validation patterns
✅ Multiple operation handling
✅ Retry logic
✅ Graceful degradation
✅ Specific error types
✅ Error accumulation

**Next:** [Security Guidelines →](security-guidelines.md)

---

**Previous:** [← Naming Conventions](naming-conventions.md) | **Next:** [Security Guidelines →](security-guidelines.md)
