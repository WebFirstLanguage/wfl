# WFL Filesystem Module API Reference

## Overview

The Filesystem module provides comprehensive file and directory operations for WFL programs. It enables safe and efficient file manipulation, directory traversal, and path operations with cross-platform compatibility.

## Directory Operations

### `list_dir(path)`

Lists all files and directories in the specified path.

**Parameters:**
- `path` (Text): Directory path to list contents of

**Returns:** List of Text (file and directory names)

**Examples:**

```wfl
// List current directory contents
store current_files as list_dir of "."
display "Files in current directory:"
count file in current_files:
    display "- " with file
end

// List specific directory
store src_files as list_dir of "src"
display "Source files:"
count file in src_files:
    display "  " with file
end

// Count files in directory
store doc_files as list_dir of "Docs"
store file_count as length of doc_files
display "Documentation directory contains " with file_count with " items"
```

**Natural Language Variants:**
```wfl
// All equivalent ways to list directory
store files1 as list_dir of path
store files2 as list directory path
store files3 as files in path
store files4 as directory contents of path
```

**Error Handling:**
```wfl
// The function will error if directory doesn't exist or isn't accessible
// Safe usage with path validation
action safe_list_directory with dir_path:
    check if path_exists of dir_path:
        check if is_dir of dir_path:
            return list_dir of dir_path
        otherwise:
            display "Path is not a directory: " with dir_path
            return []
        end
    otherwise:
        display "Directory does not exist: " with dir_path
        return []
    end
end
```

---

### `makedirs(path)`

Creates a directory and all necessary parent directories.

**Parameters:**
- `path` (Text): Directory path to create

**Returns:** Nothing

**Examples:**

```wfl
// Create single directory
makedirs of "new_folder"

// Create nested directory structure
makedirs of "data/output/results"
makedirs of "logs/2025/august"

// Create project structure
store project_name as "my_project"
makedirs of project_name with "/src"
makedirs of project_name with "/docs"
makedirs of project_name with "/tests"
makedirs of project_name with "/data/input"
makedirs of project_name with "/data/output"

display "Project structure created for " with project_name
```

**Natural Language Variants:**
```wfl
// All equivalent ways to create directories
makedirs of path
make directories path
create directories path
create directory structure path
```

**Practical Use Cases:**

```wfl
// Organize output by date
action create_daily_output_dir:
    store today_str as current_date  // YYYY-MM-DD format
    store output_dir as "output/" with today_str
    makedirs of output_dir
    display "Created daily output directory: " with output_dir
    return output_dir
end

// Setup logging directory structure
action setup_logging_dirs:
    store base_log_dir as "logs"
    makedirs of base_log_dir
    makedirs of base_log_dir with "/error"
    makedirs of base_log_dir with "/info"
    makedirs of base_log_dir with "/debug"
    display "Logging directory structure ready"
end

// User data organization
action setup_user_workspace with username:
    store user_dir as "users/" with username
    makedirs of user_dir
    makedirs of user_dir with "/documents"
    makedirs of user_dir with "/downloads"
    makedirs of user_dir with "/settings"
    display "Workspace created for user: " with username
    return user_dir
end
```

## File and Path Inspection

### `path_exists(path)`

Checks if a file or directory exists at the specified path.

**Parameters:**
- `path` (Text): Path to check

**Returns:** Boolean (yes if exists, no if not)

**Examples:**

```wfl
// Check if files exist before processing
check if path_exists of "config.txt":
    display "Configuration file found"
    // Process config file
otherwise:
    display "Configuration file missing, using defaults"
end

// Validate multiple files
store required_files as ["input.txt", "settings.json", "template.html"]
store all_files_exist as yes

count file in required_files:
    check if not path_exists of file:
        display "Missing required file: " with file
        store all_files_exist as no
    end
end

check if all_files_exist:
    display "All required files present"
otherwise:
    display "Cannot proceed - missing files"
end
```

**Natural Language Variants:**
```wfl
// All equivalent ways to check existence
check if path_exists of path
check if path exists
check if file exists path
check if there is a path
```

---

### `is_file(path)`

Checks if a path points to a file (not a directory).

**Parameters:**
- `path` (Text): Path to check

**Returns:** Boolean (yes if file, no if not or if directory)

**Examples:**

