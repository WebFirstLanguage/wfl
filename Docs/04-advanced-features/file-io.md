# File I/O

WFL provides comprehensive file and directory operations with natural language syntax. Read, write, and manage files easily.

## Why File I/O Matters

Most real applications need to:
- Store data persistently
- Read configuration files
- Process input files
- Generate output files
- Log information

WFL makes file operations simple and clear.

## Basic File Operations

### Writing a File

```wfl
open file at "data.txt" for writing as myfile
wait for write content "Hello, WFL!" into myfile
close file myfile

display "File written successfully!"
```

**Syntax:**
```wfl
open file at "<path>" for writing as <variable>
wait for write content "<text>" into <variable>
close file <variable>
```

### Reading a File

```wfl
// Set up a file to read
open file at "data.txt" for writing as writer
wait for write content "Hello, WFL!" into writer
close file writer

// Read it back
open file at "data.txt" for reading as myfile
wait for store file_content as read content from myfile
close file myfile

display "File content: " with file_content
```

**Syntax:**
```wfl
open file at "<path>" for reading as <handle>
wait for store <result> as read content from <handle>
close file <handle>
```

### Appending to a File

```wfl
open file at "log.txt" for appending as logfile
wait for append content "New log entry\n" into logfile
close file logfile
```

**Syntax:**
```wfl
open file at "<path>" for appending as <variable>
wait for append content "<text>" into <variable>
close file <variable>
```

## File Modes

Three modes for opening files:

| Mode | Purpose | If File Exists | If File Missing |
|------|---------|----------------|-----------------|
| `for writing` | Create/overwrite | Overwrites content | Creates new file |
| `for reading` | Read only | Reads content | Error |
| `for appending` | Add to end | Appends to existing | Creates new file |

### Mode Examples

**Writing (Overwrites):**
```wfl
// First write
open file at "data.txt" for writing as first_writer
wait for write content "First content" into first_writer
close file first_writer

// Second write (overwrites!)
open file at "data.txt" for writing as second_writer
wait for write content "New content" into second_writer
close file second_writer

// File now contains only "New content"
```

**Appending (Adds):**
```wfl
// First write
open file at "log.txt" for appending as first_writer
wait for append content "Line 1\n" into first_writer
close file first_writer

// Second write (appends!)
open file at "log.txt" for appending as second_writer
wait for append content "Line 2\n" into second_writer
close file second_writer

// File now contains "Line 1\nLine 2\n"
```

## Complete File Workflow

### Create, Write, Read, Delete

```wfl
display "=== File Operations Demo ==="

// 1. Create and write
open file at "example.txt" for writing as writer
wait for write content "This is WFL\n" into writer
wait for append content "File I/O is easy!\n" into writer
close file writer
display "✓ File created and written"

// 2. Read
open file at "example.txt" for reading as reader
wait for store file_content as read content from reader
close file reader
display "✓ File read:"
display file_content

// 3. Append
open file at "example.txt" for appending as appender
wait for append content "Added more content\n" into appender
close file appender
display "✓ Content appended"

// 4. Read again
open file at "example.txt" for reading as reader2
wait for store updated as read content from reader2
close file reader2
display "✓ Updated content:"
display updated

display "=== Demo Complete ==="
```

## Binary Files

Text reads (`read content`) require valid UTF-8, so they corrupt or reject
non-text files such as fonts, images, PDFs, or compressed archives. For those,
use WFL's **binary** file operations, which preserve every byte exactly.

### Reading Binary Content

```wfl
open file at "logo.png" for reading binary as image
store payload as read binary from image
close file image

display "Read " with length of payload with " bytes"
```

**Syntax:**
```wfl
open file at "<path>" for reading binary as <variable>
store <variable> as read binary from <handle>
close file <handle>
```

Read only the first N bytes (for example, to sniff a file header):

```wfl
open file at "logo.png" for reading binary as image
store head_bytes as read 8 bytes from image
close file image
```

