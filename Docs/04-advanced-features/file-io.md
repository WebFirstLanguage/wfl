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
open file at "data.txt" for reading as myfile
wait for store content as read content from myfile
close file myfile

display "File content: " with content
```

**Syntax:**
```wfl
open file at "<path>" for reading as <variable>
wait for store <variable> as read content from <variable>
close file <variable>
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
open file at "data.txt" for writing as file
wait for write content "First content" into file
close file file

// Second write (overwrites!)
open file at "data.txt" for writing as file
wait for write content "New content" into file
close file file

// File now contains only "New content"
```

**Appending (Adds):**
```wfl
// First write
open file at "log.txt" for appending as file
wait for append content "Line 1\n" into file
close file file

// Second write (appends!)
open file at "log.txt" for appending as file
wait for append content "Line 2\n" into file
close file file

// File now contains "Line 1\nLine 2\n"
```

## Complete File Workflow

### Create, Write, Read, Delete

```wfl
display "=== File Operations Demo ==="

// 1. Create and write
open file at "example.txt" for writing as file
wait for write content "This is WFL\n" into file
wait for append content "File I/O is easy!\n" into file
close file file
display "✓ File created and written"

// 2. Read
open file at "example.txt" for reading as file
wait for store content as read content from file
close file file
display "✓ File read:"
display content

// 3. Append
open file at "example.txt" for appending as file
wait for append content "Added more content\n" into file
close file file
display "✓ Content appended"

// 4. Read again
open file at "example.txt" for reading as file
wait for store updated as read content from file
close file file
display "✓ Updated content:"
display updated

display "=== Demo Complete ==="
```

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
store exists as file exists at "data.txt"

check if exists is yes:
    display "File exists"
otherwise:
    display "File not found"
end check
```

### Get File Size

```wfl
store size as file size at "data.txt"
display "File size: " with size with " bytes"
```

### Path Operations

```wfl
store filepath as "documents/report.pdf"

store extension as path extension of filepath
display "Extension: " with extension          // "pdf"

store basename as path basename of filepath
display "Filename: " with basename             // "report.pdf"

store dirname as path dirname of filepath
display "Directory: " with dirname             // "documents"
```

## Error Handling

Always use error handling for file operations:

### File Not Found

```wfl
try:
    open file at "missing.txt" for reading as file
    wait for store content as read content from file
    close file file
    display content
catch:
    display "Error: File not found"
end try
```

### Permission Denied

```wfl
try:
    open file at "/root/protected.txt" for reading as file
    wait for store content as read content from file
    close file file
catch:
    display "Error: Cannot access file (permission denied?)"
end try
```

### Disk Full

```wfl
try:
    open file at "large_file.dat" for writing as file
    // Write large amount of data
    wait for write content huge_data into file
    close file file
catch:
    display "Error: Could not write file (disk full?)"
end try
```

## Common Patterns

### Configuration File Loader

```wfl
define action called load config with parameters filename:
    try:
        open file at filename for reading as file
        wait for store config as read content from file
        close file file
        return config
    catch:
        display "Warning: Config file not found, using defaults"
        return "default configuration"
    end try
end action

store app_config as load config with "app.config"
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
// Process all .txt files in a directory
wait for store text_files as list files in "input" with pattern "*.txt"

store processed as 0

for each filename in text_files:
    try:
        // Read input file
        open file at filename for reading as infile
        wait for store content as read content from infile
        close file infile

        // Process content (example: uppercase)
        store processed_content as touppercase of content

        // Write output file
        store outfile_name as "output/" with filename
        open file at outfile_name for writing as outfile
        wait for write content processed_content into outfile
        close file outfile

        add 1 to processed
        display "Processed: " with filename
    catch:
        display "Error processing: " with filename
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
define action called backup file with parameters source:
    store backup_name as source with ".backup"

    try:
        open file at source for reading as src
        wait for store content as read content from src
        close file src

        open file at backup_name for writing as dest
        wait for write content content into dest
        close file dest

        display "Backed up " with source with " to " with backup_name
        return yes
    catch:
        display "Backup failed for " with source
        return no
    end try
end action

call backup file with "important.txt"
```

## Advanced Operations

### Counting Lines

```wfl
open file at "code.wfl" for reading as file
wait for store content as read content from file
close file file

store lines as split of content by "\n"
store line_count as length of lines

display "File has " with line_count with " lines"
```

### Finding Files with Content

```wfl
wait for store all_files as list files in "."

store matching_files as []

for each filename in all_files:
    check if filename ends with ".wfl":
        try:
            open file at filename for reading as file
            wait for store content as read content from file
            close file file

            check if contains "display" in content:
                push with matching_files and filename
            end check
        catch:
            // Skip files we can't read
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
wait for store files as list files in "."

store total_size as 0
store file_count as 0

for each filename in files:
    try:
        store size as file size at filename
        change total_size to total_size plus size
        add 1 to file_count
    catch:
        // Skip directories or inaccessible files
    end try
end for

display "Total files: " with file_count
display "Total size: " with total_size with " bytes"
```

## Best Practices

✅ **Always close files** - Use `close file` when done

✅ **Use try-catch** - File operations can fail

✅ **Use finally for cleanup** - Ensure files are closed even with errors

✅ **Validate paths** - Check if files exist before opening

✅ **Handle missing files gracefully** - Provide defaults or error messages

✅ **Use appropriate modes** - reading vs writing vs appending

✅ **Flush buffers** - Ensure data is written (use close)

❌ **Don't leave files open** - Always close

❌ **Don't ignore errors** - File operations can fail

❌ **Don't overwrite without checking** - Validate before writing

❌ **Don't hardcode paths** - Use variables for flexibility

## File I/O with Error Handling

**Pattern: Try-Catch-Finally**

```wfl
store file_handle as nothing

try:
    open file at "data.txt" for writing as file
    change file_handle to file

    wait for write content "Important data" into file
    display "File written successfully"
catch:
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
open file at "data.txt" for writing as file
wait for write content "data" into file
// Forgot to close file!
```

**Right:**
```wfl
open file at "data.txt" for writing as file
wait for write content "data" into file
close file file  // Always close!
```

### Reading Non-Existent Files

**Wrong:**
```wfl
open file at "missing.txt" for reading as file
// CRASH if file doesn't exist!
```

**Right:**
```wfl
try:
    open file at "missing.txt" for reading as file
    wait for store content as read content from file
    close file file
catch:
    display "File not found"
end try
```

### Writing Without Wait

**Wrong:**
```wfl
open file at "data.txt" for writing as file
write content "data" into file  // Missing 'wait for'
close file file
```

**Right:**
```wfl
open file at "data.txt" for writing as file
wait for write content "data" into file  // Use 'wait for'
close file file
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
