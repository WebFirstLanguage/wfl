# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Structure & Modules
- `src/`: Core compiler/runtime (`main.rs`, `lib.rs`, `repl.rs`, `builtins.rs`).
- `tests/`: Rust integration/unit tests (e.g., `file_io_*`, `crypto_test.rs`).
- `TestPrograms/`: End‑to‑end WFL programs that must all pass.
- `wfl-lsp/`: Language Server workspace member; `vscode-extension/` for VS Code.
- `Docs/`: Guides and technical notes (see `Docs/guides/building.md`).
- `scripts/`: Utilities (`run_integration_tests.ps1|.sh`, `configure_lsp.ps1`).
- `.cursor/rules/`: Cursor IDE rules and guidelines (`wfl-rules.mdc`).

## Core Architecture
The WFL compiler pipeline consists of:
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
  - Web server support with HTTP request/response handling (integrated via `warp`)
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
- **REPL** (`src/repl.rs`): Interactive Read-Eval-Print Loop for experimentation

## Build, Test, and Dev Commands
- **Build**: `cargo build` (release: `cargo build --release`).
- **Run**: `cargo run -- <file.wfl>` or `target/release/wfl <file.wfl>`.
- **Test**: `cargo test`; integration requires release binary.
  - Windows: `./scripts/run_integration_tests.ps1`
  - Linux/macOS: `./scripts/run_integration_tests.sh`
- **Bench**: `cargo bench` (Criterion).
- **Format**: `cargo fmt --all`.
- **Lint**: `cargo clippy --all-targets --all-features -- -D warnings`.

### WFL CLI
- `wfl <file>`: Run a WFL program.
- `wfl`: Start interactive REPL.
- `wfl --lint <file>`: Lint WFL code.
- `wfl --fix <file> --in-place`: Auto-fix WFL code.
- `wfl --debug <file>`: Debug WFL execution.
- `wfl --step <file>`: Run in single-step debug mode.
- `wfl --time <file>`: Run with execution timing.
- `wfl --lex <file>` / `wfl --parse <file>`: Dump tokens or AST.
- `wfl --configCheck` / `wfl --configFix`: Check/fix configuration.
- `wfl --dump-env`: Dump environment for troubleshooting.
- `wfl --analyze <file>`: Run static analysis.

## Key Language Features
- **Natural Language Syntax**: `store name as "value"`, `check if x is greater than 5`.
- **Type Safety**: Static typing with intelligent type inference.
- **Async Support**: Built-in async/await using Tokio runtime.
- **Pattern Matching**: Regex-like engine with Unicode support.
- **Container System**: OOP with containers.
- **Security**: WFLHASH custom crypto, secure subprocess spawning.

## Coding Style & Naming
- **Format**: `cargo fmt --all` (see `.rustfmt.toml`).
- **Lint**: `cargo clippy --all-targets --all-features -- -D warnings`.
- **Naming**:
  - Functions/Files: `snake_case`
  - Types/Traits: `CamelCase`
  - Constants: `SCREAMING_SNAKE_CASE`

## Testing Guidelines
- **TDD is mandatory**: Write failing tests FIRST for any feature or bug fix.
- **Locations**:
  - Unit/Integration: `tests/`
  - End-to-End: `TestPrograms/` (must pass with release build)
- **Conventions**: feature‑oriented names (`*_test.rs`), keep perf benches under `benches/`.

## Commit & Pull Request Guidelines
- **Conventional Commits**: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`.
- **Pull Requests**: Clear description, linked issues, tests added/updated, repro steps.
- **Pre‑PR Checks**:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all --verbose`

## Agent‑Specific Policies (Critical Rules)
- **Backward Compatibility**: Sacred. Never break existing WFL programs. Run all `TestPrograms/`.
- **Integration Tests**: Require `cargo build --release` and provided scripts.
- **Documentation**: Keep `Docs/` current. Major changes warrant a Dev Diary note.
- **Security**: Review `SECURITY.md`. Avoid logging secrets. Use zeroization.
- **Rules**: Refer to `.cursor/rules/wfl-rules.mdc`.

## Technical Requirements
- **Rust Edition**: 2024 (Min: 1.75+, Dev: 1.91.1+)
- **Versioning**: YY.MM.BUILD (e.g., 26.1.13). Major version always < 256 (Windows MSI compatibility).
- **Key Dependencies**:
  - `logos`: Lexer
  - `tokio`: Async runtime
  - `reqwest`: HTTP client
  - `sqlx`: DB support
  - `warp`: Web server
  - `tower-lsp`: LSP server
  - `zeroize`, `subtle`: Crypto

## LSP Development Workflow
- **Location**: `wfl-lsp/` (LSP), `vscode-extension/` (VS Code).
- **Build/Run**: `cargo build -p wfl-lsp`.
- **Debug**: `RUST_LOG=trace cargo run -p wfl-lsp`.
- **Setup**: `scripts/configure_lsp.ps1`, `scripts/install_vscode_extension.ps1`.
- **Docs**: `Docs/guides/wfl-lsp-guide.md`, `Docs/guides/wfl-mcp-guide.md`.
