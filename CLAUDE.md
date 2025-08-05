# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

WFL (WebFirst Language) is a natural language programming language implemented in Rust. It features intuitive syntax like "store x as 5" and "display 'Hello'", with static typing, async support, and comprehensive development tooling. The project is developed with AI assistance from Devin.ai, ChatGPT, and Claude.

## Memory Bank Context

This project uses a comprehensive memory bank system located in `.kilocode/rules/memory-bank/`. Always consult these files for detailed context:
- `architecture.md` - System design and processing pipeline
- `context.md` - Development history and key decisions
- `product.md` - Features, roadmap, and user experience
- `tech.md` - Implementation details and technical specifications

## Prime Development Directives

1. **Test Programs MUST Pass**: After ANY code change, run ALL programs in TestPrograms/ and ensure they execute successfully
2. **Backward Compatibility is Sacred**: NEVER break existing WFL programs. Maintain 100% compatibility with all syntax
3. **User Experience First**: Error messages must be helpful, clear, and actionable
4. **Performance Matters**: Optimize for speed without sacrificing clarity or correctness
5. **Document Your Journey**: Create detailed Dev Diary entries for all significant changes
6. **All documentation is in the Docs folder** off the main project root - keep it updated
7. **All components must be documented** (parser, lexer, bytecode, etc.)

## Critical Development Rules

### Backward Compatibility Commitment
**NEVER BREAK EXISTING WFL PROGRAMS**. This is the #1 rule. Before merging any change:
1. Run ALL test programs in TestPrograms/
2. Verify identical behavior for existing syntax
3. Add new tests for new features
4. Document any edge cases
5. If implementing something in the parser, also update the bytecode

## Core Development Commands

### Building and Testing
```bash
# Standard build and test cycle
cargo fmt --all              # Format code (uses .rustfmt.toml config)
cargo build                  # Build debug version
cargo test                   # Run all tests
cargo clippy --all-targets -- -D warnings  # Lint code

# Release build
cargo build --release
cargo test --release

# Run a single test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Test a specific module
cargo test --package wfl --lib module_name

# Windows-specific: Run WFL programs directly
./target/debug/wfl.exe TestPrograms/simple_test.wfl
./target/release/wfl.exe TestPrograms/simple_test.wfl

# Run benchmarks
cargo bench

# Memory profiling (optional)
cargo build --features dhat-heap
```

### Running WFL Programs
```bash
# Run a WFL program
cargo run -- path/to/program.wfl

# From release build
./target/release/wfl path/to/program.wfl

# With debug output
cargo run -- --debug path/to/program.wfl > debug.txt 2>&1

# Interactive mode (REPL)
cargo run -- --interactive

# Run all test programs (common validation)
# Linux/macOS:
for file in TestPrograms/*.wfl; do ./target/release/wfl "$file"; done

# Windows PowerShell:
Get-ChildItem TestPrograms\*.wfl | ForEach-Object { .\target\release\wfl.exe $_.FullName }
```

### Code Quality Tools
```bash
# Lint WFL code
cargo run -- --lint script.wfl

# Static analysis
cargo run -- --analyze script.wfl

# Auto-fix code issues
cargo run -- --fix script.wfl --in-place

# Check fix without applying
cargo run -- --fix script.wfl --check

# View diff of proposed fixes
cargo run -- --fix script.wfl --diff

# Check configuration
cargo run -- --configCheck
cargo run -- --configFix
```

### VSCode Extension Development
```bash
cd vscode-extension
npm install
npm run compile     # Build extension
npm run watch      # Watch mode for development
npm run test       # Run tests

# Install extension locally (Windows PowerShell)
../scripts/install_vscode_extension.ps1
```

## CLI Flag Reference

| Flag | Description | Example |
|------|-------------|---------|
| `--lex` | Output lexer tokens only | `cargo run -- --lex program.wfl` |
| `--parse` | Output AST only | `cargo run -- --parse program.wfl` |
| `--lint` | Check code style | `cargo run -- --lint program.wfl` |
| `--analyze` | Static analysis | `cargo run -- --analyze program.wfl` |
| `--fix` | Auto-format code | `cargo run -- --fix program.wfl` |
| `--in-place` | Modify file directly | `cargo run -- --fix program.wfl --in-place` |
| `--check` | Dry run for --fix | `cargo run -- --fix program.wfl --check` |
| `--diff` | Show diff for --fix | `cargo run -- --fix program.wfl --diff` |
| `--debug` | Enable debug output | `cargo run -- --debug program.wfl` |
| `--config` | Specify config file | `cargo run -- --config custom.wflcfg program.wfl` |
| `--time` | Show execution time | `cargo run -- --time program.wfl` |
| `--interactive` | Start REPL mode | `cargo run -- --interactive` |
| `-v, --version` | Show version info | `cargo run -- --version` |

