# WFL (WebFirst Language)

<div align="center">
  <img src="https://img.shields.io/badge/version-2025.40-blue" alt="Version">
  <img src="https://img.shields.io/badge/status-alpha-orange" alt="Status">
  <img src="https://img.shields.io/badge/license-Apache--2.0-green" alt="License">
  <img src="https://img.shields.io/badge/rust-1.75+-brown" alt="Rust Version">
</div>

<div align="center">
  <h3>Programming that reads like plain English</h3>
  <p><em>Bridge the gap between natural language and code</em></p>
</div>

---

## ⚠️ Alpha Software Notice

**This software is in alpha stage and should not be used in production environments.** We're actively developing and improving WFL. Your feedback and contributions are welcome!

## 🎯 What is WFL?

WFL (WebFirst Language) is a programming language designed to make coding more intuitive and accessible. Instead of abstract symbols and cryptic syntax, WFL uses natural English-like constructs that anyone can read and understand.

```wfl
store greeting as "Hello, World!"
display greeting

check if 5 is greater than 3:
    display "Math works!"
otherwise:
    display "Something is wrong with the universe."
end check

count from 1 to 5:
    display "Counting: " with the current count
end count
```

## ✨ Key Features

- **📖 Natural Language Syntax**: Write code that reads like English sentences
- **🚀 Modern Async Support**: Built-in async/await for concurrent operations
- **🛡️ Type Safety**: Static type checking with intelligent inference
- **🌐 Web-First Design**: Native HTTP and database support
- **🎨 Developer Experience**: Comprehensive tooling and real-time error checking
- **♻️ Backward Compatibility**: Your code will always work with future versions

## 🚀 Quick Start

### Prerequisites

- Rust 1.75 or later
- Cargo (comes with Rust)

### Installation

```bash
# Clone the repository
git clone https://github.com/logbie/wfl.git
cd wfl

# Build the project
cargo build --release

# Add to PATH (optional)
export PATH="$PATH:$(pwd)/target/release"
```

### Your First Program

Create a file called `hello.wfl`:

```wfl
store name as "Developer"
display "Welcome to WFL, " with name with "!"

// Using async operations
try:
    wait for open url "https://api.github.com/zen" and read response
    display "GitHub says: " with response
when error:
    display "Could not fetch wisdom from GitHub"
end try
```

Run it:

```bash
wfl hello.wfl
```

## 🛠️ Development Tools

WFL comes with a comprehensive suite of development tools:

### 🔍 Code Quality

```bash
# Check code style and conventions
wfl --lint your_program.wfl

# Perform static analysis
wfl --analyze your_program.wfl

# Auto-format and fix code
wfl --fix your_program.wfl --in-place

# View changes before applying
wfl --fix your_program.wfl --diff
```

### 🐛 Debugging

```bash
# Run with debug output
wfl --debug your_program.wfl > debug.txt 2>&1

# Show execution timing
wfl --time your_program.wfl

# Output tokens or AST
wfl --lex your_program.wfl
wfl --parse your_program.wfl
```

### 📝 Editor Support

WFL includes a Language Server Protocol (LSP) implementation for real-time error checking and auto-completion in your favorite editor.

**VSCode Extension**:
```bash
# Install the VSCode extension
scripts/install_vscode_extension.ps1
```

Features:
- Syntax highlighting
- Real-time error checking
- Auto-completion
- Go-to definition
- Hover documentation

## 📚 Language Overview

### Variables and Types

```wfl
// Simple variable assignment
store age as 25
store name as "Alice"
store pi as 3.14159
store is active as true
store items as [1, 2, 3, 4, 5]

// Type inference
display typeof of age       // "number"
display typeof of name      // "text"
display typeof of items     // "list"
```

### Control Flow

```wfl
// Conditional statements
check if age is greater than 18:
    display "You can vote!"
otherwise check if age is 18:
    display "You just became eligible to vote!"
otherwise:
    display "You'll be able to vote in the future"
end check

// Loops
count from 1 to 10:
    display "Number: " with the current count
end count

for each item in items:
    display "Processing: " with item
end for
```

### Actions (Functions)

```wfl
action greet with name:
    display "Hello, " with name with "!"
end action

action calculate area with width and height:
    store result as width times height
    return result
end action

// Using actions
call greet with "World"
store room area as calculate area with 10 and 20
```

### Async Operations

```wfl
// Concurrent web requests
wait for:
    open url "https://api.example.com/data1" and read response1
    open url "https://api.example.com/data2" and read response2
end wait

display "Got both responses!"

// File operations
wait for open file "data.txt" and read contents
display "File contents: " with contents
```

### Error Handling

```wfl
try:
    store result as risky operation()
    display "Success: " with result
when error:
    display "An error occurred: " with error message
otherwise:
    display "Operation completed"
end try
```

## 🔧 Tools

### WFL Combiner

WFL includes a powerful file combiner tool for merging markdown and source files with table of contents generation:

```bash
# Combine markdown files with table of contents
wfl run tools/combiner.wfl --input Docs/ --type docs --output combined/docs.md

# Combine Rust source files
wfl run tools/combiner.wfl --input src/ --type src --output combined/src.md

# Available options:
# --input DIR     Input directory to search
# --type TYPE     File type: 'docs' (.md) or 'src' (.rs)  
# --output FILE   Output file path
# --no-toc        Disable table of contents generation
# --no-txt        Skip .txt file generation
# --help          Show detailed help
```

