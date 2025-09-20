# WFL Language Server Protocol (LSP) Guide

The WFL Language Server Protocol implementation provides rich IDE features for WFL development, including real-time diagnostics, intelligent code completion, hover information, and more. This guide covers installation, configuration, and usage of the WFL LSP server.

## Table of Contents

- [Overview](#overview)
- [Installation](#installation)
- [VS Code Integration](#vs-code-integration)
- [Features](#features)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)
- [Advanced Usage](#advanced-usage)
- [Performance](#performance)

## Overview

The WFL Language Server (`wfl-lsp`) is a standalone server that implements the Language Server Protocol (LSP) specification. It provides intelligent language features for WFL development in any LSP-compatible editor.

### Key Benefits

- **Real-time Error Detection**: Syntax and semantic errors highlighted as you type
- **Intelligent Code Completion**: Context-aware suggestions for variables, functions, and keywords
- **Rich Hover Information**: Detailed information about symbols, types, and documentation
- **Cross-Platform**: Works on Windows, macOS, and Linux
- **Editor Agnostic**: Compatible with VS Code, Vim, Emacs, and other LSP-enabled editors

### Architecture

```
Editor (VS Code, Vim, etc.)
    ↕ LSP Protocol (JSON-RPC)
WFL Language Server (wfl-lsp)
    ↕ Direct Integration
WFL Compiler Components
    ├── Lexer (Tokenization)
    ├── Parser (AST Generation)
    ├── Analyzer (Semantic Analysis)
    └── Type Checker (Type Validation)
```

## Installation

### Prerequisites

- WFL compiler installed and accessible in PATH
- Rust toolchain (for building from source)

### Option 1: Pre-built Binaries

Download the latest release from the WFL releases page:

```bash
# Windows
curl -L -o wfl-lsp.exe https://github.com/WebFirstLanguage/wfl/releases/latest/download/wfl-lsp-windows.exe

# macOS
curl -L -o wfl-lsp https://github.com/WebFirstLanguage/wfl/releases/latest/download/wfl-lsp-macos
chmod +x wfl-lsp

# Linux
curl -L -o wfl-lsp https://github.com/WebFirstLanguage/wfl/releases/latest/download/wfl-lsp-linux
chmod +x wfl-lsp
```

### Option 2: Build from Source

```bash
# Clone the WFL repository
git clone https://github.com/WebFirstLanguage/wfl.git
cd wfl

# Build the LSP server
cargo build --release --package wfl-lsp

# The binary will be at target/release/wfl-lsp (or wfl-lsp.exe on Windows)
```

### Option 3: Install via Cargo

```bash
cargo install --path wfl-lsp
```

### Verify Installation

```bash
wfl-lsp --version
# Should output: wfl-lsp 0.1.0
```

## VS Code Integration

### Automatic Setup

1. **Install the WFL Extension**: Search for "WFL" in the VS Code Extensions marketplace
2. **Install WFL LSP**: Follow the installation steps above
3. **Configure Path**: The extension will auto-detect `wfl-lsp` if it's in your PATH

### Manual Configuration

If auto-detection doesn't work, configure the LSP server path manually:

1. Open VS Code Settings (`Ctrl+,` or `Cmd+,`)
2. Search for "wfl.serverPath"
3. Set the path to your `wfl-lsp` executable:

```json
{
    "wfl.serverPath": "/path/to/wfl-lsp",
    "wfl.serverArgs": ["--log-level", "info"]
}
```

### Verify VS Code Integration

1. Open a `.wfl` file
2. Check the status bar for "WFL LSP" indicator
3. Look for syntax highlighting and error diagnostics
4. Try code completion with `Ctrl+Space`

## Features

### 1. Real-time Diagnostics

The LSP server provides immediate feedback on code issues:

**Syntax Errors**:
```wfl
store x as  // Missing value - shows red underline
display x
```

**Semantic Errors**:
```wfl
display undefined_variable  // Undefined variable - shows red underline
```

**Type Errors**:
```wfl
store x as 5
store y as "hello"
store result as x + y  // Type mismatch - shows red underline
```

### 2. Intelligent Code Completion

Context-aware suggestions based on your code:

**Variable Completion**:
```wfl
store myVariable as 42
store myList as [1, 2, 3]

display my|  // Suggests: myVariable, myList
```

**Function Completion**:
```wfl
define action calculateSum with parameters x, y
    return x + y
end action

store result as calc|  // Suggests: calculateSum
```

**Standard Library Completion**:
```wfl
store text as "Hello World"
display length of |  // Suggests: text, and other variables
display |  // Suggests: length of, uppercase, lowercase, etc.
```

**Context-Aware Completion**:
```wfl
if |  // Suggests: variables, comparison operators
store x as |  // Suggests: values, expressions
count from i as 1 to |  // Suggests: numbers, variables
```

### 3. Rich Hover Information

Hover over symbols to see detailed information:

**Variables**:
```wfl
store userName as "Alice"  // Hover shows: Variable 'userName' of type 'text' with value "Alice"
```

**Functions**:
```wfl
define action greet with parameters name
    display "Hello, " + name
end action

greet("Bob")  // Hover on 'greet' shows: Function 'greet(name: text) -> null'
```

**Keywords**:
```wfl
if condition  // Hover on 'if' shows: Conditional statement for branching logic
```

**Standard Library Functions**:
```wfl
length of myList  // Hover shows: Returns the number of elements in a list or characters in text
```

### 4. Error Recovery

The LSP server gracefully handles incomplete or malformed code:

```wfl
store x as
if x > 5
    display "Large"
// Missing 'end if' - LSP continues to provide features for the rest of the file
```

## Configuration

### LSP Server Settings

Configure the LSP server behavior through your editor's settings:

**VS Code Settings** (`settings.json`):
```json
{
    "wfl.serverPath": "/usr/local/bin/wfl-lsp",
    "wfl.serverArgs": [
        "--log-level", "info",
        "--max-completion-items", "50"
    ],
    "wfl.versionMode": "warn"
}
```

### Command Line Options

The LSP server supports various command-line options:

```bash
wfl-lsp --help

Options:
  --log-level <LEVEL>           Set logging level [default: warn] [possible values: error, warn, info, debug, trace]
  --max-completion-items <NUM>  Maximum completion items to return [default: 100]
  --hover-timeout <MS>          Timeout for hover requests in milliseconds [default: 1000]
  --stdio                       Use stdio for communication (default)
  --tcp <PORT>                  Use TCP on specified port
  --version                     Show version information
  --help                        Show this help message
```

### Performance Tuning

For large projects, you can tune performance:

```json
{
    "wfl.serverArgs": [
        "--max-completion-items", "25",
        "--hover-timeout", "500"
    ]
}
```

## Troubleshooting

### Common Issues

#### 1. LSP Server Not Starting

**Symptoms**: No syntax highlighting, no diagnostics, no completion

**Solutions**:
1. Verify `wfl-lsp` is installed and in PATH:
   ```bash
   wfl-lsp --version
   ```

2. Check VS Code settings for correct server path:
   ```json
   {
       "wfl.serverPath": "/correct/path/to/wfl-lsp"
   }
   ```

3. Check the WFL output channel in VS Code for error messages

#### 2. Slow Performance

**Symptoms**: Delayed completion, slow hover responses

**Solutions**:
1. Reduce completion items:
   ```json
   {
       "wfl.serverArgs": ["--max-completion-items", "25"]
   }
   ```

2. Increase hover timeout for complex expressions:
   ```json
   {
       "wfl.serverArgs": ["--hover-timeout", "2000"]
   }
   ```

#### 3. Incomplete Diagnostics

**Symptoms**: Some errors not highlighted

**Solutions**:
1. Ensure the WFL file has valid syntax structure
2. Check that the file is saved (LSP analyzes saved content)
3. Try restarting the LSP server: `Ctrl+Shift+P` → "WFL: Restart Language Server"

#### 4. Missing Completions

**Symptoms**: Expected completions don't appear

**Solutions**:
1. Ensure variables/functions are defined before use
2. Check that the cursor is in a valid completion context
3. Try triggering completion manually with `Ctrl+Space`

### Debug Mode

Enable debug logging for detailed troubleshooting:

```json
{
    "wfl.serverArgs": ["--log-level", "debug"]
}
```

Check the WFL output channel in VS Code for detailed logs.

### Log Files

LSP server logs are available in:
- **VS Code**: Output panel → WFL channel
- **Command Line**: stderr when running `wfl-lsp` directly

## Advanced Usage

### Custom Editor Integration

For editors other than VS Code, configure LSP client settings:

**Vim with coc.nvim** (`.vim/coc-settings.json`):
```json
{
    "languageserver": {
        "wfl": {
            "command": "wfl-lsp",
            "args": ["--stdio"],
            "filetypes": ["wfl"],
            "rootPatterns": ["*.wfl", ".git"]
        }
    }
}
```

**Emacs with lsp-mode**:
```elisp
(add-to-list 'lsp-language-id-configuration '(wfl-mode . "wfl"))
(lsp-register-client
 (make-lsp-client :new-connection (lsp-stdio-connection "wfl-lsp")
                  :major-modes '(wfl-mode)
                  :server-id 'wfl-lsp))
```

### TCP Mode

For remote development or debugging:

```bash
# Start LSP server on TCP port 8080
wfl-lsp --tcp 8080

# Configure editor to connect to TCP server
```

### Multiple Projects

The LSP server handles multiple WFL projects simultaneously. Each project is isolated with its own:
- Symbol table
- Diagnostic context
- Completion scope

## Performance

### Benchmarks

The WFL LSP server is optimized for performance:

- **Startup Time**: < 100ms
- **Completion Response**: < 50ms for typical files
- **Hover Response**: < 100ms for complex expressions
- **Diagnostic Update**: < 200ms for files up to 1000 lines

### Memory Usage

- **Base Memory**: ~10MB
- **Per File**: ~1MB for typical WFL files
- **Large Files**: Scales linearly with file size

### Optimization Tips

1. **Keep files modular**: Smaller files provide faster response times
2. **Use meaningful names**: Helps with completion accuracy
3. **Regular saves**: LSP analyzes saved content for best results
4. **Close unused files**: Reduces memory usage in large projects

## LSP Protocol Features

The WFL LSP server implements the following LSP protocol features:

### Core Features

| Feature | Status | Description |
|---------|--------|-------------|
| `textDocument/didOpen` | ✅ | Document opened notification |
| `textDocument/didChange` | ✅ | Document content changed |
| `textDocument/didSave` | ✅ | Document saved notification |
| `textDocument/didClose` | ✅ | Document closed notification |
| `textDocument/publishDiagnostics` | ✅ | Error and warning reporting |

### Language Features

| Feature | Status | Description |
|---------|--------|-------------|
| `textDocument/completion` | ✅ | Code completion suggestions |
| `textDocument/hover` | ✅ | Symbol information on hover |
| `textDocument/signatureHelp` | 🚧 | Function signature help (planned) |
| `textDocument/definition` | 🚧 | Go to definition (planned) |
| `textDocument/references` | 🚧 | Find all references (planned) |
| `textDocument/documentSymbol` | 🚧 | Document outline (planned) |
| `textDocument/formatting` | 🚧 | Document formatting (planned) |
| `textDocument/rangeFormatting` | 🚧 | Range formatting (planned) |

### Workspace Features

| Feature | Status | Description |
|---------|--------|-------------|
| `workspace/didChangeConfiguration` | ✅ | Configuration change handling |
| `workspace/didChangeWatchedFiles` | 🚧 | File system watching (planned) |
| `workspace/symbol` | 🚧 | Workspace-wide symbol search (planned) |

## Code Examples

### Basic WFL Program with LSP Features

```wfl
// LSP provides syntax highlighting and error checking
store userName as "Alice"
store userAge as 25
store isActive as true

// Hover over 'greetUser' shows function signature
define action greetUser with parameters name, age
    // Completion suggests 'name' and 'age' parameters
    display "Hello, " + name + "! You are " + age + " years old."

    // Type checking ensures correct types
    if age >= 18
        display "You are an adult."
    else
        display "You are a minor."
    end if
end action

// Completion suggests 'greetUser' function
// Hover shows parameter information
greetUser(userName, userAge)

// Error: undefined variable (red underline)
display undefinedVariable

// Error: type mismatch (red underline)
store result as userName + userAge
```

### Standard Library Integration

```wfl
// LSP knows about all standard library functions
store myList as [1, 2, 3, 4, 5]
store myText as "Hello, World!"

// Completion suggests standard library functions
display length of myList        // Hover: Returns number of elements
display first of myList         // Hover: Returns first element
display last of myList          // Hover: Returns last element

// Text functions with completion and hover
display uppercase myText        // Hover: Converts text to uppercase
display lowercase myText        // Hover: Converts text to lowercase
display length of myText        // Hover: Returns number of characters

// Math functions
store numbers as [10, 20, 30]
display sum of numbers          // Hover: Calculates sum of numeric list
display average of numbers      // Hover: Calculates arithmetic mean
display maximum of numbers      // Hover: Returns largest value
```

### Error Handling with LSP

```wfl
// LSP provides real-time error detection
try
    store result as divide(10, 0)  // Potential runtime error
    display result
catch error
    // Completion suggests error handling keywords
    display "Error occurred: " + error

    // LSP validates error handling syntax
    if error contains "division"
        display "Cannot divide by zero"
    end if
end try
```

## Integration Examples

### VS Code Workspace Configuration

Create a `.vscode/settings.json` file in your WFL project:

```json
{
    "wfl.serverPath": "./target/release/wfl-lsp",
    "wfl.serverArgs": [
        "--log-level", "info",
        "--max-completion-items", "50"
    ],
    "wfl.versionMode": "warn",
    "files.associations": {
        "*.wfl": "wfl"
    },
    "editor.formatOnSave": true,
    "editor.formatOnType": true,
    "[wfl]": {
        "editor.tabSize": 4,
        "editor.insertSpaces": true,
        "editor.wordWrap": "on"
    }
}
```

### Multi-root Workspace

For projects with multiple WFL modules:

```json
{
    "folders": [
        {
            "name": "Core Module",
            "path": "./core"
        },
        {
            "name": "Web Module",
            "path": "./web"
        },
        {
            "name": "Utils Module",
            "path": "./utils"
        }
    ],
    "settings": {
        "wfl.serverPath": "../tools/wfl-lsp",
        "wfl.serverArgs": ["--log-level", "debug"]
    }
}
```

## Testing LSP Features

### Manual Testing

1. **Completion Testing**:
   ```wfl
   store test|  // Type 'test' and press Ctrl+Space
   ```

2. **Hover Testing**:
   ```wfl
   store myVar as 42
   display myVar  // Hover over 'myVar'
   ```

3. **Diagnostics Testing**:
   ```wfl
   store x as  // Missing value - should show error
   ```

### Automated Testing

The WFL LSP server includes comprehensive test suites:

```bash
# Run all LSP tests
cargo test --package wfl-lsp

# Run specific test categories
cargo test --package wfl-lsp --test lsp_completion_test
cargo test --package wfl-lsp --test lsp_hover_test
cargo test --package wfl-lsp --test lsp_diagnostics_test
cargo test --package wfl-lsp --test lsp_performance_stability_test
```

## Contributing to LSP Development

### Development Setup

1. **Clone Repository**:
   ```bash
   git clone https://github.com/WebFirstLanguage/wfl.git
   cd wfl
   ```

2. **Build LSP Server**:
   ```bash
   cargo build --package wfl-lsp
   ```

3. **Run Tests**:
   ```bash
   cargo test --package wfl-lsp
   ```

4. **Test with VS Code**:
   ```bash
   # Install extension in development mode
   cd vscode-extension
   npm install
   npm run compile
   code --install-extension vscode-wfl-0.1.0.vsix
   ```

### Adding New Features

1. **Implement LSP Handler**: Add new request handlers in `wfl-lsp/src/lib.rs`
2. **Add Tests**: Create comprehensive tests in `wfl-lsp/tests/`
3. **Update Documentation**: Update this guide and VS Code extension README
4. **Test Integration**: Verify features work in VS Code and other editors

### Code Architecture

```
wfl-lsp/
├── src/
│   ├── lib.rs          # Main LSP server implementation
│   └── main.rs         # CLI entry point
└── tests/
    ├── lsp_completion_test.rs           # Completion feature tests
    ├── lsp_hover_test.rs                # Hover feature tests
    ├── lsp_diagnostics_test.rs          # Diagnostics tests
    ├── lsp_performance_stability_test.rs # Performance tests
    └── lsp_end_to_end_validation_test.rs # Integration tests
```

---

## Appendix

### LSP Specification Compliance

The WFL LSP server follows the [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/) version 3.17.

### Supported File Extensions

- `.wfl` - WFL source files
- `.wflcfg` - WFL configuration files (limited support)

### Version Compatibility

| WFL Version | LSP Version | Compatibility |
|-------------|-------------|---------------|
| 25.9.x      | 0.1.0       | ✅ Full       |
| 25.8.x      | 0.1.0       | ✅ Full       |
| 25.7.x      | 0.1.0       | ⚠️ Limited    |

---

For more information, see:
- [WFL Language Reference](../language-reference/wfl-spec.md)
- [VS Code Extension Guide](../../vscode-extension/README.md)
- [WFL Getting Started](wfl-getting-started.md)
- [WFL Architecture](../technical/wfl-architecture-diagram.md)
