# Filesystem Module

The Filesystem module provides comprehensive file and directory operations. Read, write, manage files and directories with natural language syntax.

## File Operations

File operations are covered in detail in [File I/O](../04-advanced-features/file-io.md). This reference covers the utility functions.

### Core File Functions

- `open file at <path> for reading/writing/appending` - Open files
- `read content from <file>` - Read file content
- `write content <text> into <file>` - Write to file
- `append content <text> into <file>` - Append to file
- `close file <file>` - Close file handle

**See:** [File I/O Guide](../04-advanced-features/file-io.md)

## Directory Functions

### list files in

**Purpose:** List files in a directory.

**Signature:**
```wfl
list files in <path>
```

**Alternative with pattern:**
```wfl
list files in <path> with pattern <pattern>
```

**Parameters:**
- `path` (Text): Directory path
- `pattern` (Text, optional): Glob pattern (e.g., "*.wfl")

**Returns:** List - Filenames in the directory

**Example:**
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

**Use Cases:**
- Directory listing
- File discovery
- Batch processing

---

### makedirs

**Purpose:** Create a directory and all parent directories (like `mkdir -p`).

**Signature:**
```wfl
makedirs <path>
```

**Alternative:**
```wfl
create directory at <path>
```

**Parameters:**
- `path` (Text): Directory path to create

**Returns:** None

**Example:**
```wfl
makedirs "output/reports/2026"
display "Directory structure created"

// Creates:
// output/
// output/reports/
// output/reports/2026/
```

**Use Cases:**
- Ensure output directories exist
- Create nested directory structures

---

## Path Functions

### path_join

**Purpose:** Join path components into a single path.

**Signature:**
```wfl
path_join of <part1> and <part2> and ...
```

**Parameters:**
- One or more path components. `path_join` is variadic — it joins all the
  components you pass (e.g. `path_join of "home" and "user" and "docs"` →
  `home/user/docs`). Note: the type checker currently emits a non-fatal warning
  when more than two components are passed, so the two-at-a-time / chained form
  shown below avoids that warning. Tracking: wfl#571.

**Returns:** Text - Joined path with proper separators

**Example:**
```wfl
store part as path_join of "home" and "user"
store dir_path as path_join of part and "documents"
display dir_path
// Output: home/user/documents (or home\user\documents on Windows)

store file_path as path_join of "output" and "report.txt"
display file_path
// Output: output/report.txt
```

**Use Cases:**
- Build paths programmatically
- Cross-platform path handling

---

### path_extension

**Purpose:** Get the file extension from a path.

**Signature:**
```wfl
path_extension of <path>
```

**Parameters:**
- `path` (Text): File path

**Returns:** Text - Extension without dot (e.g., "txt", "pdf")

**Example:**
```wfl
store ext1 as path_extension of "document.pdf"
display ext1                              // Output: pdf

store ext2 as path_extension of "archive.tar.gz"
display ext2                              // Output: gz

store ext3 as path_extension of "readme"
display ext3                              // Output: (empty string)
```

**Use Cases:**
- File type detection
- Content-type determination
- File filtering

**Example: Content Type Mapping**
```wfl
define action called get_content_type with parameters filename:
    store ext as path_extension of filename

    check if ext is "html":
        return "text/html"
    otherwise:
        check if ext is "css":
            return "text/css"
        otherwise:
            check if ext is "js":
                return "application/javascript"
            otherwise:
                check if ext is "json":
                    return "application/json"
                otherwise:
                    check if ext is "pdf":
                        return "application/pdf"
                    otherwise:
                        return "text/plain"
                    end check
                end check
            end check
        end check
    end check
end action

store content_type as get_content_type of "index.html"
display content_type                      // Output: text/html
```

---

### path_basename

**Purpose:** Get the filename from a path (last component).

**Signature:**
```wfl
path_basename of <path>
```

**Parameters:**
- `path` (Text): File path

**Returns:** Text - The filename with extension

