# Async Programming

WFL supports asynchronous operations using natural language syntax. The `wait for` keyword lets a slow operation yield cooperatively — while it waits on I/O, the WFL runtime can make progress on other awaited work instead of the thread sitting idle.

## What is Async?

**Synchronous (blocking):** Operations run one at a time, and while one runs the thread can do nothing else.

**Asynchronous (cooperative):** An awaited operation *yields* while it waits on I/O, so the runtime can drive other awaited work in the meantime.

> **Concurrent, not parallel.** WFL's async today is cooperative and single-threaded: awaited work is *interleaved* on one thread, not run on multiple cores at once. In a plain script, statements — including `wait for` statements — still execute one after another. The payoff of `wait for` is that a waiting operation releases the thread to the runtime rather than hard-blocking it. Running independent operations so they actually overlap is a planned feature (see [Concurrent Async](#concurrent-async-future-feature) below).

## The `wait for` Keyword

WFL uses `wait for` for async operations:

```wfl
// Wait for an async write to finish
open file at "notes.txt" for writing as notes_file
wait for write content "data" into notes_file
close file notes_file

// Wait for an async read to finish
open file at "notes.txt" for reading as notes_reader
wait for store file_content as read content from notes_reader
close file notes_reader
```

**Syntax:**
```wfl
wait for <async operation>
```

This tells WFL: "This operation might take time, handle it asynchronously."

## Why Async Matters

### Without Async (Blocking)

```wfl
// Prepare two sample files
open file at "file1.txt" for writing as setup1
wait for write content "first file" into setup1
close file setup1
open file at "file2.txt" for writing as setup2
wait for write content "second file" into setup2
close file setup2

// Each operation waits for the previous one
open file at "file1.txt" for reading as file1
store content1 as read content from file1  // Blocks here
close file file1

open file at "file2.txt" for reading as file2
store content2 as read content from file2  // Blocks here too
close file file2

// Total time: Time1 + Time2
```

### With Async (Cooperative)

```wfl
// Prepare two sample files
open file at "file1.txt" for writing as setup1
wait for write content "first file" into setup1
close file setup1
open file at "file2.txt" for writing as setup2
wait for write content "second file" into setup2
close file setup2

// Each `wait for` still completes before the next statement runs — these do
// not overlap. What `wait for` changes is that while an operation waits on
// I/O, the thread yields to the runtime instead of hard-blocking, so other
// runtime work (such as a web server's transport layer) keeps making progress.
open file at "file1.txt" for reading as file1
wait for store content1 as read content from file1  // Yields while waiting
close file file1

open file at "file2.txt" for reading as file2
wait for store content2 as read content from file2  // Runs after the first
close file file2

// Total time today: Time1 + Time2. Overlapping independent operations is a
// planned feature (see "Concurrent Async" below) — it is not available yet.
```

## Common Async Operations

### File I/O

```wfl
// Async file write
open file at "output.txt" for writing as out_file
wait for write content "async data" into out_file
close file out_file
display "File write complete"

// Async file read
open file at "output.txt" for reading as in_file
wait for store file_content as read content from in_file
close file in_file
display "File read complete"
```

### Web Requests

```wfl
listen on port 8080 as web_server

// Wait for the next request. The transport layer accepts connections
// concurrently, but your handler code below runs one request at a time.
wait for request comes in on web_server as incoming

respond to incoming with "Response" and content_type "text/plain"
```

### Directory Listing

```wfl
wait for store files as list files in "."
display "File listing complete"
```

## Using Async Results

Store results from async operations:

```wfl
// Prepare a data file
open file at "data.txt" for writing as writer
wait for write content "important data" into writer
close file writer

// Read file asynchronously
open file at "data.txt" for reading as reader
wait for store file_content as read content from reader
close file reader

// Use the result
display "Content: " with file_content
```

## Error Handling with Async

Always use try-catch with async operations:

```wfl
store file_handle as nothing
try:
    open file at "data.txt" for reading as data_file
    change file_handle to data_file
    wait for store file_content as read content from data_file
    display "Success: " with file_content
when error:
    display "Error reading file: " with error_message
end try

// Cleanup runs even if the read failed. A variable defined inside `try`
// is not visible after `end try`, so we track the handle in an outer variable.
check if file_handle is not nothing:
    close file file_handle
end check
```

## Async in Web Servers

Web servers naturally use async operations. Note that request *handlers* run one
at a time today — the transport layer (accepting connections, TLS handshakes) is
concurrent, but your handler code is serial. See
[Web Servers → Limitations](web-servers.md#limitations--notes) for details.

```wfl
listen on port 8081 as web_server

// This waits asynchronously for requests
wait for request comes in on web_server as incoming

// Handle request (potentially with more async operations)
try:
    open file at "data.txt" for reading as data_file
    wait for store file_content as read content from data_file
    close file data_file
    respond to incoming with file_content and content_type "text/plain"
catch:
    respond to incoming with "Error" and status 500 and content_type "text/plain"
end try
```

## Multiple Async Operations

### Sequential Async

Operations run one after another:

```wfl
// Operation 1
open file at "step1.txt" for writing as step_file
wait for write content "first result" into step_file
close file step_file

// Operation 2 (waits for 1 to finish)
open file at "step1.txt" for reading as step_read
wait for store result1 as read content from step_read
close file step_read

// Operation 3 (waits for 2 to finish)
wait for store result2 as list files in "."

display "All operations complete"
```

### Concurrent Async (Future Feature)

Planned syntax for running independent operations concurrently (so they actually
overlap instead of running one after another):

```wfl
// This is planned for future versions
wait for all operations complete as results
// Multiple operations run concurrently
```

## Common Patterns

### Async File Processing

```wfl
define action called handle_file with parameters filename:
    // Track handles in outer variables so cleanup can run after `end try`
    // (a variable defined inside `try` is not visible after it).
    store in_handle as nothing
    store out_handle as nothing
    store result as no
    try:
        open file at filename for reading as in_file
        change in_handle to in_file
        wait for store file_content as read content from in_file

        // Process content
        store processed as touppercase of file_content

        store output_name as filename with ".processed"
        open file at output_name for writing as out_file
        change out_handle to out_file
        wait for write content processed into out_file

        display "Processed: " with filename
        change result to yes
    when error:
        display "Error processing: " with filename with ": " with error_message
        change result to no
    end try

    // Always close whatever we managed to open, even on failure
    check if in_handle is not nothing:
        close file in_handle
    end check
    check if out_handle is not nothing:
        close file out_handle
    end check
    return result
end action

// Prepare an input directory with a sample file
makedirs "input"
open file at "input/sample.txt" for writing as seed_file
wait for write content "hello" into seed_file
close file seed_file

wait for store input_files as list files in "input"
for each name in input_files:
    store file_path as "input/" with name
    call handle_file with file_path
end for
```

### Async Request Handler

```wfl
listen on port 8082 as web_server

wait for request comes in on web_server as incoming

// Async data lookup (here: read the users from a data file)
open file at "users.txt" for reading as data_file
wait for store user_data as read content from data_file
close file data_file

// Build response with the data
store reply as "Users: " with user_data
respond to incoming with reply and content_type "text/plain"
```

## Best Practices

✅ **Use `wait for` with I/O** - File, network, database operations

✅ **Handle errors** - Async operations can fail

✅ **Close resources** - Prefer `finally:` so cleanup always runs

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
✅ **Why async matters** - Cooperative, non-blocking I/O (concurrent, not parallel)
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
