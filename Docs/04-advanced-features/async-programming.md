# Async Programming

WFL supports asynchronous operations using natural language syntax. Handle multiple operations concurrently without blocking.

## What is Async?

**Synchronous (blocking):** Operations run one at a time. If one is slow, everything waits.

**Asynchronous (non-blocking):** Operations can run concurrently. Slow operations don't block others.

## The `wait for` Keyword

WFL uses `wait for` for async operations:

```wfl
wait for file operation completes
wait for store content as read content from file
wait for write content "data" into file
```

**Syntax:**
```wfl
wait for <async operation>
```

This tells WFL: "This operation might take time, handle it asynchronously."

## Why Async Matters

### Without Async (Blocking)

```wfl
// Each operation waits for the previous one
open file at "file1.txt" for reading as file1
store content1 as read content from file1  // Blocks here
close file file1

open file at "file2.txt" for reading as file2
store content2 as read content from file2  // Blocks here too
close file file2

// Total time: Time1 + Time2
```

### With Async (Non-Blocking)

```wfl
// Operations can overlap
open file at "file1.txt" for reading as file1
wait for store content1 as read content from file1  // Doesn't block

open file at "file2.txt" for reading as file2
wait for store content2 as read content from file2  // Can run concurrently

// Total time: ~max(Time1, Time2)
```

## Common Async Operations

### File I/O

```wfl
// Async file read
open file at "data.txt" for reading as file
wait for store content as read content from file
close file file
display "File read complete"

// Async file write
open file at "output.txt" for writing as file
wait for write content "async data" into file
close file file
display "File write complete"
```

### Web Requests

```wfl
// Async HTTP request
wait for request comes in on server as req
respond to req with "Response"
```

### Directory Listing

```wfl
wait for store files as list files in "."
display "File listing complete"
```

## Using Async Results

Store results from async operations:

```wfl
// Read file asynchronously
open file at "data.txt" for reading as file
wait for store file_content as read content from file
close file file

// Use the result
display "Content: " with file_content
```

## Error Handling with Async

Always use try-catch with async operations:

```wfl
try:
    open file at "data.txt" for reading as file
    wait for store content as read content from file
    close file file
    display "Success: " with content
catch:
    display "Error reading file"
end try
```

## Async in Web Servers

Web servers naturally use async operations:

```wfl
listen on port 8080 as server

// This waits asynchronously for requests
wait for request comes in on server as req

// Handle request (potentially with more async operations)
try:
    open file at "data.txt" for reading as file
    wait for store content as read content from file
    close file file
    respond to req with content
catch:
    respond to req with "Error" and status 500
end try
```

## Multiple Async Operations

### Sequential Async

Operations run one after another:

```wfl
// Operation 1
wait for store result1 as async_operation1()

// Operation 2 (waits for 1 to finish)
wait for store result2 as async_operation2()

// Operation 3 (waits for 2 to finish)
wait for store result3 as async_operation3()

display "All operations complete"
```

### Concurrent Async (Future Feature)

Planned syntax for running operations in parallel:

```wfl
// This is planned for future versions
wait for all operations complete as results
// Multiple operations run concurrently
```

## Common Patterns

### Async File Processing

```wfl
define action called process file with parameters filename:
    try:
        open file at filename for reading as file
        wait for store content as read content from file
        close file file

        // Process content
        store processed as touppercase of content

        store output_name as filename with ".processed"
        open file at output_name for writing as outfile
        wait for write content processed into outfile
        close file outfile

        display "Processed: " with filename
        return yes
    catch:
        display "Error processing: " with filename
        return no
    end try
end action

wait for store files as list files in "input"
for each filename in files:
    call process file with filename
end for
```

### Async Request Handler

```wfl
listen on port 8080 as server

wait for request comes in on server as req

// Async database query (conceptual)
wait for store data as query database with "SELECT * FROM users"

// Build response with data
store response as "Users: " with data
respond to req with response
```

## Best Practices

✅ **Use `wait for` with I/O** - File, network, database operations

✅ **Handle errors** - Async operations can fail

✅ **Close resources** - Use finally blocks

✅ **Log operations** - Track what's happening

❌ **Don't forget `wait for`** - Sync operations may not work as expected

❌ **Don't assume instant completion** - Async takes time

❌ **Don't ignore errors** - Network/file failures are common

## Current Async Support

WFL's async support is built on the Tokio runtime and includes:

✅ **File I/O operations** - Read, write, append
✅ **Web server requests** - Request handling
✅ **Directory operations** - Listing files

**Future planned:**
- 🔄 HTTP client requests
- 🔄 Database queries
- 🔄 Parallel async operations
- 🔄 Async/await in actions
- 🔄 Timeouts

## What You've Learned

In this section, you learned:

✅ **The `wait for` keyword** - Async operation syntax
✅ **Why async matters** - Non-blocking operations
✅ **Common async operations** - File I/O, web requests, directory listing
✅ **Error handling** - Try-catch with async
✅ **Async in web servers** - Request handling
✅ **Best practices** - When to use async

## Next Steps

Apply async programming to:

**[Web Servers →](web-servers.md)**
Build efficient HTTP servers with async request handling.

**[File I/O →](file-io.md)**
Use async file operations for better performance.

**[Containers (OOP) →](containers-oop.md)**
Organize async code with object-oriented programming.

---

**Previous:** [← Pattern Matching](pattern-matching.md) | **Next:** [Containers (OOP) →](containers-oop.md)