**Example:**
```wfl
store name1 as path_basename of "documents/report.pdf"
display name1                             // Output: report.pdf

store name2 as path_basename of "/home/user/file.txt"
display name2                             // Output: file.txt

store name3 as path_basename of "readme.md"
display name3                             // Output: readme.md
```

**Use Cases:**
- Extract filename from full path
- Display friendly names
- File organization

---

### path_dirname

**Purpose:** Get the directory part of a path.

**Signature:**
```wfl
path_dirname of <path>
```

**Parameters:**
- `path` (Text): File path

**Returns:** Text - The directory path

**Example:**
```wfl
store dir1 as path_dirname of "documents/report.pdf"
display dir1                              // Output: documents

store dir2 as path_dirname of "/home/user/file.txt"
display dir2                              // Output: /home/user

store dir3 as path_dirname of "file.txt"
display dir3                              // Output: (current directory)
```

**Use Cases:**
- Get file location
- Validate directory exists
- Build related paths

---

### path_stem

**Purpose:** Get filename without extension.

**Signature:**
```wfl
path_stem of <path>
```

**Parameters:**
- `path` (Text): File path

**Returns:** Text - Filename without extension

**Example:**
```wfl
store stem1 as path_stem of "document.pdf"
display stem1                             // Output: document

store stem2 as path_stem of "archive.tar.gz"
display stem2                             // Output: archive.tar

store stem3 as path_stem of "readme"
display stem3                             // Output: readme
```

**Use Cases:**
- Generate output filenames
- File renaming
- Comparison without extensions

**Example: Generate Output Filename**
```wfl
store input as "report.txt"
store stem as path_stem of input
store out_name as stem with ".processed.txt"
display out_name
// Output: report.processed.txt
```

---

## File Information Functions

### path_exists

**Purpose:** Check if a path exists (file or directory).

**Signature:**
```wfl
path_exists of <path>
```

**Alternative (checks a file specifically):**
```wfl
file exists at <path>
```

**Parameters:**
- `path` (Text): Path to check

**Returns:** Boolean - `yes` if exists, `no` otherwise

**Example:**
```wfl
check if file exists at "data.txt":
    display "File exists"
otherwise:
    display "File not found"
end check

check if directory exists at "output":
    display "Directory exists"
otherwise:
    makedirs "output"
    display "Directory created"
end check
```

---

### is_file

**Purpose:** Check if path is a regular file.

**Signature:**
```wfl
is_file of <path>
```

**Parameters:**
- `path` (Text): Path to check

**Returns:** Boolean - `yes` if file, `no` if directory or doesn't exist

**Example:**
```wfl
check if is_file of "data.txt":
    display "It's a file"
end check

check if is_file of "output":
    display "It's a file"
otherwise:
    display "It's a directory (or doesn't exist)"
end check
```

---

### is_dir

**Purpose:** Check if path is a directory.

**Signature:**
```wfl
is_dir of <path>
```

**Parameters:**
- `path` (Text): Path to check

**Returns:** Boolean - `yes` if directory, `no` otherwise

**Example:**
```wfl
check if is_dir of "output":
    display "It's a directory"
end check
```

---

### file_size

**Purpose:** Get file size in bytes.

**Signature:**
```wfl
file size of <path>
```

**Parameters:**
- `path` (Text): File path

**Returns:** Number - Size in bytes

**Example:**
```wfl
// Create a file first so it has a size to report
open file at "data.txt" for writing as data_file
wait for write content "Hello, WFL!" into data_file
close file data_file

store size as file size of "data.txt"
display "File size: " with size with " bytes"

// Convert to KB
store kb as size divided by 1024
display "File size: " with round of kb with " KB"
```

**Use Cases:**
- Check file size before reading
- Disk usage calculation
- Progress indicators

---

### count_lines

**Purpose:** Count lines in a text file.

**Signature:**
```wfl
count_lines of <path>
```

**Parameters:**
- `path` (Text): File path

**Returns:** Number - Line count

**Example:**
```wfl
// Create a small file first
open file at "code.wfl" for writing as code_file
wait for write content "line one\nline two\nline three" into code_file
close file code_file

store lines as count_lines of "code.wfl"
display "File has " with lines with " lines"
```

