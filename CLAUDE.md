# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

WFL (WebFirst Language) is a natural language programming language implemented in Rust. It features intuitive syntax like "store x as 5" and "display 'Hello'", with static typing, async support, and comprehensive development tooling.

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

## Critical Development Rules

### Backward Compatibility Commitment
**NEVER BREAK EXISTING WFL PROGRAMS**. This is the #1 rule. Before merging any change:
1. Run ALL test programs in TestPrograms/
2. Verify identical behavior for existing syntax
3. Add new tests for new features
4. Document any edge cases

### Development Commands
```bash
# Build the project
cargo build --release

# Run a WFL program
cargo run -- program.wfl

# Run with debugging
cargo run -- program.wfl --debug > debug.txt 2>&1

# Run all tests
cargo test

# Lint a program
cargo run -- --lint program.wfl

# Analyze for issues
cargo run -- --analyze program.wfl

# Auto-fix formatting
cargo run -- --fix program.wfl --in-place

# Start REPL
cargo run

# Check code quality
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

## Standard Debug Procedure

When debugging ANY issue:
1. Create minimal test case in TestPrograms/
2. Run with debug flag: `cargo run -- test.wfl --debug > test_debug.txt 2>&1`
3. Check debug output for execution trace
4. Run static analyzer: `cargo run -- --analyze test.wfl`
5. Fix issues and verify ALL existing tests still pass

## Documentation Requirements

Before making changes:
1. Read `Docs/wfl-spec.md` for language specification
2. Check module-specific docs in `Docs/`
3. Review recent Dev Diary entries
4. Consult memory bank files in `.kilocode/rules/memory-bank/`

After making changes:
1. Update relevant documentation
2. Create Dev Diary entry with implementation details
3. Add/update tests in appropriate locations

## Testing Requirements

**ALL test programs in TestPrograms/ MUST pass**. Test categories:
- Basic syntax tests (variables, loops, conditions)
- Async/await tests
- Error handling tests
- Standard library tests
- Performance benchmarks

Run specific test: `cargo test test_name`
Run module tests: `cargo test --package wfl --lib module_name`

## Development Workflow

1. **Understand the task**: Read all relevant documentation
2. **Check existing code**: Search for similar patterns
3. **Write tests first**: Add to TestPrograms/ or unit tests
4. **Implement feature**: Follow existing code style
5. **Run all tests**: `cargo test` and TestPrograms/
6. **Check quality**: `cargo fmt` and `cargo clippy`
7. **Update docs**: Modify relevant .md files
8. **Create Dev Diary**: Document your implementation

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
| `--debug` | Enable debug output | `cargo run -- --debug program.wfl` |
| `--config` | Specify config file | `cargo run -- --config custom.wflcfg program.wfl` |
| `--time` | Show execution time | `cargo run -- --time program.wfl` |
| `-v, --version` | Show version info | `cargo run -- --version` |

## Architecture Overview

```
Input (.wfl) → Lexer → Parser → Analyzer → Type Checker → Interpreter → Output
                ↓       ↓         ↓           ↓              ↓
              Tokens   AST    Validated   Type Info    Execution
                              AST                       Results
```

Key components:
- **Lexer**: Token generation with Logos
- **Parser**: Recursive descent, indentation-aware
- **Analyzer**: Semantic validation, dead code detection
- **Type Checker**: Static type analysis
- **Interpreter**: Direct AST execution with async support
- **Stdlib**: core, math, text, list, time, pattern modules

## Key Implementation Notes

### Error Handling
- Use `InterpreterError` for runtime errors
- Include source location via spans
- Provide helpful error messages with context

### Async Operations
- All I/O operations are async (web.get, file operations)
- Use `await` keyword in WFL code
- Tokio runtime handles execution

### Type System
- Static typing with inference
- Types: text, number, boolean, list, null, any
- Function types for callbacks
- Pattern matching with regex support

### Memory Management
- Variables stored in Environment HashMap
- Scope management with push/pop
- Automatic cleanup on scope exit

## Current Focus Areas (June 2025)

1. **Testing**: Expanding test coverage and TestPrograms
2. **Performance**: Optimizing lexer and parser
3. **Error Messages**: Improving clarity and helpfulness
4. **Documentation**: Keeping all docs up-to-date
5. **Stability**: Ensuring backward compatibility

Remember: The goal is to make programming accessible while maintaining professional-grade tooling and performance.