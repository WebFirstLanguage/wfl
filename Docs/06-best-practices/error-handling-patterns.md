# Error Handling Patterns

Robust applications handle errors gracefully. This guide shows proven patterns for error handling in WFL.

## Golden Rules

1. **Always use try / when / catch for risky operations** (file I/O, network, user input)
2. **Provide context in error messages** (what failed, why, how to fix)
3. **Clean up resources** (prefer a `finally:` clause so cleanup always runs)
4. **Fail fast** (validate early)
5. **Don't swallow errors** (log or display them)

## Pattern 1: File Operations

**Always wrap file operations in try-catch:**

```wfl
define action called safe_read_file with parameters filepath:
    // Keep the handle and result in outer variables so finally can close the file
    // and we can return nothing on failure (callers use isnothing).
    store file_handle as nothing
    store succeeded as no
    store file_content as ""
    try:
        open file at filepath for reading as myfile
        change file_handle to myfile
        change file_content to read content from myfile
        change succeeded to yes
    when error:
        display "Error: Could not read file '" with filepath with "'"
    finally:
        check if file_handle is not nothing:
            close file file_handle
        end check
    end try
    check if succeeded is yes:
        return file_content
    otherwise:
        return nothing
    end check
end action

store config_data as safe_read_file of "config.txt"
check if isnothing of config_data:
    display "Using default configuration"
    change config_data to "default config"
end check
```

## Pattern 2: Resource Cleanup with `finally`

Prefer `finally:` so cleanup runs on both success and error paths. Track handles in outer variables if you open them inside the try body:

```wfl
store file_handle as nothing

try:
    open file at "data.txt" for writing as myfile
    change file_handle to myfile
    wait for write content "important data" into myfile
    display "Write successful"
when error:
    display "Error: Failed to write file"
finally:
    check if file_handle is not nothing:
        close file file_handle
        display "File closed"
    end check
end try
```

This form is clearer than scattering cleanup after `end try`, and it matches the beginner-friendly rule: risky work, handle errors, always clean up.

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
define action called perform_step1:
    display "Running step 1"
end action

define action called perform_step2:
    display "Running step 2"
end action

store step1_success as no
store step2_success as no

try:
    call perform_step1
    change step1_success to yes
when error:
    display "Step 1 failed"
end try

try:
    call perform_step2
    change step2_success to yes
when error:
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
define action called risky_operation:
    // Pretend this may fail on transient errors
    return "operation result"
end action

define action called retry_operation with parameters max_attempts:
    store attempts as 0

    repeat while attempts is less than max_attempts:
        try:
            store result as risky_operation
            store attempt_number as attempts plus 1
            display "Success after " with attempt_number with " attempt(s)"
            return result
        when error:
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

store final_result as retry_operation of 3
display "Result: " with final_result
```

## Pattern 6: Graceful Degradation

**Provide fallbacks when operations fail:**

```wfl
define action called load_config_with_fallback with parameters filename:
    // Default result up front; close the file after end try, then return —
    // so cleanup runs whether the read succeeds or fails.
    store file_handle as nothing
    store config_text as "default configuration"
    try:
        open file at filename for reading as myfile
        change file_handle to myfile
        change config_text to read content from myfile
    when error:
        display "Warning: Could not load config, using defaults"
    end try
    check if file_handle is not nothing:
        close file file_handle
    end check
    return config_text
end action

store app_config as load_config_with_fallback of "app.config"
// Always has a value, even if file doesn't exist
```

## Pattern 7: Specific Error Types

**Handle different errors differently:**

```wfl
store file_handle as nothing

try:
    open file at "data.txt" for reading as myfile
    change file_handle to myfile
    store file_content as read content from myfile
when file not found:
    display "File doesn't exist - creating it"
    create file at "data.txt" with ""
when permission denied:
    display "Cannot access file - check permissions"
when error:
    display "Unknown error occurred"
end try

check if file_handle is not nothing:
    close file file_handle
end check
```

## Pattern 8: Error Accumulation

**Collect multiple errors before failing:**

```wfl
define action called validate_input with parameters username and password and email:
    create list errors:
    end list

    check if length of username is less than 3:
        push with errors and "Username too short"
    end check

    check if length of password is less than 8:
        push with errors and "Password too short"
    end check

    store has_at as contains of email and "@"
    check if has_at is equal to no:
        push with errors and "Invalid email format"
    end check

    check if length of errors is greater than 0:
        display "Validation errors:"
        for each validation_error in errors:
            display "  - " with validation_error
        end for
        return no
    otherwise:
        return yes
    end check
end action

store is_valid as validate_input of "ab" and "short" and "bademail"
display "Valid: " with is_valid
```

## Best Practices

✅ **Always try / when / catch risky operations** - File I/O, network, subprocess
✅ **Use `finally` for cleanup** - Close files and release resources reliably
✅ **Provide helpful error messages** - Include context
✅ **Log errors** - For debugging
✅ **Validate early** - Fail fast on bad input
✅ **Return nothing on error** - Clear signal of failure
✅ **Use specific error types** - When available
✅ **Implement retries** - For transient failures
✅ **Provide fallbacks** - Graceful degradation

❌ **Don't swallow errors** - Always log or display
❌ **Don't leave resources open** - Prefer `finally` for close paths
❌ **Don't give vague errors** - "Error" is not helpful
❌ **Don't assume success** - Always handle failure cases

## What You've Learned

✅ File operation error handling
✅ Resource cleanup with `finally`
✅ Input validation patterns
✅ Multiple operation handling
✅ Retry logic
✅ Graceful degradation
✅ Specific error types
✅ Error accumulation

**Next:** [Security Guidelines →](security-guidelines.md)

---

**Previous:** [← Naming Conventions](naming-conventions.md) | **Next:** [Security Guidelines →](security-guidelines.md)
