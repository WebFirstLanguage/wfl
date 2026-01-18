#!/bin/bash
# Auto-format Rust files after Edit/Write operations
# Cross-platform hook for Claude Code

# Read JSON from stdin
json_input=$(cat)

# Extract file_path from JSON (handles both Unix and Windows paths)
file_path=$(echo "$json_input" | grep -o '"file_path"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*"\([^"]*\)".*/\1/')

# Check if it's a Rust file
if [[ "$file_path" == *.rs ]]; then
    cargo fmt --all
fi
