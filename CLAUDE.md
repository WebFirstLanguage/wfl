# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Building and Testing
```bash
# Build debug version
cargo build

# Build release version (REQUIRED for integration tests)
cargo build --release

# Run all tests
cargo test

# Run integration tests (requires release build)
# Windows:
.\scripts\run_integration_tests.ps1
# Linux/macOS:
./scripts/run_integration_tests.sh

# Run specific integration test
cargo test --test split_functionality

# Format code
cargo fmt --all

# Run linter
cargo clippy --all-targets --all-features -- -D warnings
```

### WFL Language Commands
```bash
# Run WFL program
wfl program.wfl

# Start interactive REPL
wfl

# Lint WFL code
wfl --lint program.wfl

# Auto-fix WFL code
wfl --fix program.wfl --in-place

# Debug WFL execution
wfl --debug program.wfl

# Show tokens/AST
wfl --lex program.wfl
wfl --parse program.wfl

# Check and fix configuration
wfl --configCheck
wfl --configFix

# Run with execution timing
wfl --time program.wfl

# Run in single-step debug mode
wfl --step program.wfl

# Dump environment info
wfl --dump-env
```

### VSCode Extension
```bash
# Install WFL VSCode extension
scripts/install_vscode_extension.ps1
```

## Architecture Overview

WFL is a natural language programming language implemented in Rust with a traditional compiler pipeline enhanced for async execution.

### Core Processing Pipeline
```
Source Code → Lexer → Parser → Analyzer → Type Checker → Interpreter
              ↓       ↓         ↓           ↓              ↓
            Tokens   AST    Validated   Type Info    Execution
```

### Key Components

- **Lexer** (`src/lexer/`): High-performance tokenization using Logos crate
- **Parser** (`src/parser/`): Recursive descent parser with natural language constructs and error recovery
  - Includes specialized parsers for containers and AST generation
  - Maintains contextual keyword handling for natural language syntax
- **Analyzer** (`src/analyzer/`): Semantic validation and static analysis
- **Type Checker** (`src/typechecker/`): Static type analysis with intelligent inference
- **Interpreter** (`src/interpreter/`): Async-capable direct AST execution using Tokio runtime
  - Includes subprocess handling with security sanitization
  - Web server support with HTTP request/response handling
  - Environment management with scope control
- **Pattern Module** (`src/pattern/`): Pattern matching engine with bytecode VM
  - Compiler for pattern expressions
  - VM-based execution for regex-like patterns
  - Unicode support and advanced pattern features
- **Standard Library** (`src/stdlib/`): Built-in modules
  - Core functions (print, typeof, etc.)
  - Math operations (abs, round, random, etc.)
  - Text manipulation (length, uppercase, substring, etc.)
  - List operations (push, pop, contains, etc.)
  - Filesystem I/O with async support
  - Crypto module with WFLHASH (custom hash function)
  - Time functions
  - Random number generation
- **LSP Server** (`wfl-lsp/`): Language Server Protocol implementation for IDE integration
- **Development Tools**: Linter, code fixer, analyzer with real-time error checking
- **REPL** (`src/repl.rs`): Interactive Read-Eval-Print Loop for experimentation

### Workspace Structure
- Root crate `wfl` contains the main compiler/interpreter
- `wfl-lsp/` workspace member provides Language Server Protocol support
- `vscode-extension/` provides VSCode language support
- `TestPrograms/` contains WFL test programs that MUST all pass
- `tests/` contains Rust unit and integration tests
- `Docs/` contains all user-facing documentation
- `Dev diary/` contains development history and progress notes
- `.cursor/rules/` contains project-specific rules

## Critical Development Rules

### Test-Driven Development (MANDATORY)
**TDD is as critical as backward compatibility. Every change MUST start with a failing test.**