## Architecture Overview

### Module Structure
The codebase follows a pipeline architecture:

```
Input (.wfl) → Lexer → Parser → Analyzer → Type Checker → Interpreter → Output
                ↓       ↓         ↓           ↓              ↓
              Tokens   AST    Validated   Type Info    Execution
                              AST                       Results
```

1. **Lexer** (`src/lexer/`) - Tokenizes source code using Logos library
2. **Parser** (`src/parser/`) - Builds AST with natural language support
3. **Analyzer** (`src/analyzer/`) - Semantic analysis and validation
4. **Type Checker** (`src/typechecker/`) - Static type analysis
5. **Interpreter** (`src/interpreter/`) - Executes AST with Tokio async runtime
6. **Linter** (`src/linter/`) - Code style checking
7. **Fixer** (`src/fixer/`) - Automatic code formatting
8. **LSP** (`wfl-lsp/`) - Language Server Protocol implementation

### Key Design Patterns

- **Error Handling**: Comprehensive error types with codespan-reporting for user-friendly messages
- **Async Operations**: Full Tokio integration for concurrent operations (v1.35.1)
- **Standard Library**: Modular design in `src/stdlib/` with core, math, text, list, time, and pattern modules
- **Configuration**: Hierarchical config system (global → local) in `src/config.rs` and `src/wfl_config/`
- **Logging**: Dual logging system - standard logger and execution tracer using `exec_trace!` macro

### Container System
WFL uses "containers" (similar to classes) with:
- Properties and actions (methods)
- Inheritance support
- Interface implementation
- Event handling
- Found in `src/parser/container_*.rs`

### Natural Language Parsing
The parser supports English-like syntax:
- "store X as Y" for variable assignment
- "check if X is greater than Y" for conditionals  
- "count from X to Y" for loops
- Function calls like "length of mylist"

## Standard Debug Procedure

When debugging ANY issue:
1. Create minimal test case in TestPrograms/
2. Run with debug flag: `cargo run -- test.wfl --debug > test_debug.txt 2>&1`
3. Check debug output for execution trace
4. Run static analyzer: `cargo run -- --analyze test.wfl`
5. Fix issues and verify ALL existing tests still pass
6. Run: `cargo fmt --all && cargo clippy --all-targets -- -D warnings`

## AI Development Rules

When working on this codebase:

1. **Never break existing functionality** - All changes must maintain backward compatibility
2. **Follow the 6-step debug procedure** for any issues:
   - Understand the issue
   - Review code and logs
   - Form hypothesis
   - Make targeted change
   - Test thoroughly
   - Document in Dev diary
3. **Test all changes** - Run the full test suite before considering work complete
4. **Update Dev diary** - Create entries in `Dev diary/` for significant changes
5. **Maintain clean separation** - Debug output uses `exec_trace!`, never pollutes program output
6. **Read todo.md and implementation_progress_{date}.md** in Docs folder to understand current state
7. **Update README.md** with any new important information

## Testing Requirements

**ALL test programs in TestPrograms/ MUST pass**. Test categories:
- Basic syntax tests (variables, loops, conditions)
- Async/await tests
- Error handling tests
- Standard library tests
- Container and inheritance tests
- Performance benchmarks

Run specific test: `cargo test test_name`
Run module tests: `cargo test --package wfl --lib module_name`
Run integration tests: `cargo test --test '*'`

## Documentation Requirements

Before making changes:
1. Read `Docs/wfl-spec.md` for language specification
2. Check module-specific docs in `Docs/`
3. Review recent Dev Diary entries
4. Consult memory bank files in `.kilocode/rules/memory-bank/`
5. Read the README.md for project overview

After making changes:
1. Update relevant documentation in `Docs/`
2. Create Dev Diary entry with implementation details
3. Add/update tests in appropriate locations
4. Update README.md if adding major features

## Critical Implementation Notes

