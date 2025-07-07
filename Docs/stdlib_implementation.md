# WFL Standard Library Implementation

This document describes the implementation of the WFL standard library as specified in the `wfl-stdlib.md` document.

## Overview

The standard library is implemented as a set of Rust functions that are registered with the WFL interpreter. These functions are organized into modules:

- **Core**: Basic functions like `print`, `typeof`, and `isnothing`
- **Math**: Mathematical functions like `abs`, `round`, `floor`, `ceil`, `random`, and `clamp`
- **Text**: String manipulation functions like `length`, `touppercase`, `tolowercase`, `contains`, `substring`, and regex operations
- **List**: List manipulation functions like `length`, `push`, `pop`, `contains`, `indexof`, and advanced sorting
- **File System**: File operations like `glob`, `rglob`, `read_text`, and streaming write operations
- **Path**: Path manipulation utilities like `path_join`, `path_basename`, `path_dirname`, `path_relpath`, and `path_normalize`
- **CLI**: Command-line interface functions like `get_args`, `parse_flags`, and `usage`
- **Time**: Time and date functions like `now_iso` for ISO timestamp generation

## Implementation Details

### Function Naming Convention

Following WFL's principle of minimizing special characters, we've renamed functions to avoid underscores:

- `type_of` → `typeof`
- `is_nothing` → `isnothing`
- `to_uppercase` → `touppercase`
- `to_lowercase` → `tolowercase`
- `index_of` → `indexof`

For backward compatibility, we've kept aliases for the old names.

### Type Checking

The standard library functions are registered with the type checker to ensure proper type checking at compile time. Each function has a defined signature with parameter types and a return type.

## Testing Challenges

During testing, we encountered issues with the WFL parser's handling of function calls. The current parser implementation doesn't properly support function calls with arguments using the natural language syntax we attempted (e.g., `typeof of number value`).

The parser treats expressions like `typeof of number value` as variable names rather than function calls with arguments. When attempting to run even simple test programs, the parser panics with an error:

```
thread 'main' panicked at src/parser/mod.rs:554:44:
called `Option::unwrap()` on a `None` value
```

This confirms that the parser needs to be updated to handle function calls with arguments using the natural language syntax that aligns with WFL's design principles. Until the parser is updated, the standard library functions cannot be fully tested with WFL programs.

## New Stdlib Modules (Added for File Combiner Port)

### File System Module (`fs.rs`)

Provides file system operations with security validation and streaming support:

- `glob(directory, pattern)` - Find files matching pattern in directory
- `rglob(directory, pattern)` - Recursively find files matching pattern
- `read_text(path)` - Read text file with UTF-8 encoding and binary safety checks
- `write_stream_open(path)` - Open file for streaming writes, returns handle ID
- `write_stream_write(handle_id, content)` - Write content to stream
- `write_stream_close(handle_id)` - Close stream and release resources

**Security Features:**
- Path validation to prevent writes outside repository root
- Binary file detection with encoding fallback support
- Symlink loop protection in recursive operations

### Path Module (`path.rs`)

Cross-platform path manipulation utilities:

- `path_join(path1, path2)` - Join path components safely
- `path_basename(path)` - Extract filename from path
- `path_dirname(path)` - Extract directory from path  
- `path_relpath(path, base)` - Calculate relative path
- `path_normalize(path)` - Normalize path with `/` separators and collapse `.`/`..`

**Cross-Platform Design:**
- Consistent `/` separator usage for TOC generation
- Handles both Unix and Windows path formats
- Prevents path traversal vulnerabilities

### CLI Module (`cli.rs`)

Command-line argument parsing with argparse-style functionality:

- `get_args()` - Get command-line arguments as list
- `parse_flags(spec)` - Parse flags according to specification string
- `usage(spec)` - Generate usage text from specification

**Flag Specification Format:**
```
--flag-name: type default value
```

Supported types: `boolean`, `string`, `number`, `choice`

### Extended Text Module

Added regex support to existing text functions:

- `regex_find(text, pattern)` - Find first regex match
- `regex_match_all(text, pattern)` - Find all regex matches
- `regex_replace(text, pattern, replacement)` - Replace regex matches

### Extended List Module

Added advanced sorting capabilities:

- `sort_by(list, criteria)` - Sort with multiple criteria types
  - `"alpha"` - Alphabetical sorting
  - `"time"` - Time-based sorting (requires file metadata)
  - Custom list - Sort by custom order with alphabetical fallback

### Extended Time Module

Added ISO timestamp generation:

- `now_iso()` - Generate ISO 8601 timestamp for file headers

## Implementation Notes

### Streaming API Design

The file system module uses a streaming approach for large file operations:

```rust
// Open stream with configurable chunk size
let handle = write_stream_open("output.md");
write_stream_write(handle, content_chunk);
write_stream_close(handle);
```

This prevents memory issues when processing large repositories and implements `Drop` trait for automatic cleanup.

### Error Handling Strategy

All new stdlib functions follow WFL's error handling patterns:

- Return `RuntimeError` for invalid arguments or system failures
- Validate input parameters with descriptive error messages
- Use `Result<Value, RuntimeError>` return type consistently

### Type Registration

New functions are registered in both the interpreter environment and type checker:

- `src/stdlib/mod.rs` - Runtime function registration
- `src/stdlib/typechecker.rs` - Compile-time type checking

## Current Parser Limitations

The new stdlib functions are implemented and registered, but cannot be fully tested due to parser limitations with the `function of argument` syntax pattern. The parser currently:

- Treats `of` as `KeywordOf` token in lexer but expects `Identifier("of")` in parser
- Only supports `of` in specific pattern contexts, not general function calls
- Causes semantic errors when attempting to use new stdlib functions

## Future Work

1. **Parser Enhancement**: Update the parser to properly handle function calls with arguments using natural language syntax.
2. **Integration Testing**: Once parser is fixed, implement comprehensive tests comparing WFL vs Python combiner outputs
3. **Performance Optimization**: Add benchmarking for large repository processing
4. **Cross-Platform Testing**: Validate Windows compatibility for path operations
5. **Documentation Generation**: Auto-generate `Docs/STD_LIB.md` index for new stdlib functions
