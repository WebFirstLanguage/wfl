---
type: "always_apply"
---

# WFL Development Guide for AI Assistants

This comprehensive guide provides instructions for AI assistants working on the WebFirst Language (WFL) project.

## Project Overview

WFL (WebFirst Language) is a natural language programming language implemented in Rust. It features intuitive syntax like "store x as 5" and "display 'Hello'", with static typing, async support, and comprehensive development tooling. The project is developed with AI assistance from Devin.ai, ChatGPT, and Claude.

## Memory Bank Context

This project uses a comprehensive memory bank system located in `.kilocode/rules/memory-bank/`. Always consult these files for detailed context:
- `architecture.md` - System design and processing pipeline
- `context.md` - Development history and key decisions
- `product.md` - Features, roadmap, and user experience
- `tech.md` - Implementation details and technical specifications

## Project Structure & Organization

### Core Architecture
```
wfl/
├── src/                    # Main source code
│   ├── lexer/             # Tokenization (Logos-based)
│   ├── parser/            # AST generation with natural language support
│   ├── analyzer/          # Semantic analysis and validation
│   ├── typechecker/       # Static type analysis
│   ├── interpreter/       # Execution engine with Tokio async runtime
│   ├── stdlib/            # Standard library modules
│   ├── linter/            # Code style checking
│   └── fixer/             # Auto-formatting
├── TestPrograms/          # Integration test programs (TDD)
├── tests/                 # Unit and integration tests (TDD)
├── Docs/                  # All user-facing documentation
├── Dev diary/             # Development history and progress
├── wfl-lsp/              # Language Server Protocol implementation
├── vscode-extension/      # VSCode language support
└── .kilocode/rules/      # Memory bank and AI context
```

### Module Organization
- Root crate `wfl` (compiler/runtime) in `src/`
- Workspace member `wfl-lsp/` provides Language Server (VS Code integration in `vscode-extension/`)
- Tests: inline (`src/**/tests.rs`) and integration (`tests/` using `*_test.rs`)
- Benchmarks in `benches/` (Criterion), Examples in `examples/`
- Scripts in `scripts/` (PowerShell/Bash), Packaging assets in `wix/`

## Core Development Principles

### 1. Test-Driven Development (TDD) is MANDATORY

**TDD is as critical as backward compatibility. Violating TDD is equivalent to breaking the build.**

#### TDD Rules (NEVER VIOLATE):
1. **Always write comprehensive failing tests FIRST** for any change
2. **Explicitly confirm that tests fail** before writing implementation code
3. **Commit failing tests as a baseline** before starting implementation
4. **Never modify tests to make them pass** - fix the implementation instead
5. **"Done" means all tests pass** with no changes to original test intent

#### TDD Workflow for Every Change:
```bash
# Step 1: Write failing test
echo "Writing test that MUST fail first..."
cargo test new_test_name 2>&1 | grep -E "(FAILED|failed)"  # MUST see failure

# Step 2: Commit failing test
git add tests/new_test.rs  # or TestPrograms/new_test.wfl
git commit -m "test: Add failing test for [feature/fix]"

# Step 3: Implement minimal code to pass
# Write ONLY enough code to make the test pass

# Step 4: Verify test passes
cargo test new_test_name  # MUST pass now

# Step 5: Refactor if needed (tests still pass)
cargo fmt --all && cargo clippy --all-targets -- -D warnings

# Step 6: Commit implementation
git add -A
git commit -m "feat/fix: Implement [feature/fix] to pass tests"
```

### 2. Backward Compatibility is Sacred
**NEVER BREAK EXISTING WFL PROGRAMS**. Before merging any change:
1. Write new tests for new features FIRST
2. Run ALL test programs in TestPrograms/
3. Verify identical behavior for existing syntax
4. Document any edge cases
5. If implementing something in the parser, also update the bytecode

