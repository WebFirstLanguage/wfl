# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

WFL (WebFirst Language) is a programming language designed with natural language syntax to lower the barrier to entry for programming. It's currently in active development with most core components complete and stable. The project is developed with AI assistance from Devin.ai, ChatGPT, and Claude.

## Core Development Commands

### Building and Testing
```bash
# Standard build and test cycle
cargo fmt --all              # Format code
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
```

### Running WFL Programs
```bash
# Run a WFL program
cargo run -- path/to/program.wfl

# With debug output
cargo run -- --debug path/to/program.wfl

# Interactive mode (REPL)
cargo run -- --interactive
```

### Code Quality Tools
```bash
# Lint WFL code
cargo run -- --lint script.wfl

# Static analysis
cargo run -- --analyze script.wfl

# Auto-fix code issues
cargo run -- --fix script.wfl --in-place

# Check configuration
cargo run -- --configCheck
cargo run -- --configFix
```

### VSCode Extension Development
```bash
cd vscode-extension
npm install
npm run compile     # Build extension
npm run watch      # Watch mode
npm run test       # Run tests
```

## Architecture Overview

### Module Structure
The codebase follows a pipeline architecture:

1. **Lexer** (`src/lexer/`) - Tokenizes source code using Logos library
2. **Parser** (`src/parser/`) - Builds AST with natural language support
3. **Analyzer** (`src/analyzer/`) - Semantic analysis and validation
4. **Type Checker** (`src/typechecker/`) - Static type analysis
5. **Interpreter** (`src/interpreter/`) - Executes AST with Tokio async runtime

### Key Design Patterns

- **Error Handling**: Comprehensive error types with codespan-reporting for user-friendly messages
- **Async Operations**: Full Tokio integration for concurrent operations
- **Standard Library**: Modular design in `src/stdlib/` with core, math, text, list, and pattern modules
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

## Critical Implementation Notes

### Parser Stability
- The parser has comprehensive end token handling to prevent infinite loops
- Always consume orphaned tokens during error recovery
- Use `peek_token()` for lookahead, never `next_token()` unless consuming

### Memory Management
- Optional dhat heap profiling with `--features dhat-heap`
- Careful lifetime management in parser to avoid borrow checker issues
- Async operations properly handle cleanup

### Error Reporting
- All errors use the unified diagnostic system
- Include source context with precise spans
- Provide actionable suggestions when possible

### Testing Strategy
- Unit tests embedded in modules (`#[cfg(test)]`)
- Integration tests in `tests/` directory
- Example programs in `Test Programs/` for end-to-end testing
- Error examples in `Test Programs/error_examples/`

## Common Workflows

### Adding a New Feature
1. Update the lexer if new tokens needed
2. Extend the parser AST and parsing logic
3. Add semantic analysis rules
4. Implement type checking rules
5. Add interpreter execution logic
6. Write comprehensive tests
7. Update documentation

### Debugging Runtime Issues
1. Enable debug logging in `.wflcfg`
2. Check the generated `*_debug.txt` file
3. Use `exec_trace!` macro for additional logging
4. Review the execution flow in `wfl_exec.log`

### Updating Standard Library
1. Add function to appropriate module in `src/stdlib/`
2. Register in module's `register_functions()`
3. Add type signatures and validation
4. Write tests in the module's test section
5. Document in function catalog

## Key Files to Understand

- `src/parser/mod.rs` - Core parser logic and natural language handling
- `src/interpreter/mod.rs` - Execution engine with async support
- `src/stdlib/mod.rs` - Standard library registration
- `src/diagnostics/mod.rs` - Error reporting system
- `src/main.rs` - CLI entry point and command handling
- `.kilocode/rules/` - Additional AI assistant context and rules

Remember: This is alpha software under active development. Always prioritize stability and backward compatibility while implementing new features.