---

## File Operations Functions

### copy_file

**Purpose:** Copy a file from source to destination.

**Signature:**
```wfl
copy_file from <source> to <destination>
```

**Parameters:**
- `source` (Text): Source file path
- `destination` (Text): Destination path

**Returns:** None

**Example:**
```wfl
// Create a source file first
open file at "original.txt" for writing as source_file
wait for write content "Important data" into source_file
close file source_file

copy_file from "original.txt" to "backup.txt"
display "File copied"
```

---

### move_file

**Purpose:** Move or rename a file.

**Signature:**
```wfl
move_file from <source> to <destination>
```

**Parameters:**
- `source` (Text): Current path
- `destination` (Text): New path

**Returns:** None

**Example:**
```wfl
// Create a file to rename first
open file at "old_name.txt" for writing as old_file
wait for write content "content" into old_file
close file old_file

move_file from "old_name.txt" to "new_name.txt"
display "File renamed"
```

---

### remove_file

**Purpose:** Delete a file.

**Signature:**
```wfl
remove_file at <path>
```

**Aliases:** `delete_file`

**Parameters:**
- `path` (Text): File to delete

**Returns:** None

**Example:**
```wfl
// Create a temporary file to delete
open file at "temp.txt" for writing as temp_file
close file temp_file

try:
    remove_file at "temp.txt"
    display "File deleted"
when error:
    display "Could not delete file"
end try
```

---

### remove_dir

**Purpose:** Remove a directory.

**Signature:**
```wfl
remove_dir at <path>
```

**With recursive option:**
```wfl
remove_dir at <path> recursive yes
```

**Parameters:**
- `path` (Text): Directory to remove
- `recursive` (Boolean, optional): If yes, removes contents too

**Returns:** None

**Example:**
```wfl
// Create then remove an empty directory
makedirs "empty_folder"
remove_dir at "empty_folder"

// Create a directory with contents, then remove it recursively
makedirs "old_folder/sub"
remove_dir at "old_folder" recursive yes
```

---

## Complete Example

Using filesystem functions together:

```wfl
display "=== Filesystem Module Demo ==="
display ""

// List files
wait for store files as list files in "."
display "Files in current directory: " with length of files

// Filter WFL files
create list wfl_files:
end list

for each filename in files:
    store ext as path_extension of filename
    check if ext is "wfl":
        push with wfl_files and filename
    end check
end for

display "WFL files found: " with length of wfl_files
display ""

// File information
for each wfl_file in wfl_files:
    check if is_file of wfl_file:
        store size as file size of wfl_file
        store lines as count_lines of wfl_file
        store stem as path_stem of wfl_file

        display wfl_file with ":"
        display "  Size: " with size with " bytes"
        display "  Lines: " with lines
        display "  Name: " with stem
    end check
end for

display ""
display "=== Demo Complete ==="
```

## Best Practices

✅ **Check exists before operations:** Use `path exists at` or `file exists at`

✅ **Use try-catch for file operations:** They can fail

✅ **Validate paths:** Prevent directory traversal attacks

✅ **Use path_join for portability:** Works on Windows and Unix

✅ **Check is_file vs is_dir:** Verify path type before operations

❌ **Don't hardcode path separators:** Use path_join instead

❌ **Don't delete without confirmation:** Especially with recursive

❌ **Don't assume files exist:** Always check first

## What You've Learned

In this module, you learned:

✅ **Directory operations** - list files in, makedirs, remove_dir
✅ **Path operations** - join, extension, basename, dirname, stem
✅ **File information** - exists, is_file, is_dir, size, count_lines
✅ **File operations** - copy, move, remove
✅ **Best practices** - Safety, validation, error handling

## Next Steps

**[Time Module →](time-module.md)**
Date and time operations.

**[File I/O Guide →](../04-advanced-features/file-io.md)**
Complete guide to reading and writing files.

Or return to:
**[Standard Library Index →](index.md)**

---

**Previous:** [← List Module](list-module.md) | **Next:** [Time Module →](time-module.md)
