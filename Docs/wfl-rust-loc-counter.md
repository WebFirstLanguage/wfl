# Rust Line of Code Counter

## Overview

The Rust Line of Code Counter is a Python utility script that analyzes the Rust codebase in the `src` directory, providing detailed metrics about code composition. This tool helps in understanding the size and structure of the codebase by counting and categorizing lines of code.

## Features

- **Comprehensive Counting**: Counts all lines in Rust (`.rs`) files within the `src` directory and its subdirectories
- **Line Categorization**: Distinguishes between:
  - Code lines
  - Comment lines (both single-line and multi-line block comments)
  - Blank lines
- **Hierarchical Analysis**: Provides statistics at multiple levels:
  - Overall project totals
  - Per-directory breakdown
  - Per-file details
- **Report Generation**: Creates both console output and a Markdown report file

## Usage

To run the line counter:

```bash
python Tools/rust_loc_counter.py
```

The script will:
1. Scan all `.rs` files in the `src` directory and its subdirectories
2. Print a summary report to the console
3. Generate a more detailed Markdown report in `Docs/rust_loc_report.md`

## Report Contents

The generated report includes:

- **Overall Statistics**:
  - Total number of files processed
  - Total lines of code
  - Breakdown of code, comment, and blank lines with percentages

- **Directory Breakdown**:
  - Line counts for each directory
  - Sorted by total line count (descending)

- **File Breakdown**:
  - Line counts for each Rust file
  - Sorted by total line count (descending)

## Implementation Details

The counter uses a state-based approach to accurately categorize lines, handling:
- Empty/blank lines
- Single-line comments (`//`)
- Block comments (`/* ... */`)
- Mixed lines (containing both code and comments)
- Nested comments

## Maintenance

When the codebase evolves, run this script periodically to track changes in the code size and composition. This can provide insights into code growth trends and help identify areas of the codebase that may need refactoring or documentation improvements.