```wfl
// Distinguish between files and directories
store items as list_dir of "."
count item in items:
    check if is_file of item:
        display "📄 File: " with item
    check if is_dir of item:
        display "📁 Directory: " with item
    otherwise:
        display "❓ Unknown: " with item
    end
end

// Filter for files only
action get_files_only with directory:
    store all_items as list_dir of directory
    store files_only as []
    
    count item in all_items:
        store full_path as path_join of directory and item
        check if is_file of full_path:
            push of files_only and item
        end
    end
    
    return files_only
end
```

---

### `is_dir(path)`

Checks if a path points to a directory (not a file).

**Parameters:**
- `path` (Text): Path to check

**Returns:** Boolean (yes if directory, no if not or if file)

**Examples:**

```wfl
// Find subdirectories
action get_subdirectories with parent_dir:
    store all_items as list_dir of parent_dir
    store directories as []
    
    count item in all_items:
        store full_path as path_join of parent_dir and item
        check if is_dir of full_path:
            push of directories and item
        end
    end
    
    return directories
end

// Recursive directory processing
action process_directory with dir_path:
    check if is_dir of dir_path:
        display "Processing directory: " with dir_path
        
        store contents as list_dir of dir_path
        count item in contents:
            store item_path as path_join of dir_path and item
            check if is_dir of item_path:
                // Recursive call would go here
                display "  Found subdirectory: " with item
            otherwise:
                display "  Found file: " with item
            end
        end
    otherwise:
        display "Not a directory: " with dir_path
    end
end
```

---

### `file_mtime(path)`

Returns the modification time of a file as a Unix timestamp.

**Parameters:**
- `path` (Text): Path to the file

**Returns:** Number (Unix timestamp in seconds)

**Examples:**

```wfl
// Check file modification time
store config_mtime as file_mtime of "config.txt"
display "Config last modified: " with config_mtime

// Compare file ages
action find_newer_file with file1 and file2:
    store mtime1 as file_mtime of file1
    store mtime2 as file_mtime of file2
    
    check if mtime1 > mtime2:
        display file1 with " is newer than " with file2
        return file1
    otherwise:
        display file2 with " is newer than " with file1
        return file2
    end
end

// Find most recently modified file
action find_newest_file with files:
    check if length of files is 0:
        return nothing
    end
    
    store newest_file as files[0]
    store newest_time as file_mtime of newest_file
    
    count file in files:
        store file_time as file_mtime of file
        check if file_time > newest_time:
            store newest_file as file
            store newest_time as file_time
        end
    end
    
    return newest_file
end
```

## Path Manipulation

### `path_join(component1, component2, ...)`

Joins multiple path components into a single path using the appropriate separator.

**Parameters:**
- `component1, component2, ...` (Text): Path components to join

**Returns:** Text (joined path)

**Examples:**

```wfl
// Basic path joining
store doc_path as path_join of "docs" and "api" and "index.html"
display "Documentation path: " with doc_path  // "docs/api/index.html" or "docs\api\index.html"

// Build paths dynamically
store base_dir as "data"
store year as "2025"
store month as "08"
store filename as "report.txt"

store full_path as path_join of base_dir and year and month and filename
display "Report path: " with full_path  // "data/2025/08/report.txt"

// User-specific paths
action get_user_file_path with username and filename:
    return path_join of "users" and username and "files" and filename
end

store user_doc as get_user_file_path of "alice" and "document.txt"
display "User document: " with user_doc  // "users/alice/files/document.txt"
```

**Cross-platform Benefits:**
```wfl
// The same code works on Windows, macOS, and Linux
store config_path as path_join of "app" and "config" and "settings.json"
// Windows: "app\config\settings.json"
// Unix-like: "app/config/settings.json"
```

---

### `path_basename(path)`

Returns the filename portion of a path (everything after the last separator).

**Parameters:**
- `path` (Text): File path

**Returns:** Text (filename)

**Examples:**

```wfl
// Extract filenames
store filename1 as path_basename of "/home/user/document.txt"
display "Filename: " with filename1  // "document.txt"

store filename2 as path_basename of "C:\\Users\\Alice\\photo.jpg"
display "Photo name: " with filename2  // "photo.jpg"

// Process multiple files
store file_paths as [
    "docs/readme.md",
    "src/main.wfl",
    "tests/test_core.wfl"
]

display "Processing files:"
count path in file_paths:
    store filename as path_basename of path
    display "- " with filename
end
```

**Practical Use Cases:**

```wfl
// Extract file extension
action get_file_extension with filepath:
    store filename as path_basename of filepath
    store name_length as length of filename
    
    // Find last dot position (simplified - would need better string functions)
    store extension as ""  // Would need to implement dot finding
    return extension
end

// Create backup filename
action create_backup_name with original_path:
    store filename as path_basename of original_path
    store directory as path_dirname of original_path
    
    store backup_name as filename with ".backup"
    return path_join of directory and backup_name
end
```