### Parser Stability
- The parser has comprehensive end token handling to prevent infinite loops
- Always consume orphaned tokens during error recovery
- Use `peek_token()` for lookahead, never `next_token()` unless consuming
- Enhanced end token handling is a critical stability fix (May 2025)

### Memory Management
- Optional dhat heap profiling with `--features dhat-heap`
- Careful lifetime management in parser to avoid borrow checker issues
- Async operations properly handle cleanup
- Variables stored in Environment HashMap
- Scope management with push/pop
- Automatic cleanup on scope exit

### Error Reporting
- All errors use the unified diagnostic system
- Include source context with precise spans
- Provide actionable suggestions when possible
- Use `InterpreterError` for runtime errors
- Errors should be helpful without demanding code changes (backward compatibility)

### Type System
- Static typing with inference
- Types: text, number, boolean, list, null, any
- Function types for callbacks
- Pattern matching with regex support
- Flexible type handling for backward compatibility

### Async Operations
- All I/O operations are async (web.get, file operations)
- Use `await` keyword in WFL code
- Tokio runtime handles execution
- HTTP requests via Reqwest (v0.11.24)
- Database support via SQLx (v0.8.1)

## Common Workflows

### Adding a New Feature
1. Update the lexer if new tokens needed
2. Extend the parser AST and parsing logic
3. Add semantic analysis rules
4. Implement type checking rules
5. Add interpreter execution logic
6. Write comprehensive tests in TestPrograms/
7. Update documentation in Docs/
8. Update bytecode implementation if parser was changed

### Development Workflow
1. **Understand the task**: Read all relevant documentation
2. **Check existing code**: Search for similar patterns
3. **Write tests first**: Add to TestPrograms/ or unit tests
4. **Implement feature**: Follow existing code style
5. **Run all tests**: `cargo test` and TestPrograms/
6. **Check quality**: `cargo fmt` and `cargo clippy`
7. **Update docs**: Modify relevant .md files in Docs/
8. **Create Dev Diary**: Document your implementation

### Updating Standard Library
1. Add function to appropriate module in `src/stdlib/`
2. Register in module's `register_functions()`
3. Add type signatures and validation
4. Write tests in the module's test section
5. Document in function catalog

## Debug and code quality

MUST ALWAYS run cargo clippy --all-targets --all-features -- -D warnings and fix any reported errors
MUST ALWAYS run cargo fmt --all to fix formatting issues

## Key Files to Understand

- `src/main.rs` - CLI entry point and command handling
- `src/parser/mod.rs` - Core parser logic and natural language handling
- `src/interpreter/mod.rs` - Execution engine with async support
- `src/stdlib/mod.rs` - Standard library registration
- `src/diagnostics/mod.rs` - Error reporting system
- `src/lexer/mod.rs` - Tokenization with Logos
- `src/analyzer/mod.rs` - Semantic analysis
- `src/typechecker/mod.rs` - Type checking
- `.kilocode/rules/` - Additional AI assistant context and rules
- `Cargo.toml` - Dependencies and project configuration

## Project Structure

```
wfl/
├── src/                    # Main source code
│   ├── lexer/             # Tokenization
│   ├── parser/            # AST generation
│   ├── analyzer/          # Semantic analysis
│   ├── typechecker/       # Type checking
│   ├── interpreter/       # Execution engine
│   ├── stdlib/            # Standard library
│   ├── linter/            # Code style checking
│   └── fixer/             # Auto-formatting
├── TestPrograms/          # Integration test programs
├── tests/                 # Unit and integration tests
├── Docs/                  # All documentation
├── Dev diary/             # Development history
├── vscode-extension/      # VSCode language support
├── wfl-lsp/              # Language Server Protocol
└── .kilocode/rules/      # Memory bank and AI context
```

## Current Focus Areas (August 2025)

1. **Testing**: Expanding test coverage and TestPrograms
2. **Performance**: Optimizing lexer and parser
3. **Error Messages**: Improving clarity and helpfulness
4. **Documentation**: Keeping all docs up-to-date
5. **Stability**: Ensuring backward compatibility
6. **Version**: Currently at v25.8.3

## Debugging 

- **Interpreter Debugging Principle**: We are building WFL, so unless told to debug the script, we are debugging the interpreter itself

Remember: This is alpha software under active development. Always prioritize stability and backward compatibility while implementing new features. The goal is to make programming accessible while maintaining professional-grade tooling and performance.