### 3. Prime Development Directives
1. **TDD Compliance is Non-Negotiable**: Every change starts with a failing test
2. **Test Programs MUST Pass**: After ANY code change, run ALL programs in TestPrograms/
3. **User Experience First**: Error messages must be helpful, clear, and actionable
4. **Performance Matters**: Optimize for speed without sacrificing clarity
5. **Document Your Journey**: Create detailed Dev Diary entries for significant changes
6. **All documentation is in the Docs folder** - keep it updated
7. **All components must be documented** (parser, lexer, bytecode, etc.)

## Development Workflow: Explore → Plan → Code → Commit

### 1. EXPLORE Phase (Gather Context)
**Goal**: Understand the task without writing code

```bash
# Read relevant documentation
cat Docs/language-reference/wfl-spec.md
cat .kilocode/rules/memory-bank/*.md

# Search for similar patterns
cargo run -- --analyze similar_feature.wfl
grep -r "similar_pattern" src/

# Understand existing tests
ls tests/ TestPrograms/
cargo test --list | grep relevant_area

# DO NOT write any implementation code in this phase
```

### 2. PLAN Phase (Design Tests)
**Goal**: Create a TDD plan with specific test cases

Create `plan.md` with:
```markdown
# TDD Plan for [Feature/Fix Name]

## Test Cases to Write:
1. [ ] Test case 1: Description (expected to fail because...)
2. [ ] Test case 2: Description (expected to fail because...)
3. [ ] Edge case test: Description

## Implementation Strategy:
- Minimal code needed to pass test 1
- Additional code for test 2
- Refactoring opportunities

## Files to Modify:
- tests/new_test.rs (new test file)
- src/module/file.rs (implementation)
```

### 3. CODE Phase (TDD Implementation)
**Goal**: Write failing tests, then minimal implementation

```bash
# Write test first
echo "Creating failing test..."
# Edit tests/feature_test.rs or TestPrograms/feature.wfl

# Confirm test fails
cargo test feature_test 2>&1 | tee test_failure.log
grep -q "FAILED" test_failure.log || echo "ERROR: Test must fail first!"

# Commit failing test
git add tests/
git commit -m "test: Add failing test for [feature]"

# NOW write implementation
echo "Writing minimal implementation..."
# Edit src/module/implementation.rs

# Verify test passes
cargo test feature_test

# Run ALL tests to ensure no regression
cargo test
Get-ChildItem TestPrograms\*.wfl | ForEach-Object { .\target\release\wfl.exe $_.FullName }
```

### 4. COMMIT Phase (Finalize)
**Goal**: Clean code, update docs, commit everything

```bash
# Format and lint
cargo fmt --all
cargo clippy --all-targets -- -D warnings

# Update documentation
# Edit Docs/relevant_doc.md

# Create Dev Diary entry
echo "## $(date): [Feature Name]" >> "Dev diary/$(date +%Y-%m).md"

# Final test run
cargo test --release
cargo run -- --analyze TestPrograms/*.wfl

# Commit implementation with tests
git add -A
git commit -m "feat: [Feature] with comprehensive tests

- Added failing tests first (commit SHA)
- Implemented minimal solution
- All TestPrograms/ still pass
- Updated documentation"
```

## Build, Test, and Run Commands

### Building and Testing (TDD-Enhanced)
```bash
# TDD cycle commands
cargo test --lib my_new_test 2>&1 | grep FAILED  # Must fail first!
git add tests/ && git commit -m "test: failing test for X"
cargo build  # Now implement
cargo test --lib my_new_test  # Must pass now!

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

# Run all test programs (MANDATORY before commit)
# Use the integration test scripts (recommended):
# Windows PowerShell:
.\scripts\run_integration_tests.ps1

# Linux/macOS:
./scripts/run_integration_tests.sh

# Or run manually:
# Windows PowerShell:
Get-ChildItem TestPrograms\*.wfl | ForEach-Object { .\target\release\wfl.exe $_.FullName }

# Linux/macOS:
for file in TestPrograms/*.wfl; do ./target/release/wfl "$file"; done

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

## Coding Style & Standards

### Rust Style Guidelines
- Rust style via rustfmt. Format before pushing: `cargo fmt`
- Lint with Clippy: `cargo clippy -- -D warnings`
- Indentation: 4 spaces; max width ~100 (see rustfmt config)
- Tests and files: prefer descriptive names; integration tests use `*_test.rs`

### Commit & PR Guidelines
- Prefer Conventional Commits style: `feat:`, `fix:`, `test:`, `chore:`, `refactor:`
- PRs should include: clear description, rationale, test updates, and `cargo test` output
- Link issues with `Fixes #123`
- Keep changes scoped; update docs/examples when behavior changes

