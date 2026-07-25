# Repository Guidelines

## Project Governance (do not dig — start here)

WFL is a **maintainer-led** open-source project (primary Maintainer: **Brad**, Logbie LLC).
Binding community and contribution policy lives at the **repo root** (not only under `Docs/`):

| File | What it is |
|---|---|
| `GOVERNANCE.md` | Authority, roles (Maintainer / Contributor / Participant), decision rights, binding technical policies |
| `CODE_OF_CONDUCT.md` | Community standards and enforcement |
| `AI_POLICY.md` | **AI-assisted work is welcome** — WFL was built with AI; do not discriminate against AI use; human author remains accountable |
| `CONTRIBUTING.md` | How to contribute; **Contributor application** process (Discussion or email) |
| `SECURITY.md` | Private vulnerability reporting only — never file security bugs as public issues |
| `testing.md` | **Binding Logbie Testing Policy + WFL testing profile** — Red→Green TDD evidence, required test layers, risk classes, and merge/release gates (see **Testing Guidelines** below) |

**Agent implications (already in force via governance):**

- **AI is first-class** — use coding agents freely; same quality bar as hand-written work (tests, docs, compatibility, reviewability).
- **Backward compatibility is sacred** — never break existing WFL programs without the documented deprecation path.
- **TDD mandatory** — failing tests first (`tests/`, `TestPrograms/`), governed by the binding **Logbie Testing Policy** in root `testing.md`: auditable **Red→Green** evidence for every behavioral change, coverage at the lowest useful layer plus every affected higher layer.
- **Docs ship with the feature** — same change; validate examples; Dev Diary for non-trivial work.
- **Quality gates** — `cargo fmt`, `clippy -D warnings`, `cargo test`; conventional commits.
- **Do not invent maintainer identity or process** — Contributor status is by application; Maintainers own merges and releases unless those responsibilities are **explicitly delegated**. Prefer first name **Brad** only if referring to the primary maintainer in docs (no last name).
- Community tone: follow `CODE_OF_CONDUCT.md`; technical disagreement is fine; harassment and AI-shaming are not.

When changing contribution workflow, community rules, or project authority, update the root governance suite **and** keep this section accurate.

## Project Structure & Modules
- `src/`: Core compiler/runtime (`main.rs`, `lib.rs`, `repl.rs`, `builtins.rs`).
- `crates/`: Internal crates (e.g., `wfl_core`).
- `tests/`: Rust integration/unit tests (e.g., `file_io_*`, `crypto_test.rs`).
- `benches/`: Performance benchmarks (Criterion).
- `examples/`: Example WFL programs and demos.
- `TestPrograms/`: End‑to‑end WFL programs that must all pass.
- `wfl-lsp/`: Language Server workspace member.
- `vscode-extension/`: VS Code extension integration.
- `Docs/`: Complete user documentation (organized in 6 sections plus guides/reference). See `Docs/README.md`.
- `scripts/`: Utilities (`run_integration_tests.ps1|.sh`, `configure_lsp.ps1`, `sync-branch.sh`).
- `Tools/`: Helper tools (Python scripts, WFL tools).
- `Nexus/`: Experimental WFL test programs.
- `wflpkg/`: Package Manager design documents.
- `wix/`: Windows Installer (MSI) configuration.
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
  - Integration: `./scripts/run_integration_tests.ps1` or `.sh`
  - Web Server: `./scripts/run_web_tests.ps1` or `.sh`
  - Docs Validation: `python scripts/validate_docs_examples.py`
- **Bench**: `cargo bench` (Criterion).
- **Format**: `cargo fmt --all`.
- **Lint**: `cargo clippy --all-targets --all-features -- -D warnings`.

