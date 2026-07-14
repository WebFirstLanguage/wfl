# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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

**Agent implications (already in force via governance):**

- **AI is first-class** — use coding agents freely; same quality bar as hand-written work (tests, docs, compatibility, reviewability).
- **Backward compatibility is sacred** — never break existing WFL programs without the documented deprecation path.
- **TDD mandatory** — failing tests first (`tests/`, `TestPrograms/`).
- **Docs ship with the feature** — same change; validate examples; Dev Diary for non-trivial work.
- **Quality gates** — `cargo fmt`, `clippy -D warnings`, `cargo test`; conventional commits.
- **Do not invent maintainer identity or process** — Contributor status is by application; Maintainers own merges and releases unless those responsibilities are **explicitly delegated**. Prefer first name **Brad** only if referring to the primary maintainer in docs (no last name).
- Community tone: follow `CODE_OF_CONDUCT.md`; technical disagreement is fine; harassment and AI-shaming are not.

When changing contribution workflow, community rules, or project authority, update the root governance suite **and** keep this section accurate.

## WFL Fundamentals (Guiding Principles)
These 19 principles are the foundation of WFL's design. Every language, documentation, and tooling change should uphold them. Full descriptions live in `Docs/wfl-foundation.md`.

1. **Natural-Language Syntax** — Mirror natural language so code reads like English and lowers the learning curve.
2. **Minimize Special Characters** — Prefer words over symbols; use special characters only when they serve a clear purpose.
3. **Readability & Clarity** — Favor self-explanatory code over terse or cryptic expressions.
4. **Clear & Actionable Error Reporting** — Provide context-aware, Elm-inspired errors that suggest solutions.
5. **Type Safety & Compatibility** — Enforce strict type checking with inference to prevent runtime errors.
6. **Support for Modern Features** — Express async operations and pattern matching naturally.
7. **Interoperability with Web Standards** — Integrate seamlessly with JavaScript, CSS, and HTML.
8. **Built-in Security Features** — Embed secure defaults (e.g., output escaping) to prevent common vulnerabilities.
9. **Accessibility for Beginners** — Keep features approachable and easy to learn.
10. **Expressiveness for Experienced Developers** — Offer powerful, concise features for sophisticated code.
11. **Balanced Simplicity & Power** — Stay simple to use while retaining robust capabilities.
12. **Community & Collaboration** — Foster sharing and mutual learning through clear code.
13. **Performance Optimization** — Optimize transparently (short-circuiting, caching) without manual tuning.
14. **Integration with Standard Libraries** — Provide a comprehensive stdlib aligned with natural-language syntax.
15. **Scalability & Maintainability** — Support small scripts and large applications with modular structures.
16. **Gradual Learning Curve** — Introduce advanced concepts progressively.
17. **Error Transparency** — Make error handling and debugging straightforward and transparent.
18. **Encouragement of Best Practices** — Promote standards that yield high-quality, maintainable code.
19. **Avoidance of Unnecessary Conventions** — Challenge legacy conventions (e.g., mandatory semicolons) that lack clear justification.

### The No-Unlearning Invariant (Overarching Design Law)
WFL is deliberately both a "my first language" and a language strong enough for production. This only works as a *gradient*, not a *compromise* — the beginner path must be a subset of the expert path, with no cliffs between them. When principles appear to conflict, this invariant takes precedence:

> **For every feature, the beginner form and the expert form must be the same form, or connected by a smooth path with nothing to unlearn.**

Apply it as a test on every language, docs, or tooling change: if a beginner learns a habit that a production user must later undo — or must work around the language to do the most natural thing — that is a crack in the tightrope to fix, not to document. Terser expert forms are welcome only when a beginner can grow into them without unlearning the simple form. Full description in `Docs/wfl-foundation.md`.

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
- **Disk space (run `cargo clean` before every build)**: The build environment has a limited disk allowance and the `target/` tree (debug + release, with `debug = true` on release) grows to tens of GB, which causes `No space left on device` / linker `Bus error` failures. Run `cargo clean` before each build so a fresh build never runs out of space. (Trade-off: this forgoes incremental compilation, so every build is a full rebuild.)
- **Build**: `cargo clean && cargo build` (release: `cargo clean && cargo build --release`).
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
- **TDD is mandatory**: Write failing tests FIRST for any feature or bug fix.
- **Locations**:
  - Rust Unit/Integration: `tests/`
  - WFL End-to-End: `TestPrograms/` (must pass with release build)
  - WFL Test Framework: Use `describe`/`test` blocks, run with `wfl --test <file>`
