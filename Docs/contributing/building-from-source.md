# Building WFL from Source

Compile WFL on your machine, run tests, and use a local binary. You do not need this for everyday WFL programming if you already have an installer or release build—this guide is for contributors and anyone who wants the latest source.

## Prerequisites

| Tool | Notes |
|------|--------|
| **Rust** | 1.94 or later (raised by the `sqlx` 0.9 dependency) |
| **Cargo** | Comes with Rust via rustup |
| **Git** | To clone the repository |

### Install Rust

**Windows, macOS, Linux** — use [rustup](https://rustup.rs/):

```bash
# Unix / Git Bash / WSL example:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify
rustc --version
cargo --version
```

On Windows you can also install via the rustup installer executable from the same site.

## Clone the Repository

```bash
git clone https://github.com/WebFirstLanguage/wfl.git
cd wfl
```

## Build Commands

### Development build

Faster compile, slower runtime—good while iterating on the compiler:

```bash
cargo build
```

| OS | Binary |
|----|--------|
| Windows | `target\debug\wfl.exe` |
| Linux / macOS | `target/debug/wfl` |

### Release build

Slower compile, optimized runtime—**required for integration tests and TestPrograms**:

```bash
cargo build --release
```

| OS | Binary |
|----|--------|
| Windows | `target\release\wfl.exe` |
| Linux / macOS | `target/release/wfl` |

### Build the LSP server

```bash
cargo build --release -p wfl-lsp
```

| OS | Binary |
|----|--------|
| Windows | `target\release\wfl-lsp.exe` |
| Linux / macOS | `target/release/wfl-lsp` |

## Running Tests

```bash
# Unit and most integration tests
cargo test

# One test by name
cargo test test_name

# Show println! output
cargo test -- --nocapture

# End-to-end scripts (need release binary first)
cargo build --release
./scripts/run_integration_tests.ps1   # Windows PowerShell
./scripts/run_integration_tests.sh    # Linux / macOS
```

Web server tests have their own scripts: `scripts/run_web_tests.ps1` / `.sh`.

## Code Quality

WFL’s own codebase uses the same quality bar we recommend for WFL programs: clear structure, automated checks, no silent breakage.

```bash
# Format Rust sources
cargo fmt --all

# Lint (warnings are errors in CI)
cargo clippy --all-targets --all-features -- -D warnings

# Check formatting without writing
cargo fmt --all -- --check
```

## Running WFL Locally

```bash
# Run a program (use path style for your OS)
./target/release/wfl program.wfl          # Unix
.\target\release\wfl.exe program.wfl      # Windows PowerShell

# Interactive REPL
./target/release/wfl

# Useful flags
./target/release/wfl --lint program.wfl
./target/release/wfl --analyze program.wfl
./target/release/wfl --time program.wfl
./target/release/wfl --test program.wfl
```

## Troubleshooting

### Build fails

```bash
rustc --version   # need 1.75+
rustup update
cargo clean
cargo build --release
```

### First build is slow

| Build | Typical time |
|-------|----------------|
| First release | ~8–15 minutes (downloads and compiles dependencies) |
| Later builds | ~1–2 minutes when little changed |

That is normal for a Rust project of this size.

### Integration tests fail with “binary not found”

Build release first, then re-run tests or scripts:

```bash
cargo build --release
cargo test
```

## Next Steps

- [Contributing Guide](contributing-guide.md) — how patches are reviewed
- [Architecture Overview](architecture-overview.md) — pipeline from source to execution
- [LSP Integration](lsp-integration.md) · [MCP Integration](mcp-integration.md) — editor and AI tooling

---

**Previous:** [← Development](index.md) | **Next:** [Contributing Guide →](contributing-guide.md)