---

### `path_dirname(path)`

Returns the directory portion of a path (everything before the last separator).

**Parameters:**
- `path` (Text): File path

**Returns:** Text (directory path)

**Examples:**

```wfl
// Extract directory paths
store dir1 as path_dirname of "/home/user/documents/file.txt"
display "Directory: " with dir1  // "/home/user/documents"

store dir2 as path_dirname of "src/main.wfl"
display "Source directory: " with dir2  // "src"

// Organize files by directory
action group_files_by_directory with file_paths:
    store directories as []
    
    count path in file_paths:
        store dir as path_dirname of path
        check if not contains of directories and dir:
            push of directories and dir
        end
    end
    
    display "Found directories:"
    count dir in directories:
        display "- " with dir
    end
    
    return directories
end
```

## Pattern Matching and Search

### `glob(pattern, base_path)`

Finds files matching a glob pattern in the specified directory.

**Parameters:**
- `pattern` (Text): Glob pattern to match
- `base_path` (Text): Base directory to search in

**Returns:** List of Text (matching file paths)

**Pattern Syntax:**
- `*` - Matches any number of characters (except path separators)
- `?` - Matches any single character
- `[abc]` - Matches any character in brackets
- `[a-z]` - Matches any character in range

**Examples:**

```wfl
// Find all text files
store text_files as glob of "*.txt" and "."
display "Text files:"
count file in text_files:
    display "- " with file
end

// Find WFL programs
store wfl_programs as glob of "*.wfl" and "TestPrograms"
store program_count as length of wfl_programs
display "Found " with program_count with " WFL programs"

// Pattern matching examples
store js_files as glob of "*.js" and "web/js"
store test_files as glob of "test_*.wfl" and "tests"
store config_files as glob of "config*" and "settings"

// Character range matching
store log_files as glob of "log_[0-9]*.txt" and "logs"
store temp_files as glob of "temp_?.dat" and "temp"
```

**Practical Use Cases:**

```wfl
// Find files by type
action find_image_files with directory:
    store jpg_files as glob of "*.jpg" and directory
    store png_files as glob of "*.png" and directory
    store gif_files as glob of "*.gif" and directory
    
    store all_images as []
    count file in jpg_files:
        push of all_images and file
    end
    count file in png_files:
        push of all_images and file
    end
    count file in gif_files:
        push of all_images and file
    end
    
    return all_images
end

// Backup old files
action find_old_backups with backup_dir:
    // Find backup files with date pattern
    store old_backups as glob of "backup_20[0-9][0-9]-[0-9][0-9]-[0-9][0-9]*" and backup_dir
    return old_backups
end

// Development tools
action find_source_files with project_dir:
    store wfl_files as glob of "*.wfl" and project_dir
    store rust_files as glob of "*.rs" and project_dir
    
    display "Source files found:"
    display "WFL files: " with length of wfl_files
    display "Rust files: " with length of rust_files
    
    return [wfl_files, rust_files]
end
```

---

### `rglob(pattern, base_path)`

Recursively finds files matching a glob pattern in the specified directory and all subdirectories.

**Parameters:**
- `pattern` (Text): Glob pattern to match
- `base_path` (Text): Base directory to search from

**Returns:** List of Text (matching file paths, including subdirectories)

**Examples:**

```wfl
// Recursively find all Rust source files
store all_rust_files as rglob of "*.rs" and "src"
display "All Rust files in src/ tree:"
count file in all_rust_files:
    display "- " with file
end

// Find all documentation files
store all_docs as rglob of "*.md" and "."
store doc_count as length of all_docs
display "Found " with doc_count with " documentation files"

// Find test files anywhere in project
store all_tests as rglob of "*test*.wfl" and "."
display "Test files throughout project:"
count test_file in all_tests:
    display "  " with test_file
end
```

**Comparison with `glob`:**
```wfl
// glob - only current directory
store current_wfl as glob of "*.wfl" and "."

// rglob - all subdirectories too
store all_wfl as rglob of "*.wfl" and "."

display "WFL files in current directory: " with length of current_wfl
display "WFL files in entire tree: " with length of all_wfl
```

**Practical Use Cases:**