### Writing Binary Content

`write binary` accepts either a binary value (from `read binary`) or a list of
byte numbers (each 0–255):

```wfl
create list signature_bytes:
    add 137
    add 80
    add 78
    add 71
end list

open file at "signature.bin" for writing binary as out_file
write binary signature_bytes into out_file
close file out_file
```

**Syntax:**
```wfl
open file at "<path>" for writing binary as <variable>
write binary <binary-or-byte-list> into <handle>
close file <handle>
```

> **Note:** `data` and `header` are **always-reserved** keywords and can never
> be used as a variable or list name. `bytes` is **contextual** — it works as a
> plain `store` variable but is rejected as a `create list` name. The simplest
> way to avoid all of these is to append a descriptive suffix with an underscore:
> `file_data`, `header_bytes`, `content_bytes`, `signature_bytes`. See
> [`reserved-keywords.md`](../reference/reserved-keywords.md) for the full list
> and the always-reserved vs. contextual distinction.

Text and binary reads are capped at 50 MiB per operation by default. The cap is
enforced as bytes stream in (including for special files without a finite
metadata length), and applies to `read N bytes` before its buffer is allocated.
Set `max_file_read_size` in `.wflcfg` to tune the limit; an oversized read raises
a catchable resource-limit error. Binary byte-list writes retain their existing
50 MiB safety check.

## Directory Operations

### Listing Files

```wfl
wait for store files as list files in "."

display "Files in current directory:"
for each filename in files:
    display "  - " with filename
end for
```

**With pattern:**
```wfl
wait for store wfl_files as list files in "." with pattern "*.wfl"

display "WFL files:"
for each wfl_file in wfl_files:
    display "  - " with wfl_file
end for
```

### Creating Directories

```wfl
create directory at "output"
display "Directory created"
```

### Checking If Directory Exists

```wfl
check if directory exists at "output":
    display "Directory exists"
otherwise:
    create directory at "output"
    display "Directory created"
end check
```

## File Information

### Check If File Exists

```wfl
// Create the file so the check succeeds
open file at "data.txt" for writing as writer
wait for write content "hi" into writer
close file writer

store file_found as file exists at "data.txt"

check if file_found is yes:
    display "File exists"
otherwise:
    display "File not found"
end check
```

### Get File Size

```wfl
// Create the file so it has a measurable size
open file at "data.txt" for writing as writer
wait for write content "Hello, WFL!" into writer
close file writer

store file_bytes as file size of "data.txt"
display "File size: " with file_bytes with " bytes"
```

### Path Operations

```wfl
store filepath as "documents/report.pdf"

store extension as path_extension of filepath
display "Extension: " with extension          // "pdf"

store basename as path_basename of filepath
display "Filename: " with basename             // "report.pdf"

store dirname as path_dirname of filepath
display "Directory: " with dirname             // "documents"
```

## Error Handling

Always use error handling for file operations:

### File Not Found

```wfl
try:
    open file at "missing.txt" for reading as myfile
    wait for store file_content as read content from myfile
    close file myfile
    display file_content
when error:
    display "Error: File not found"
end try
```

### Permission Denied

```wfl
try:
    open file at "/root/protected.txt" for reading as myfile
    wait for store file_content as read content from myfile
    close file myfile
when error:
    display "Error: Cannot access file (permission denied?)"
end try
```

### Disk Full

```wfl
store huge_data as "a very large amount of data..."

try:
    open file at "large_file.dat" for writing as myfile
    // Write large amount of data
    wait for write content huge_data into myfile
    close file myfile
when error:
    display "Error: Could not write file (disk full?)"
end try
```

## Common Patterns

### Configuration File Loader