### WFL CLI
- `wfl <file>`: Run a WFL program.
- `wfl`: Start interactive REPL.
- `wfl --lint <file>`: Lint WFL code.
- `wfl --lint --fix <file> --in-place`: Auto-fix WFL code.
- `wfl --edit <file>`: Open the specified file in the default editor.
- `wfl --step <file>`: Run in single-step debug mode.
- `wfl --time <file>`: Run with execution timing.
- `wfl --lex <file>` / `wfl --parse <file>`: Dump tokens or AST.
- `wfl --init [dir]`: Create .wflcfg interactively (default: current directory).
- `wfl --configCheck` / `wfl --configFix`: Check/fix configuration.
- `wfl --dump-env`: Dump environment for troubleshooting.
- `wfl --analyze <file>`: Run static analysis.
- `wfl --test <file>`: Run file in test mode (executes describe/test blocks).

## Key Language Features
- **Natural Language Syntax**: `store name as "value"`, `check if x is greater than 5`.
- **Type Safety**: Static typing with intelligent type inference.
- **Async Support**: Built-in async/await using Tokio runtime.
- **Pattern Matching**: Regex-like engine with Unicode support.
- **Container System**: OOP with containers.
- **Testing Framework**: Built-in testing with `describe`, `test`, and natural language assertions.
- **Security**: WFLHASH custom crypto, secure subprocess spawning.

## Coding Style & Naming
- **Format**: `cargo fmt --all` (see `.rustfmt.toml`).
- **Lint**: `cargo clippy --all-targets --all-features -- -D warnings`.
- **Naming**:
  - Functions/Files: `snake_case`
  - Types/Traits: `CamelCase`
  - Constants: `SCREAMING_SNAKE_CASE`

## Testing Guidelines

**Binding policy:** root `testing.md` holds the **Logbie Testing Policy** and the
WFL testing profile. It governs every behavioral change. Non-negotiables an agent
MUST follow:

- **Red → Green → Refactor → Broaden → Record** — write the smallest useful test
  FIRST, run it, confirm it **fails for the intended reason**, then make it pass;
  a defect fix reproduces the defect. Keep auditable Red evidence (a Red commit
  that is an ancestor of Green, or a timestamped CI artifact). A test first
  observed after the code already passed is **not** a valid Red step. (§3, §6)
- **Classify risk first (R0–R3)** — concurrency, cancellation, lifecycle,
  streaming, untrusted input, crypto/secrets, and backward compatibility are
  **R3** and require negative/failure-path plus §11 risk-triggered tests. Risk is
  never lowered to dodge a gate. (§5, §11)
- **Real boundaries, real assertions** — don't mock the boundary under test;
  assert outcomes + side effects (not "didn't crash"); use negative assertions
  for cancellation, writes-after-close, denial. (§7, §8.3)
- **No manufactured green** — never retry/skip/quarantine a required test to go
  green; a flaky required test is failing. Non-executable docs programs use the
  runner's `// CI-SKIP:` first-line directive and stay statically validated. (§8.2)
- **Concurrency/streaming/lifecycle (§11.3)** — for this repo's async/web/
  streaming work, prove races/ordering, cancellation, timeouts, disconnects,
  bounded queues/backpressure, resource limits, clean shutdown, writes-after-
  close, and that one slow/failed handler doesn't block unrelated work.
- **PR evidence (§15)** — record risk class, acceptance criteria → tests, Red
  evidence, layers run, and residual risk (template in `testing.md`).

### Mechanics
- **Locations**:
  - Rust Unit/Integration: `tests/`
  - WFL End-to-End: `TestPrograms/` (must pass with release build)
  - WFL Test Framework: Use `describe`/`test` blocks, run with `wfl --test <file>`
- **Conventions**: feature‑oriented names (`*_test.rs`, `*.test.wfl`), keep perf benches under `benches/`.
- **Commands & profile**: one command per layer + the "run all presubmit" block are in root `testing.md`.
- **Testing Guide**: See `Docs/guides/testing-guide.md` for WFL testing framework documentation.

## Commit & Pull Request Guidelines
- **Conventional Commits**: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`.
- **Pull Requests**: Clear description, linked issues, tests added/updated, repro steps.
- **Pre‑PR Checks**:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all --verbose`

