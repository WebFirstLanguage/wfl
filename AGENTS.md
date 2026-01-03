# Repository Guidelines

## Project Structure & Modules
- `src/`: Core compiler/runtime (`main.rs`, `lib.rs`, `repl.rs`, `builtins.rs`).
- `tests/`: Rust integration/unit tests (e.g., `file_io_*`, `crypto_test.rs`).
- `TestPrograms/`: End‑to‑end WFL programs that must all pass.
- `wfl-lsp/`: Language Server workspace member; `vscode-extension/` for VS Code.
- `Docs/`: Guides and technical notes (see `Docs/guides/building.md`).
- `scripts/`: Utilities (`run_integration_tests.ps1|.sh`, `configure_lsp.ps1`).
- `.cursor/rules/`: Cursor IDE rules and guidelines (`wfl-rules.mdc`).

## Build, Test, and Dev Commands
- Build: `cargo build` (release: `cargo build --release`).
- Run: `cargo run -- <file.wfl>` or `target/release/wfl <file.wfl>`.
- Test: `cargo test`; integration requires release binary. Windows: `./scripts/run_integration_tests.ps1` (Linux/macOS: `.sh`).
- Bench: `cargo bench` (Criterion).
- WFL CLI:
  - `wfl --lint` / `wfl --lint --fix [--in-place|--diff]`: Lint and fix code.
  - `wfl --debug`: Debug WFL execution.
  - `wfl --step`: Run in single-step execution mode.
  - `wfl --time`: Measure execution time.
  - `wfl --lex` / `wfl --parse`: Dump tokens or AST.
  - `wfl --configCheck` / `wfl --configFix`: Check/fix configuration.
  - `wfl --dump-env`: Dump environment for troubleshooting.
- LSP/VS Code: `./scripts/configure_lsp.ps1` and `scripts/install_vscode_extension.ps1`.

## Coding Style & Naming
- Rust 2024; format with `cargo fmt --all` (see `.rustfmt.toml`).
- Lint clean: `cargo clippy --all-targets --all-features -- -D warnings`.
- Naming: `snake_case` (fns/files), `CamelCase` (types/traits), `SCREAMING_SNAKE_CASE` (consts).

## Testing Guidelines
- TDD is mandatory: write failing tests first (see `.cursor/rules/wfl-rules.mdc`).
- Locations: unit/integration in `tests/`; E2E in `TestPrograms/` with release build.
- Conventions: feature‑oriented names (`*_test.rs`), keep perf benches under `benches/`.

## Commit & Pull Request Guidelines
- Prefer Conventional Commits (`feat:`, `fix:`, `docs:`, `test:`, `refactor:`). Version bumps may include `[skip ci]`.
- PRs: clear description, linked issues, tests added/updated, repro steps for fixes.
- Pre‑PR checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all --verbose`. CI must be green.

## Agent‑Specific Policies
- Backward compatibility is sacred: do not break existing WFL programs; run all `TestPrograms/`.
- Integration tests require `cargo build --release` and provided scripts.
- Keep docs current; update `Docs/` and relevant indexes when adding features. Major changes warrant a Dev Diary note.
- For security, review `SECURITY.md`; avoid logging secrets and prefer zeroization for sensitive data.
- Refer to `.cursor/rules/wfl-rules.mdc` for additional project-specific rules.

## LSP Development Workflow
- Location: LSP crate in `wfl-lsp/`; VS Code extension in `vscode-extension/`.
- Build/Run: `cargo build -p wfl-lsp`; dev run via `cargo run -p wfl-lsp` (stdio by default; see guide for flags).
- Editor setup: `scripts/configure_lsp.ps1` and `scripts/install_vscode_extension.ps1` wire VS Code to the LSP.
- Logging: enable trace logs with `RUST_LOG=trace cargo run -p wfl-lsp` (PowerShell: `$env:RUST_LOG='trace'; cargo run -p wfl-lsp`).
- Integration: many LSP features rely on the compiler; ensure `cargo build --release` provides `target/release/wfl`.
- Docs: see `Docs/guides/wfl-lsp-guide.md` and `Docs/guides/wfl-lsp-quick-reference.md` for protocol details and troubleshooting.