```wfl
define action called load_config with parameters filename:
    try:
        open file at filename for reading as myfile
        wait for store config_text as read content from myfile
        close file myfile
        return config_text
    when error:
        display "Warning: Config file not found, using defaults"
        return "default configuration"
    end try
end action

store app_config as load_config of "app.config"
display "Configuration: " with app_config
```

### Log Writer

```wfl
define action called log message with parameters message:
    try:
        open file at "app.log" for appending as logfile
        store timestamp as current time in milliseconds
        wait for append content timestamp with ": " with message with "\n" into logfile
        close file logfile
    catch:
        display "Failed to write log"
    end try
end action

call log message with "Application started"
call log message with "Processing data"
call log message with "Application finished"
```

### File Processor

```wfl
// Prepare input and output directories with a sample file
create directory at "input"
create directory at "output"
open file at "input/report.txt" for writing as setupfile
wait for write content "hello from wfl" into setupfile
close file setupfile

// Process all .txt files in a directory
wait for store text_files as list files in "input"

store processed as 0

for each filename in text_files:
    // Track handles in outer variables; close them in finally so a throw
    // during read or write cannot leave a file open.
    store in_handle as nothing
    store out_handle as nothing
    try:
        // Read input file
        open file at "input/" with filename for reading as infile
        change in_handle to infile
        wait for store file_content as read content from infile

        // Process content (example: uppercase)
        store processed_content as touppercase of file_content

        // Write output file
        store outfile_name as "output/" with filename
        open file at outfile_name for writing as outfile
        change out_handle to outfile
        wait for write content processed_content into outfile

        add 1 to processed
        display "Processed: " with filename
    when error:
        display "Error processing: " with filename
    finally:
        check if in_handle is not nothing:
            close file in_handle
        end check
        check if out_handle is not nothing:
            close file out_handle
        end check
    end try
end for

display ""
display "Processed " with processed with " files"
```

### Data Export

```wfl
// Export data to CSV format
create list users:
    add "Alice,28,Developer"
    add "Bob,35,Designer"
    add "Carol,30,Manager"
end list

open file at "users.csv" for writing as csvfile
wait for write content "Name,Age,Role\n" into csvfile

for each user in users:
    wait for append content user with "\n" into csvfile
end for

close file csvfile
display "CSV export complete"
```

### File Backup

```wfl
// Create a file to back up
open file at "important.txt" for writing as setupfile
wait for write content "critical data" into setupfile
close file setupfile

define action called backup_file with parameters source:
    store backup_name as source with ".backup"

    // Keep both handles in outer variables so we can close them after end try,
    // and use a `succeeded` flag so the returns happen after cleanup — returning
    // inside the try would skip the close on the success path.
    store src_handle as nothing
    store dest_handle as nothing
    store succeeded as no
    try:
        open file at source for reading as src
        change src_handle to src
        wait for store file_content as read content from src

        open file at backup_name for writing as dest
        change dest_handle to dest
        wait for write content file_content into dest

        change succeeded to yes
    when error:
        display "Backup failed for " with source
    end try
    check if src_handle is not nothing:
        close file src_handle
    end check
    check if dest_handle is not nothing:
        close file dest_handle
    end check
    check if succeeded is yes:
        display "Backed up " with source with " to " with backup_name
        return yes
    otherwise:
        return no
    end check
end action

call backup_file with "important.txt"
```

## Advanced Operations

### Counting Lines

```wfl
// Create a file with several lines
open file at "code.wfl" for writing as writer
wait for write content "line 1\nline 2\nline 3" into writer
close file writer

open file at "code.wfl" for reading as myfile
wait for store file_content as read content from myfile
close file myfile

store lines as split file_content by "\n"
store line_count as length of lines

display "File has " with line_count with " lines"
```

### Finding Files with Content

```wfl
wait for store all_files as list files in "."

store matching_files as []

for each filename in all_files:
    check if ends_with of filename and ".wfl":
        try:
            open file at filename for reading as myfile
            wait for store file_content as read content from myfile
            close file myfile

            check if file_content contains "display":
                push with matching_files and filename
            end check
        when error:
            // Skip files we can't read
            display "Skipping " with filename
        end try
    end check
end for

display "Files containing 'display':"
for each match in matching_files:
    display "  - " with match
end for
```

