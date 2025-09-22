# Rust Line Counter Tool Specification

## Overview

The Rust Line Counter (`rust_loc_counter.py`) is a Python-based code analysis tool designed to count and categorize lines of code in Rust projects. It provides detailed statistics about code composition, including breakdowns by files and directories, and generates both console and markdown reports.

## Purpose

This tool serves multiple purposes:
- **Code Quality Assessment**: Understand the size and structure of Rust codebases
- **Project Documentation**: Generate automated reports for documentation
- **Development Tracking**: Monitor codebase growth over time
- **Code Review Support**: Provide metrics for code review processes

## Core Functionality

### Line Classification

The tool categorizes every line in Rust source files (`.rs` files) into four distinct types:

1. **Code Lines**: Executable Rust code, including:
   - Variable declarations and assignments
   - Function definitions and calls
   - Control flow statements (if, loop, match, etc.)
   - Import statements (use, mod)
   - Struct/enum definitions
   - Any line containing executable logic

2. **Comment Lines**: Documentation and explanatory text, including:
   - Single-line comments (`// comment`)
   - Multi-line block comments (`/* comment */`)
   - Doc comments (`/// comment`, `/** comment */`)

3. **Blank Lines**: Empty lines or lines containing only whitespace

4. **Total Lines**: Sum of all line types

### Processing Features

- **Recursive Directory Traversal**: Scans all `.rs` files in the specified directory and subdirectories
- **Smart Comment Handling**: Properly handles both single-line and multi-line comments
- **Block Comment State Tracking**: Accurately tracks multi-line comment boundaries
- **Mixed Line Processing**: Handles lines containing both code and comments
- **Error Recovery**: Continues processing even if individual files encounter errors

## Technical Specifications

### Input Requirements

- **Target Directory**: Defaults to `../src` relative to the script location
- **File Types**: Processes only Rust source files (`.rs` extension)
- **Encoding**: Expects UTF-8 encoded files
- **File System**: Works on Windows, Linux, and macOS

### Output Formats

#### Console Report
- Plain text format with tabular data
- Includes header with generation timestamp
- Three main sections: Overall Statistics, Lines by Directory, Lines by File
- Sorted by total line count (descending)

#### Markdown Report
- Automatically saved to `../Docs/rust_loc_report.md`
- Formatted with proper markdown headers and tables
- Includes generation timestamp
- Same statistical breakdown as console report

### Statistical Calculations

- **Percentage Calculations**: Shows proportion of each line type relative to total
- **Directory Aggregation**: Sums statistics for all files within each directory
- **File-Level Detail**: Individual statistics for each `.rs` file
- **Sorting**: Results sorted by total line count in descending order

## Algorithm Details

### Comment Processing Logic

The tool implements sophisticated comment handling:

```python
# Single-line comments
if line.startswith("//"):
    comment_lines += 1

# Block comment start
if line.startswith("/*"):
    comment_lines += 1
    if "*/" not in line:
        in_block_comment = True

# Block comment continuation
if in_block_comment:
    comment_lines += 1
    if "*/" in line:
        in_block_comment = False
        # Check for code after comment end
```

### Path Processing

- Uses `os.path.relpath()` for clean path display
- Normalizes paths for consistent reporting
- Handles cross-platform path separators

## Key Functions

### `count_lines_in_file(file_path)`
**Purpose**: Analyzes a single Rust file and categorizes its lines
**Returns**: Tuple of `(total_lines, code_lines, comment_lines, blank_lines)`
**Features**:
- State machine for block comment tracking
- UTF-8 encoding handling
- Exception handling for file access errors

### `count_rust_lines(directory)`
**Purpose**: Recursively processes all `.rs` files in a directory tree
**Returns**: Tuple of `(stats_by_file, stats_by_dir, total_stats)`
**Features**:
- `os.walk()` for recursive directory traversal
- Aggregation of statistics at directory and project levels
- File filtering by extension

### `generate_report(stats_by_file, stats_by_dir, total_stats)`
**Purpose**: Creates formatted console report from statistics
**Returns**: String containing the complete report
**Features**:
- Tabular formatting with fixed-width columns
- Percentage calculations
- Timestamp inclusion

### `save_report_to_markdown(report, output_path)`
**Purpose**: Converts console report to markdown format
**Features**:
- Table formatting conversion
- Header generation
- File output with UTF-8 encoding

## Usage Instructions

### Basic Usage
```bash
python3 rust_loc_counter.py
```

### Prerequisites
- Python 3.6 or higher
- Standard Python libraries: `os`, `re`, `collections`, `datetime`
- Read access to target Rust source directory

### Default Behavior
1. Scans `../src` directory relative to script location
2. Processes all `.rs` files recursively
3. Displays report to console
4. Saves markdown report to `../Docs/rust_loc_report.md`

## Output Example

### Console Output
```
================================================================================
RUST CODE LINE COUNT REPORT
Generated on: 2024-08-14 10:30:45
================================================================================

OVERALL STATISTICS:
Total files processed: 45
Total lines: 12,456
Code lines: 8,234 (66.1%)
Comment lines: 2,890 (23.2%)
Blank lines: 1,332 (10.7%)

LINES BY DIRECTORY:
Directory                                Total      Code       Comments   Blank     
--------------------------------------------------------------------------------
src/parser                              3,245      2,156      789        300      
src/interpreter                         2,890      1,945      634        311      
...
```

## Integration with WFL Project

### Project Context
- Part of the WFL (WebFirst Language) toolchain
- Located in `Tools/` directory
- Generates reports saved to `Docs/` directory
- Supports development workflow documentation

### Automation Potential
- Can be integrated into CI/CD pipelines
- Suitable for pre-commit hooks
- Supports automated documentation generation
- Compatible with build scripts

## Limitations and Considerations

### Current Limitations
- Only processes `.rs` files (Rust source files)
- Does not analyze code complexity or quality metrics
- No configuration options for customization
- Fixed output format and location

### Performance Characteristics
- Linear time complexity O(n) where n is total lines of code
- Memory usage proportional to number of files processed
- Suitable for projects with thousands of files

### Error Handling
- Continues processing if individual files fail
- Prints error messages for problematic files
- Does not halt execution on encoding issues

## Dependencies

### Required Python Modules
All dependencies are part of Python's standard library:
- `os`: File system operations
- `re`: Regular expression support (imported but not actively used)
- `collections.defaultdict`: Directory statistics aggregation
- `datetime`: Timestamp generation

### System Requirements
- Python 3.6+
- File system read permissions
- UTF-8 capable text editor for viewing reports

## Maintenance and Extension

### Extension Points
- Custom file type support by modifying file extension filter
- Additional statistical metrics in the analysis functions
- Alternative output formats (JSON, CSV, XML)
- Configuration file support for customizable behavior

### Code Quality
- Well-documented with docstrings
- Modular function design
- Error handling for robustness
- Clear separation of concerns

---

*This specification document describes the Rust Line Counter tool as implemented in `rust_loc_counter.py`. The tool is designed for analyzing WFL project Rust source code and generating comprehensive line count reports.*