## Documentation Development
- **Location**: `Docs/` organized in 6 sections (Introduction, Getting Started, Language Basics, Advanced Features, Standard Library, Best Practices).
- **Structure**: Follow `Docs/wfl-documentation-policy.md` and 19 principles in `Docs/wfl-foundation.md`.
- **Reference Documentation**: Two-tiered system for keywords
  - `Docs/reference/keyword-reference.md` - Quick scannable lookup (2-3 pages, all 181 keywords)
  - `Docs/reference/reserved-keywords.md` - Complete technical reference (10-15 pages, classifications, edge cases)
  - Both updated together; quick reference for speed, comprehensive for understanding
- **Validation**: ALL code examples MUST be validated with MCP tools before adding to docs.
  - Test examples in `TestPrograms/docs_examples/` with manifest tracking in `_meta/manifest.json`.
  - Run validation: `python scripts/validate_docs_examples.py`
  - Use MCP tools: `mcp__wfl-lsp__parse_wfl`, `mcp__wfl-lsp__analyze_wfl`, `mcp__wfl-lsp__typecheck_wfl`, `mcp__wfl-lsp__lint_wfl`
- **Critical Syntax**:
  - Conditionals use NESTED blocks: `otherwise: check if`, NOT `otherwise check if`
  - Reserved keywords: **181 keywords total** (54 structural, 29 contextual, 96 other, 7 literals; see `Docs/reference/reserved-keywords.md`)
    - Always reserved: `is`, `file`, `add`, `current`, `check`, `store`, etc.
    - Contextual (can be variables in some contexts): `count`, `list`, `pattern`, `text`, `at`, etc.
    - Use underscores to avoid conflicts: `is_active`, `filename`, `my_list`
    - See `Docs/reference/keyword-reference.md` (quick) and `Docs/reference/reserved-keywords.md` (complete)
  - List push syntax: `push with <list> and <value>`, NOT `push to`
  - Loop variable: `count` in count loops, NOT `the current count`
  - Typeof syntax: `typeof of value`, NOT `typeof(value)`
  - Action syntax: `define action called name with parameters x:`, NOT `action name with x:`
- **Working Examples**:
  - Core syntax: `TestPrograms/basic_syntax_comprehensive.wfl`, `file_io_comprehensive.wfl`, `comprehensive_web_server_demo.wfl`, `containers_comprehensive.wfl`, `patterns_comprehensive.wfl`
  - Keyword examples: `TestPrograms/docs_examples/keyword_reference/` (11 example files with validation manifest)

## Agent‑Specific Policies (Critical Rules)
- **Governance**: Follow root `GOVERNANCE.md`, `CODE_OF_CONDUCT.md`, `AI_POLICY.md`, `CONTRIBUTING.md` (see **Project Governance** above). Do not re-litigate AI use; do not skip quality gates because AI produced the draft.
- **Backward Compatibility**: Sacred. Never break existing WFL programs without the documented deprecation path (`GOVERNANCE.md`). Run all `TestPrograms/`.
- **Integration Tests**: Require `cargo build --release` and provided scripts.
- **Documentation**: Keep `Docs/` current. Validate ALL code examples with MCP before adding. Major changes warrant Dev Diary note. User-facing behavior changes ship docs in the same change.
- **Security**: Review `SECURITY.md`. Avoid logging secrets. Use zeroization. No public security issues.
- **Rules**: Refer to `.cursor/rules/wfl-rules.mdc`.

## Technical Requirements
- **Rust Edition**: 2024 (Min: 1.94+ — raised by the `sqlx` 0.9 dependency; Dev: 1.94+)
- **Versioning**: YY.MM.BUILD (e.g., 26.1.22). Major version always < 256 (Windows MSI compatibility).
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
- **Docs**: See `Docs/development/lsp-integration.md` for dev guides and `Docs/02-getting-started/editor-setup.md` for user setup.