1. **Write failing tests FIRST** for any feature or bug fix
2. **Confirm tests fail** before writing implementation
3. **Never modify tests to make them pass** - fix the implementation instead
4. All TestPrograms/*.wfl files MUST pass after any change

### Backward Compatibility
**NEVER BREAK EXISTING WFL PROGRAMS**. WFL has a backward compatibility promise:
- All existing WFL code must continue to work
- Run ALL TestPrograms after changes
- If implementing parser features, also update bytecode

### Integration Test Requirements
Integration tests require the **release binary** (`target/release/wfl.exe` on Windows, `target/release/wfl` on Unix):
- Always run `cargo build --release` before integration tests
- Use provided scripts: `scripts/run_integration_tests.ps1` or `scripts/run_integration_tests.sh`
- If tests fail with "path not found", you need to build the release binary

### Configuration System
WFL uses `.wflcfg` files for project configuration:
- Supports execution settings (timeouts, logging)
- Code style settings (line length, indentation)
- Global config can be overridden with `WFL_GLOBAL_CONFIG_PATH`

### Key Language Features
- **Natural Language Syntax**: `store name as "value"`, `check if x is greater than 5`
- **Type Safety**: Static typing with intelligent type inference
- **Async Support**: Built-in async/await using Tokio runtime
- **Error Handling**: Comprehensive try/when/otherwise error handling
- **Standard Library**: Math, text, list, filesystem, crypto, and web modules
- **Container System**: Object-oriented programming with containers (classes)
- **Pattern Matching**: Powerful pattern matching engine with Unicode support
- **Subprocess Execution**: Secure subprocess spawning with command sanitization

### Memory and Performance
- Uses WFLHASH custom cryptographic hash function (see security reviews)
- Optional heap profiling with dhat feature flags (`dhat-heap`, `dhat-ad-hoc`)
- Async-capable interpreter for concurrent operations
- Memory optimization for large programs
- Pattern VM for efficient regex-like operations

### Documentation Standards
- All documentation in `Docs/` folder
- Update README.md with significant changes
- Component documentation required for all major modules
- Dev diary entries for significant changes in `Dev diary/`

### Cursor Rules Integration
The codebase includes Cursor IDE rules in `.cursor/rules/wfl-rules.mdc`:
- Always read README.md first
- Update documentation with changes
- All test programs must pass
- Update bytecode when modifying parser

## Coding Style & Naming

- **Format**: `cargo fmt --all` (see `.rustfmt.toml`)
- **Lint**: `cargo clippy --all-targets --all-features -- -D warnings`
- **Naming Conventions**:
    - Functions/Files: `snake_case`
    - Types/Traits: `CamelCase`
    - Constants: `SCREAMING_SNAKE_CASE`

## Commit & Pull Request Guidelines

- **Conventional Commits**: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`.
- **Pull Requests**:
    - clear description
    - linked issues
    - tests added/updated
    - repro steps for fixes
- **Pre-PR checks**:
    - `cargo fmt --all -- --check`
    - `cargo clippy --all-targets --all-features -- -D warnings`
    - `cargo test --all --verbose`

## Technical Requirements

### Rust Environment
- **Rust Edition**: 2024
- **Minimum Rust Version**: 1.75+
- **Current Development**: Rust 1.91.1+
- **Build System**: Cargo with workspace support

### Key Dependencies
- `logos`: Lexer generation
- `tokio`: Async runtime
- `reqwest`: HTTP client
- `sqlx`: Database support (SQLite, MySQL, PostgreSQL)
- `warp`: Web server framework
- `tower-lsp`: LSP server implementation (wfl-lsp)
- `codespan-reporting`: Error diagnostics
- `zeroize`, `subtle`: Cryptographic security
- `hkdf`, `sha2`: Key derivation for crypto

### Version Scheme
WFL uses calendar-based versioning: **YY.MM.BUILD**
- Example: `26.1.4`
- Major version always < 256 (Windows MSI compatibility)
- **All workspace members (wfl, wfl-lsp) share the same version**
- keep a dev diary in the dev diary directory on the project root