### Testing Guidelines
- Use `cargo test` for unit and integration tests
- Place integration tests in `tests/` and module tests in `src/**/tests.rs`
- Add focused tests near the code they cover
- For performance-sensitive paths, add Criterion benches in `benches/`

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
| `--time` | Measure execution time | `cargo run -- --time program.wfl` |
| `--interactive` | Start REPL mode | `cargo run -- --interactive` |
| `-v, --version` | Show version info | `cargo run -- --version` |

## Architecture Overview

### Processing Pipeline
The codebase follows a traditional compiler architecture:

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

## Testing Requirements (TDD-Enforced)

### Test Categories (ALL MUST PASS):
- Unit tests in `tests/` directory
- Integration tests in TestPrograms/
- Basic syntax tests (variables, loops, conditions)
- Async/await tests
- Error handling tests
- Standard library tests
- Container and inheritance tests
- Performance benchmarks

### TDD Test Commands:
```bash
# Write new test (MUST fail first)
echo "Writing failing test..." >> tests/new_feature.rs
cargo test new_feature 2>&1 | grep FAILED || exit 1

# Run specific test
cargo test test_name

# Run module tests
cargo test --package wfl --lib module_name

# Run integration tests
cargo test --test '*'

# Verify all TestPrograms still work
Get-ChildItem TestPrograms\*.wfl | ForEach-Object {
    Write-Host "Testing $_"
    .\target\release\wfl.exe $_.FullName
    if ($LASTEXITCODE -ne 0) { exit 1 }
}
```

## Anti-Patterns (FORBIDDEN PRACTICES)

### TDD Violations (NEVER DO THESE):

1. ❌ **Writing implementation before tests**
2. ❌ **Skipping the "confirm failure" step**
3. ❌ **Modifying tests to make them pass**
4. ❌ **Loosely defined or incomplete test coverage**
5. ❌ **Committing without tests**
6. ❌ **"Fixing" tests instead of implementation**

### Correct TDD Pattern:
```bash
# RIGHT: Test-first development
1. Write test that captures intended behavior
2. Run test, see it fail
3. Commit failing test
4. Write minimal code to pass
5. Refactor if needed (tests still pass)
6. Commit implementation
```

## Standard Debug Procedure (TDD-Enhanced)

When debugging ANY issue:
1. **Write a failing test that reproduces the issue** in TestPrograms/
2. **Confirm the test fails** with the expected error
3. **Commit the failing test** as proof of the bug
4. Run with debug flag: `cargo run -- test.wfl --debug > test_debug.txt 2>&1`
5. Check debug output for execution trace
6. Run static analyzer: `cargo run -- --analyze test.wfl`
7. Fix issues until the test passes
8. Verify ALL existing tests still pass
9. Run: `cargo fmt --all && cargo clippy --all-targets -- -D warnings`
10. Commit the fix with reference to the test

## Documentation Requirements

Before making changes:
1. Read `Docs/language-reference/wfl-spec.md` for language specification
2. Check module-specific docs in `Docs/`
3. Review recent Dev Diary entries
4. Consult memory bank files in `.kilocode/rules/memory-bank/`
5. Read the README.md for project overview
6. **Check existing tests** to understand expected behavior

After making changes:
1. **Ensure all new code has tests** (TDD compliance)
2. Update relevant documentation in `Docs/`
3. Create Dev Diary entry with implementation details
4. Document test strategy in Dev Diary
5. Add/update tests in appropriate locations
6. Update README.md if adding major features

