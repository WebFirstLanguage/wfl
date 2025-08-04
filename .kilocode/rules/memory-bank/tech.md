# WFL Technology Stack

## Core Technologies

### Programming Languages
- **Rust**: Primary implementation language, chosen for safety, performance, and modern language features
- **TypeScript**: Used for VSCode extension development
- **WFL**: The language itself (used for testing and examples)

### Core Libraries
- **Logos**: High-performance lexical analyzer that powers the tokenization process
- **Tokio**: Asynchronous runtime that enables non-blocking I/O operations
- **Reqwest**: HTTP client library for network operations
- **SQLx**: Database connectivity supporting SQLite, MySQL, and PostgreSQL
- **codespan-reporting**: Professional error message formatting with source context
- **Rustyline**: Interactive REPL with history and editing capabilities

### AI Integration
- **Claude**: AI assistant for code development and review via GitHub Actions
- **Gemini**: AI research capabilities for deep technical research
- **Memory Bank**: System for AI context preservation in `.kilocode/rules/memory-bank/`

## Development Environment

### Requirements
- **Rust Toolchain**: Latest stable version
- **Cargo**: Package manager and build tool (comes with Rust)
- **Git**: Version control system
- **Visual Studio Code**: Recommended IDE with WFL extension support

### Development Setup
1. Clone the repository: `git clone https://github.com/logbie/wfl.git`
2. Install Rust (latest stable): `rustup update stable`
3. Build the project: `cargo build`
4. Run the project: `cargo run -- [flags] [file]` (use this instead of `wfl` during development)
5. Run tests: `cargo test`
6. Install VSCode extension: `scripts/install_vscode_extension.ps1`

## Build System

### Build Configurations
- **Debug**: `cargo build` - Includes debug symbols and assertions
- **Release**: `cargo build --release` - Optimized for performance
- **Test**: `cargo test` - Runs all unit and integration tests

### Cross-Platform Support
- **Windows**: Primary development platform with MSI installer support
- **Linux**: Supported with deb packages and tar.gz archives
- **macOS**: Supported with pkg installer

### Automated Builds
- Skip-if-unchanged logic to avoid unnecessary builds
- Nightly build pipeline for continuous integration
- Version management through `scripts/bump_version.py`
- GitHub Actions workflows for CI/CD

## Testing Framework

### Test Types
- **Unit Tests**: Located alongside source code
- **Integration Tests**: Located in `tests/` directory
- **Memory Tests**: Specialized tests for memory leak detection
- **Snapshot Tests**: Tests for diagnostics and error reporting
- **File I/O Tests**: Comprehensive tests for file operations

### Test Tools
- **Rust's built-in testing framework**: `cargo test`
- **DHAT**: Heap profiling via `dhat-heap` feature
- **Custom scripts**: `scripts/run_wfl_tests.sh`

## Deployment and Packaging

### Release Formats
- **Windows MSI**: Created via `Tools/launch_msi_build.py`
- **Debian Package**: Created via `cargo deb`
- **Portable Binary**: Created via standard release build

### Deployment Process
1. Bump version numbers
2. Run integration test suite
3. Build platform-specific installers
4. Generate documentation updates
5. Create release packages

### CI/CD Pipeline
- **Nightly Builds**: Automated builds triggered daily at 05:00 UTC
- **Skip-if-unchanged**: Avoids rebuilding when no code changes are detected
- **Smoke Tests**: Verifies installer functionality
- **Release Creation**: Automatically creates GitHub releases for nightly builds

## Technical Constraints

### Memory Management
- Careful handling of environment references to avoid leaks
- Efficient string interning for token storage
- Weak references for parent environments in closures

### Performance Considerations
- Efficient parsing with Logos-based lexer
- Memory-optimized AST representation
- Append-mode file operations instead of read-modify-write

### Compatibility Requirements
- Support for older Rust compiler versions
- Cross-platform filesystem paths
- Unicode support for source code

## Development Tools

### CLI Developer Tools
- **--lex**: Dump lexer tokens to `<name>.lex.txt` - Great for debugging tokenization and encoding issues
- **--ast**: Dump AST to `<name>.ast.txt` - Useful for visualizing program structure and verifying parser correctness
- **--lint**: Style and structural code quality checks
- **--fix**: Auto-apply linter suggestions (can be combined with --in-place or --diff)
- **--analyze**: Static semantic analysis to detect unused variables, unreachable code, etc.
- **--step**: Interactive step-by-step execution for debugging
- **--configCheck**: Validate `.wflcfg` files for correctness
- **--configFix**: Auto-repair common configuration issues

**Note**: During development, use `cargo run -- [flags]` instead of just `wfl [flags]`. For example:
- `cargo run -- --lex script.wfl` instead of `wfl --lex script.wfl`
- `cargo run -- --analyze script.wfl` instead of `wfl --analyze script.wfl`

### LSP Server
- **Purpose**: IDE integration
- **Implementation**: Custom Language Server Protocol server
- **Features**: Diagnostics, auto-completion, hover information

### VSCode Extension
- **Consolidated Architecture**: Merging of JavaScript and TypeScript implementations
- **Technologies**:
  - TypeScript for extension logic
  - TextMate grammar for syntax highlighting
  - VS Code's Language Client API for LSP integration
  - Custom formatters that work with or without WFL installed
- **Components**:
  - TextMate grammar (`syntaxes/wfl.tmLanguage.json`)
  - Language configuration (`language-configuration.json`)
  - Independent formatter (`src/formatting/base-formatter.ts`)
  - WFL CLI-based formatter (`src/formatting/wfl-formatter.ts`)
  - LSP client integration (`src/extension.ts`)
- **Build and Packaging**:
  - npm for package management
  - vsce for VS Code extension packaging
  - Automatic detection of WFL tools for enhanced functionality

### Configuration System
- `.wflcfg` files for project settings
- Global and local configuration support
- Validation with `--configCheck` and `--configFix` flags
- Extension configuration options for formatting and tool integration

### Debugging Tools
- Structured logging with verbosity levels
- Automatic debug reports on errors
- Interactive step-by-step execution

### Python Utility Tools
- **bump_version.py**: Script for managing version numbers across the codebase
- **launch_msi_build.py**: Utility for creating Windows MSI installers
- **rust_loc_counter.py**: Statistics tool for measuring code size and complexity
- **wfl_config_checker.py**: External configuration validation tool
- **wfl_md_combiner.py**: Documentation processor for combining markdown files
- **test_bump_version.py**: Test suite for the version management system

### AI Development Tools
- **Claude Code Action**: GitHub Action for code review and assistance
- **Gemini Deep Research**: AI-powered research capabilities
- **Memory Bank System**: Structured knowledge base for AI context preservation