**⚠️ Deprecation Notice**: The legacy Python combiner (`Tools/wfl_md_combiner.py`) is deprecated and will be removed in the next release cycle. Please migrate to the WFL version above.

## 📦 Standard Library

WFL includes a comprehensive standard library:

### Core Functions
- `print(text)` - Output text
- `typeof(value)` - Get type of value
- `isnothing(value)` - Check if value is null

### Math Module
- `abs(number)` - Absolute value
- `round(number)` - Round to nearest integer
- `floor(number)` - Round down
- `ceil(number)` - Round up
- `random()` - Random number 0-1
- `clamp(value, min, max)` - Constrain value

### Text Module
- `length(text)` - Get text length
- `touppercase(text)` - Convert to uppercase
- `tolowercase(text)` - Convert to lowercase
- `contains(text, search)` - Check if contains
- `substring(text, start, end)` - Extract substring

### List Module
- `length(list)` - Get list size
- `push(list, item)` - Add item to end
- `pop(list)` - Remove last item
- `contains(list, item)` - Check if contains
- `indexof(list, item)` - Find item position

### Time Module
- `time.now()` - Current timestamp
- `time.sleep(seconds)` - Pause execution
- `time.format(timestamp, format)` - Format time

### I/O Module

The WFL standard library includes comprehensive file and directory operations:

#### Directory Operations
- `walk_dir(path, recursive)` - Walk directory tree with optional recursion
- `list_files(path)` - List files in directory
- `create_dir(path)` - Create directory

#### File Operations  
- `read_file(path)` - Read file contents as text
- `write_file(path, content)` - Write text to file
- `file_exists(path)` - Check if file exists
- `file_size(path)` - Get file size in bytes
- `file_modified_time(path)` - Get last modification time

#### Glob Pattern Matching
- `glob_match(pattern, text)` - Match glob patterns
- Supported patterns: `*.md`, `**/*.rs`, `src/**/*.wfl`
- Character classes: `[abc]`, `[a-z]`, `[!abc]`
- Wildcards: `*` (any chars), `?` (single char)

#### CLI Argument Parsing
- `parse_cli_flags(args)` - Parse command-line arguments
- Supports: `--flag`, `--key=value`, `--key value`
- Boolean flags and value parameters
- Automatic help text generation

## ⚙️ Configuration

Create a `.wflcfg` file in your project directory:

```ini
# Execution settings
timeout_seconds = 60
logging_enabled = false
debug_report_enabled = true
log_level = info

# Code style settings
max_line_length = 100
max_nesting_depth = 5
indent_size = 4
snake_case_variables = true
trailing_whitespace = false
consistent_keyword_case = true
```

Validate configuration:
```bash
# Check for issues
wfl --configCheck

# Auto-fix problems
wfl --configFix
```

## 🏗️ Architecture

WFL follows a traditional compiler architecture with modern enhancements:

```
Source Code → Lexer → Parser → Analyzer → Type Checker → Interpreter
                ↓       ↓         ↓           ↓              ↓
              Tokens   AST    Validated   Type Info    Execution
```

Key components:
- **Lexer**: High-performance tokenization with Logos
- **Parser**: Recursive descent with error recovery
- **Analyzer**: Semantic validation and optimization
- **Type Checker**: Static analysis with inference
- **Interpreter**: Async-capable direct AST execution
- **LSP Server**: Real-time IDE integration

## 🤝 Contributing

We welcome contributions! Here's how to get started:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Ensure all test programs pass (`cargo run -- TestPrograms/*.wfl`)
6. Commit your changes
7. Push to your branch
8. Open a Pull Request

### Development Guidelines

- Maintain backward compatibility
- Add tests for new features
- Update documentation
- Follow the existing code style
- Create Dev Diary entries for significant changes

## 📊 Project Status

| Component | Status | Description |
|-----------|--------|-------------|
| Lexer | ✅ Complete | Fast tokenization with Logos |
| Parser | ✅ Complete | Robust parsing with error recovery |
| Type Checker | ✅ Complete | Static type analysis |
| Interpreter | ✅ Complete | Async-capable execution |
| Standard Library | ✅ Complete | Core functionality implemented |
| LSP Server | ✅ Complete | Full IDE integration |
| Error Reporting | ✅ Complete | User-friendly diagnostics |
| Code Quality Tools | ✅ Complete | Linter, analyzer, formatter |
| Bytecode VM | 🔄 Planned | Performance optimization |

## 🛡️ Our Commitment

**Backward Compatibility Promise**: We guarantee that WFL code you write today will continue to work with all future versions of the language. The interpreter adapts to your code, not the other way around.

Let's be a little more reasonable: we will not actively kill features unless a security bug forces our hand.

If we are forced to deprecate something, we will give you at least 1 year notice before we remove it.

## 📄 License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

WFL is developed by Logbie LLC with assistance from:
- Devin.ai - Primary AI development partner
- ChatGPT (GPT-o3) - Code review and optimization
- Claude (via Cline) - Documentation and architecture
- The Rust community for excellent libraries and tools

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/logbie/wfl/issues)
- **Discussions**: [GitHub Discussions](https://github.com/logbie/wfl/discussions)
- **Email**: info@logbie.com

---

<div align="center">
  <p><strong>Make programming accessible to everyone</strong></p>
  <p>© 2025 Logbie LLC</p>
</div>