## Critical Implementation Notes

### Parser Stability
- The parser has comprehensive end token handling to prevent infinite loops
- Always consume orphaned tokens during error recovery
- Use `peek_token()` for lookahead, never `next_token()` unless consuming
- Enhanced end token handling is a critical stability fix (May 2025)
- **All parser changes need comprehensive test coverage first**

### Memory Management
- Optional dhat heap profiling with `--features dhat-heap`
- Careful lifetime management in parser to avoid borrow checker issues
- Async operations properly handle cleanup
- Variables stored in Environment HashMap
- Scope management with push/pop
- Automatic cleanup on scope exit
- **Memory leak tests required for new features**

### Error Reporting
- All errors use the unified diagnostic system
- Include source context with precise spans
- Provide actionable suggestions when possible
- Use `InterpreterError` for runtime errors
- Errors should be helpful without demanding code changes (backward compatibility)
- **Error cases must have explicit tests**

### Type System
- Static typing with inference
- Types: text, number, boolean, list, null, any
- Function types for callbacks
- Pattern matching with regex support
- Flexible type handling for backward compatibility
- **Type checking needs test coverage for each type**

### Async Operations
- All I/O operations are async (web.get, file operations)
- Use `await` keyword in WFL code
- Tokio runtime handles execution
- HTTP requests via Reqwest (v0.11.24)
- Database support via SQLx (v0.8.1)
- **Async operations require timeout and error tests**

## Git Sync - Handling Diverged Branches

When branches diverge (common with CI/CD version bumps), use the sync scripts:

### Quick Usage
```bash
# Sync current branch with origin (bash)
./scripts/sync-branch.sh -f

# Or use git alias
git sync-sh

# For PowerShell (needs fixing)
git sync       # or git sync-force
```

### What the Sync Script Does
1. **Detects divergence** - Checks if local and remote have different commits
2. **Stashes changes** - Temporarily saves uncommitted work (with -f flag)
3. **Rebases commits** - Puts your local commits on top of remote changes
4. **Restores work** - Re-applies stashed changes after sync

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
- `tests/` - Unit test directory (TDD tests go here)
- `TestPrograms/` - Integration test programs (TDD integration tests)

## Current Focus Areas (September 2025)

1. **TDD Compliance**: Ensuring all new code follows test-first development
2. **Testing**: Expanding test coverage and TestPrograms
3. **Performance**: Optimizing lexer and parser (with benchmark tests)
4. **Error Messages**: Improving clarity and helpfulness (with error tests)
5. **Documentation**: Keeping all docs up-to-date
6. **Stability**: Ensuring backward compatibility
7. **Version**: Currently at v25.8.11

## Debugging Principles

- **TDD First**: Every bug gets a failing test before any fix
- **Interpreter Debugging Principle**: We are building WFL, so unless told to debug the script, we are debugging the interpreter itself
- **Test-Driven Debugging**: Bugs are fixed when the test passes, not when it "looks right"

## Final TDD Checklist

Before ANY commit, verify:
- [ ] Failing tests were written first
- [ ] Failing tests were committed separately
- [ ] Implementation is minimal to pass tests
- [ ] All existing tests still pass
- [ ] No test was modified to pass
- [ ] Coverage didn't decrease
- [ ] Documentation updated
- [ ] Dev Diary entry created

## Security & Configuration

- Do not commit secrets. Review `SECURITY.md` before reporting vulnerabilities
- Local runtime settings live in `.wflcfg` (created via `scripts/init_config.ps1`)
- Avoid checking machine-specific configs into VCS
- Init local config: `powershell ./scripts/init_config.ps1` (creates `.wflcfg`)

---

Remember: This is alpha software under active development. TDD ensures we build the right thing correctly. Always prioritize test-first development and backward compatibility while implementing new features. The goal is to make programming accessible while maintaining professional-grade tooling and performance through rigorous testing.

**TDD is not optional. It is the foundation of reliable software.**