### File Statistics

```wfl
wait for store all_files as list files in "."

store total_size as 0
store file_count as 0

for each filename in all_files:
    try:
        store file_bytes as file size of filename
        change total_size to total_size plus file_bytes
        add 1 to file_count
    when error:
        // Skip directories or inaccessible files
        display "Skipping " with filename
    end try
end for

display "Total files: " with file_count
display "Total size: " with total_size with " bytes"
```

## Best Practices

✅ **Always close files** - Use `close file` when done

✅ **Use try-catch** - File operations can fail

✅ **Use `finally` for cleanup** - Close files in a `finally:` clause so handles are released on success and error

✅ **Validate paths** - Check if files exist before opening

✅ **Handle missing files gracefully** - Provide defaults or error messages

✅ **Use appropriate modes** - reading vs writing vs appending

✅ **Flush buffers** - Ensure data is written (use close)

❌ **Don't leave files open** - Always close

❌ **Don't ignore errors** - File operations can fail

❌ **Don't overwrite without checking** - Validate before writing

❌ **Don't hardcode paths** - Use variables for flexibility

## File I/O with Error Handling

**Pattern: try / when error / finally**

Use `finally:` so cleanup runs whether the body succeeded or the error was handled:

```wfl
store file_handle as nothing

try:
    open file at "data.txt" for writing as myfile
    change file_handle to myfile

    wait for write content "Important data" into myfile
    display "File written successfully"
when error:
    display "Error writing to file"
finally:
    check if file_handle is not nothing:
        close file file_handle
        display "File closed"
    end check
end try
```

## Common Mistakes

### Forgetting to Close

**Wrong:**
```wfl
open file at "data.txt" for writing as myfile
wait for write content "data" into myfile
// Forgot to close file!
```

**Right:**
```wfl
open file at "data.txt" for writing as myfile
wait for write content "data" into myfile
close file myfile  // Always close!
```

### Reading Non-Existent Files

**Wrong:**
```wfl
open file at "missing.txt" for reading as myfile
// CRASH if file doesn't exist!
```

**Right:**
```wfl
try:
    open file at "missing.txt" for reading as myfile
    wait for store file_content as read content from myfile
    close file myfile
when error:
    display "File not found"
end try
```

### Writing Without Wait

**Wrong:**
```wfl
open file at "data.txt" for writing as myfile
write content "data" into myfile  // Missing 'wait for'
close file myfile
```

**Right:**
```wfl
open file at "data.txt" for writing as myfile
wait for write content "data" into myfile  // Use 'wait for'
close file myfile
```

## What You've Learned

In this section, you learned:

✅ **Opening files** - `open file at` with modes (reading, writing, appending)
✅ **Writing content** - `wait for write content into`
✅ **Reading content** - `wait for store as read content from`
✅ **Closing files** - `close file`
✅ **Directory operations** - `list files in`, `create directory`
✅ **File information** - `file exists at`, `file size at`
✅ **Path operations** - `path extension of`, `path basename of`
✅ **Error handling** - Try-catch for robust file operations
✅ **Common patterns** - Config loading, logging, processing, backups

## Next Steps

Expand your file handling skills:

**[Pattern Matching →](pattern-matching.md)**
Validate file content and extract data with patterns.

**[Async Programming →](async-programming.md)**
Handle file operations asynchronously.

**[Error Handling →](../03-language-basics/error-handling.md)**
Review error handling best practices.

**[Filesystem Module →](../05-standard-library/filesystem-module.md)**
Complete reference for all file functions.

---

**Previous:** [← Web Servers](web-servers.md) | **Next:** [Pattern Matching →](pattern-matching.md)