```wfl
// Code analysis
action analyze_codebase with root_dir:
    store source_files as rglob of "*.wfl" and root_dir  
    store total_files as length of source_files
    
    store total_lines as 0
    // Would need file reading functions to count lines
    
    display "Codebase analysis:"
    display "Total WFL files: " with total_files
    display "Files found:"
    count file in source_files:
        display "  " with file
    end
end

// Find configuration files
action find_all_configs with project_root:
    store json_configs as rglob of "*.json" and project_root
    store yaml_configs as rglob of "*.yml" and project_root
    store ini_configs as rglob of "*.ini" and project_root
    
    store all_configs as []
    count config in json_configs:
        push of all_configs and config
    end
    count config in yaml_configs:
        push of all_configs and config
    end
    count config in ini_configs:
        push of all_configs and config
    end
    
    return all_configs
end

// Cleanup old files
action find_temp_files_everywhere with root_dir:
    store temp_files as rglob of "*.tmp" and root_dir
    store cache_files as rglob of "*.cache" and root_dir
    store log_files as rglob of "*.log" and root_dir
    
    store cleanup_candidates as []
    count file in temp_files:
        push of cleanup_candidates and file
    end
    count file in cache_files:
        push of cleanup_candidates and file
    end
    count file in log_files:
        push of cleanup_candidates and file
    end
    
    display "Found " with length of cleanup_candidates with " files to potentially clean up"
    return cleanup_candidates
end
```

## Advanced Examples

### File Organization System

```wfl
// Organize files by type into directories
action organize_downloads with downloads_dir:
    store all_files as list_dir of downloads_dir
    
    // Create organization directories
    makedirs of downloads_dir with "/images"
    makedirs of downloads_dir with "/documents"
    makedirs of downloads_dir with "/archives"
    makedirs of downloads_dir with "/other"
    
    count filename in all_files:
        store file_path as path_join of downloads_dir and filename
        
        check if is_file of file_path:
            store basename as path_basename of filename
            
            // Simple file type detection by extension
            check if contains of basename and ".jpg" or contains of basename and ".png":
                store dest as path_join of downloads_dir and "images" and filename
                display "Would move " with filename with " to images/"
            check if contains of basename and ".pdf" or contains of basename and ".doc":
                store dest as path_join of downloads_dir and "documents" and filename
                display "Would move " with filename with " to documents/"
            check if contains of basename and ".zip" or contains of basename and ".tar":
                store dest as path_join of downloads_dir and "archives" and filename
                display "Would move " with filename with " to archives/"
            otherwise:
                store dest as path_join of downloads_dir and "other" and filename
                display "Would move " with filename with " to other/"
            end
        end
    end
end
```

### Duplicate File Finder

```wfl
// Find potential duplicate files by name and size
action find_potential_duplicates with search_dir:
    store all_files as rglob of "*" and search_dir
    store file_info as []
    
    // Collect file information
    count file_path in all_files:
        check if is_file of file_path:
            store basename as path_basename of file_path
            store mtime as file_mtime of file_path
            store info as [basename, file_path, mtime]
            push of file_info and info
        end
    end
    
    // Group by filename
    store duplicates as []
    count i from 0 to length of file_info - 1:
        store current_info as file_info[i]
        store current_name as current_info[0]
        store current_path as current_info[1]
        
        store matches as [current_path]
        
        count j from i + 1 to length of file_info - 1:
            store other_info as file_info[j]
            store other_name as other_info[0]
            store other_path as other_info[1]
            
            check if current_name is other_name:
                push of matches and other_path
            end
        end
        
        check if length of matches > 1:
            push of duplicates and [current_name, matches]
        end
    end
    
    display "Potential duplicates found:"
    count dup_group in duplicates:
        store dup_name as dup_group[0]
        store dup_paths as dup_group[1]
        
        display "File name: " with dup_name
        count path in dup_paths:
            display "  " with path
        end
    end
    
    return duplicates
end
```

### Project Structure Analyzer

```wfl
// Analyze project directory structure
action analyze_project_structure with project_root:
    store structure_info as []
    
    // Find different types of files
    store source_files as rglob of "*.wfl" and project_root
    store rust_files as rglob of "*.rs" and project_root
    store doc_files as rglob of "*.md" and project_root
    store config_files as rglob of "*.json" and project_root
    store test_files as rglob of "*test*" and project_root
    
    // Get directory structure
    store all_items as rglob of "*" and project_root
    store directories as []
    
    count item in all_items:
        check if is_dir of item:
            push of directories and item
        end
    end
    
    // Generate report
    display "Project Structure Analysis"
    display "========================"
    display "Root directory: " with project_root
    display ""
    
    display "File Type Summary:"
    display "- WFL source files: " with length of source_files
    display "- Rust source files: " with length of rust_files  
    display "- Documentation files: " with length of doc_files
    display "- Configuration files: " with length of config_files
    display "- Test files: " with length of test_files
    display "- Total directories: " with length of directories
    display ""
    
    display "Directory Structure:"
    count dir in directories:
        display "📁 " with dir
    end
    
    return [source_files, rust_files, doc_files, config_files, test_files, directories]
end
```

