# WFL LSP Quick Reference

A quick reference guide for using the WFL Language Server Protocol features.

## Installation

```bash
# Build from source
cargo build --release --package wfl-lsp

# Verify installation
wfl-lsp --version
```

## VS Code Setup

1. Install WFL extension from marketplace
2. Configure LSP server path in settings:
   ```json
   {
       "wfl.serverPath": "/path/to/wfl-lsp"
   }
   ```

## Key Features

### 🔍 **Diagnostics** (Real-time Error Detection)

| Error Type | Example | LSP Response |
|------------|---------|--------------|
| Syntax Error | `store x as` | Red underline, "Expected value after 'as'" |
| Undefined Variable | `display unknownVar` | Red underline, "Variable 'unknownVar' not found" |
| Type Mismatch | `store x as 5 + "text"` | Red underline, "Cannot add number and text" |

### ⚡ **Code Completion** (Ctrl+Space)

| Context | Trigger | Suggestions |
|---------|---------|-------------|
| Variable Names | `my\|` | `myVariable`, `myList`, `myFunction` |
| Keywords | `if \|` | `condition`, variables, expressions |
| Standard Library | `length of \|` | Available variables and lists |
| Functions | `calc\|` | User-defined functions starting with "calc" |

### 💡 **Hover Information** (Mouse Hover)

| Symbol Type | Hover Content |
|-------------|---------------|
| Variables | `Variable 'userName' of type 'text' with value "Alice"` |
| Functions | `Function 'greet(name: text) -> null'` |
| Keywords | `Conditional statement for branching logic` |
| Stdlib Functions | `Returns the number of elements in a list` |

## Common Keyboard Shortcuts

| Action | VS Code | Description |
|--------|---------|-------------|
| Trigger Completion | `Ctrl+Space` | Show completion suggestions |
| Show Hover | `Ctrl+K Ctrl+I` | Display hover information |
| Go to Definition | `F12` | Navigate to symbol definition (planned) |
| Find References | `Shift+F12` | Find all symbol references (planned) |
| Format Document | `Shift+Alt+F` | Format entire document (planned) |
| Restart LSP | `Ctrl+Shift+P` → "WFL: Restart Language Server" | Restart LSP server |

## Standard Library Completions

### Text Functions
- `length of` - Get text/list length
- `uppercase` - Convert to uppercase
- `lowercase` - Convert to lowercase
- `substring` - Extract substring
- `contains` - Check if text contains substring

### List Functions
- `first of` - Get first element
- `last of` - Get last element
- `length of` - Get list length
- `sum of` - Calculate sum (numeric lists)
- `average of` - Calculate average
- `maximum of` - Find maximum value
- `minimum of` - Find minimum value

### Math Functions
- `random number` - Generate random number
- `round` - Round to nearest integer
- `floor` - Round down
- `ceiling` - Round up
- `absolute` - Absolute value

## Context-Aware Completion

### After Keywords

```wfl
if |           # Suggests: variables, expressions, comparisons
store x as |   # Suggests: values, expressions, function calls
count from |   # Suggests: variable names, numbers
```

### In Expressions

```wfl
display |      # Suggests: variables, functions, literals
x + |          # Suggests: numbers, numeric variables
"text" + |     # Suggests: text variables, text literals
```

### Function Definitions

```wfl
define action myFunc with parameters |  # Suggests: parameter names
    return |   # Suggests: expressions, variables
end action
```

## Troubleshooting Quick Fixes

| Problem | Quick Fix |
|---------|-----------|
| No completions | Press `Ctrl+Space` to trigger manually |
| No hover info | Ensure cursor is over a valid symbol |
| No diagnostics | Save the file (`Ctrl+S`) |
| LSP not working | Check WFL output channel for errors |
| Slow performance | Reduce `--max-completion-items` in settings |

## Configuration Examples

### Basic Configuration
```json
{
    "wfl.serverPath": "wfl-lsp",
    "wfl.serverArgs": ["--log-level", "info"]
}
```

### Performance Optimized
```json
{
    "wfl.serverPath": "wfl-lsp",
    "wfl.serverArgs": [
        "--max-completion-items", "25",
        "--hover-timeout", "500"
    ]
}
```

### Debug Mode
```json
{
    "wfl.serverPath": "wfl-lsp",
    "wfl.serverArgs": ["--log-level", "debug"]
}
```

## Example WFL Code with LSP Features

```wfl
// Variables with type inference and hover info
store userName as "Alice"        // Hover: Variable 'userName' of type 'text'
store userAge as 25             // Hover: Variable 'userAge' of type 'number'
store isActive as true          // Hover: Variable 'isActive' of type 'boolean'

// Function with completion and hover
define action greetUser with parameters name, age
    // Completion suggests 'name' and 'age' parameters
    display "Hello, " + name + "!"
    
    // Type checking and diagnostics
    if age >= 18                // Hover on 'if': Conditional statement
        display "Adult user"
    else
        display "Minor user"
    end if
end action

// Function call with completion
greetUser(userName, userAge)    // Completion suggests 'greetUser'

// Standard library with hover info
store myList as [1, 2, 3, 4, 5]
display length of myList        // Hover: Returns number of elements
display first of myList         // Hover: Returns first element
display sum of myList           // Hover: Calculates sum of numeric list

// Error detection (red underlines)
display undefinedVar            // Error: Variable not found
store result as 5 + "text"      // Error: Type mismatch
```

## Command Line Usage

```bash
# Start LSP server (stdio mode)
wfl-lsp --stdio

# Start with debug logging
wfl-lsp --log-level debug

# Start on TCP port (for remote development)
wfl-lsp --tcp 8080

# Show help
wfl-lsp --help
```

## Testing LSP Features

```bash
# Run all LSP tests
cargo test --package wfl-lsp

# Test specific features
cargo test --package wfl-lsp completion
cargo test --package wfl-lsp hover
cargo test --package wfl-lsp diagnostics
```

---

For detailed information, see the [Complete WFL LSP Guide](wfl-lsp-guide.md).