- **Conventions**: feature‑oriented names (`*_test.rs`, `*.test.wfl`), keep perf benches under `benches/`.
- **Testing Guide**: See `Docs/guides/testing-guide.md` for WFL testing framework documentation.

## Commit & Pull Request Guidelines
- **Conventional Commits**: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`.
- **Pull Requests**: Clear description, linked issues, tests added/updated, repro steps.
- **Pre‑PR Checks**:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all --verbose`

## Documentation Development
- **Docs Are Part of the Feature (MANDATORY)**: Every change that adds, removes, or alters user-facing behavior — new/changed language syntax, keywords, statements, stdlib functions, CLI flags, or config options — MUST update or add the corresponding documentation in the **same change**. A feature is not complete until its docs are written. This includes:
  - The relevant guide under `Docs/` (e.g. a new statement → its section in the matching `Docs/04-advanced-features/*` or `Docs/05-standard-library/*` page).
  - Both keyword references (`Docs/reference/keyword-reference.md` and `Docs/reference/reserved-keywords.md`) when keywords are added or reclassified — update them together.
  - A working example (in `TestPrograms/`, validated with MCP) demonstrating the feature.
  - A Dev Diary entry in `Dev diary/` for any non-trivial feature or behavior change.
  - When a feature is removed or its syntax changes, remove or fix the now-stale docs and examples — don't leave contradictions.
- **Docs Must Be Honest — "validate docs" (MANDATORY)**: Documentation describes **what actually ships today**, not what is aspirational. This is a binding policy, not a preference:
  - **No overclaiming runtime behavior.** Never describe behavior the runtime does not have (e.g. calling serial request handlers "parallel" or saying they "don't block others"). Prefer the precise word — say "concurrent" (interleaved on one thread) vs "parallel" (multiple cores) deliberately, and describe the transport/handler split accurately.
  - **Mark planned/future behavior explicitly.** Anything not yet implemented must be labeled as planned/future so a reader never mistakes it for current behavior.
  - **Validate, don't just assert.** Every user-visible change ships **validated** docs (MCP tools + `python scripts/validate_docs_examples.py` for any touched example) **and** a Dev Diary entry, in the **same change**. "Validate docs" means both: the examples run, and the prose matches the implementation.
  - When behavior changes, fix the now-stale claims in the same change — a doc that contradicts the code is a bug.
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
- **Documentation**: MANDATORY — any added or changed feature MUST ship its docs in the same change (see "Documentation Development"). Keep `Docs/` current, validate ALL code examples with MCP before adding, and add a Dev Diary note for non-trivial changes.
- **Security**: Review `SECURITY.md`. Avoid logging secrets. Use zeroization. No public security issues.
- **Rules**: Refer to `.cursor/rules/wfl-rules.mdc`.

## Technical Requirements
- **Rust Edition**: 2024 (MSRV: 1.94+ — raised by the `sqlx` 0.9 dependency; Dev: 1.94+)
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

## Claude Code Hooks
- **Location**: `.claude/hooks/` (hook scripts), `.claude/settings.json` (configuration).
- **Auto-format**: Rust files are automatically formatted after Edit/Write operations via `PostToolUse` hook.
- **Prerequisites**:
  - **Windows PowerShell**: Default configuration (built into Windows).
  - **PowerShell Core (pwsh)**: Optional cross-platform alternative (requires installation).
  - **Bash**: Alternative hook available (`format-rust.sh`) for Unix/Linux/macOS/Git Bash.
- **Docs**: See `.claude/hooks/README.md` for configuration options and troubleshooting.