## Error Handling and Best Practices

### Safe File Operations

```wfl
// Wrapper for safe file operations
action safe_file_operation with operation and path:
    check if isnothing of path or length of path is 0:
        display "Error: Invalid path provided"
        return nothing
    end
    
    check if operation is "read":
        check if not path_exists of path:
            display "Error: File does not exist: " with path
            return nothing
        end
        check if not is_file of path:
            display "Error: Path is not a file: " with path
            return nothing
        end
        // Proceed with read operation
        
    check if operation is "list":
        check if not path_exists of path:
            display "Error: Directory does not exist: " with path
            return []
        end
        check if not is_dir of path:
            display "Error: Path is not a directory: " with path
            return []
        end
        return list_dir of path
        
    otherwise:
        display "Error: Unknown operation: " with operation
        return nothing
    end
end
```

### Input Validation

```wfl
// Validate path components
action validate_path with path:
    check if isnothing of path:
        return "Path is nothing"
    end
    
    check if length of path is 0:
        return "Path is empty"
    end
    
    // Check for dangerous characters (simplified)
    check if contains of path and "..":
        return "Path contains dangerous '..' sequence"
    end
    
    return "valid"
end

// Safe directory creation
action safe_makedirs with dir_path:
    store validation_result as validate_path of dir_path
    check if validation_result is not "valid":
        display "Path validation failed: " with validation_result
        return no
    end
    
    check if path_exists of dir_path:
        check if is_dir of dir_path:
            display "Directory already exists: " with dir_path
            return yes
        otherwise:
            display "Path exists but is not a directory: " with dir_path
            return no
        end
    end
    
    makedirs of dir_path
    display "Created directory: " with dir_path
    return yes
end
```

## Performance Considerations

1. **Large directories**: `list_dir` loads all entries into memory
2. **Recursive operations**: `rglob` can be slow on large directory trees
3. **Path operations**: Most functions are fast as they don't access disk
4. **File existence checks**: These require disk access and can be slow

```wfl
// Efficient directory processing
action process_large_directory with dir_path:
    store files as list_dir of dir_path
    store file_count as length of files
    
    check if file_count > 1000:
        display "Large directory detected (" with file_count with " files)"
        display "Processing in batches..."
        
        // Process in smaller chunks
        store batch_size as 100
        count i from 0 to file_count - 1 by batch_size:
            store batch_end as (i + batch_size) min file_count
            display "Processing files " with i with " to " with batch_end
            
            count j from i to batch_end - 1:
                store filename as files[j]
                // Process individual file
            end
        end
    otherwise:
        // Process normally
        count file in files:
            // Process file
        end
    end
end
```

## Integration with Other Modules

### With Text Module

```wfl
// File filtering by content patterns
action filter_files_by_name with files and pattern:
    store matching_files as []
    
    count filename in files:
        store lower_filename as tolowercase of filename
        store lower_pattern as tolowercase of pattern
        
        check if contains of lower_filename and lower_pattern:
            push of matching_files and filename
        end
    end
    
    return matching_files
end
```

### With Time Module

```wfl
// Find files modified in last N days
action find_recent_files with directory and days_ago:
    store all_files as rglob of "*" and directory
    store recent_files as []
    
    store cutoff_days as days_ago
    store cutoff_timestamp as (current_timestamp - (cutoff_days * 24 * 60 * 60))  // Conceptual
    
    count file_path in all_files:
        check if is_file of file_path:
            store file_mtime as file_mtime of file_path
            check if file_mtime > cutoff_timestamp:
                push of recent_files and file_path
            end
        end
    end
    
    return recent_files
end
```

## See Also

- [Core Module](core-module.md) - Basic utilities and type checking
- [Text Module](text-module.md) - String operations for path manipulation
- [List Module](list-module.md) - Working with file lists
- [Pattern Module](pattern-module.md) - Advanced pattern matching
- [WFL Language Reference](../language-reference/wfl-spec.md) - Complete language specification