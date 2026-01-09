# Building WFL from Source

Complete guide to compiling WFL from source code.

## Prerequisites

- **Rust** 1.75 or later
- **Cargo** (included with Rust)
- **Git**

### Install Rust

```bash
# Visit https://rustup.rs/ or run:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

You need Rust 1.75+. Development uses 1.91.1+.

## Clone Repository

```bash
git clone https://github.com/WebFirstLanguage/wfl.git
cd wfl
```

## Build Commands

### Development Build

**Fast compilation, slower runtime:**

```bash
cargo build
```

**Binary location:**
- Windows: `target\debug\wfl.exe`
- Linux/macOS: `target/debug/wfl`

### Release Build

**Slower compilation, fast runtime:**

```bash
cargo build --release
```

**Binary location:**
- Windows: `target\release\wfl.exe`
- Linux/macOS: `target/release/wfl`

**Note:** Integration tests require release build!

### Build LSP Server

```bash
cargo build --release -p wfl-lsp
```

**Binary location:**
- Windows: `target\release\wfl-lsp.exe`
- Linux/macOS: `target/release/wfl-lsp`

## Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture

# Integration tests (requires release build)
cargo build --release
./scripts/run_integration_tests.ps1  # Windows
./scripts/run_integration_tests.sh   # Linux/macOS
```

## Code Quality

```bash
# Format code
cargo fmt --all

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Check formatting
cargo fmt --all -- --check
```

## Running WFL

```bash
# Run program
./target/release/wfl program.wfl

# Start REPL
./target/release/wfl

# With flags
./target/release/wfl --lint program.wfl
./target/release/wfl --analyze program.wfl
./target/release/wfl --time program.wfl
```

## Troubleshooting

### Build Fails

**Check Rust version:**
```bash
rustc --version  # Need 1.75+
rustup update
```

**Clean and rebuild:**
```bash
cargo clean
cargo build --release
```

### Long Build Times

**First build:** 8-15 minutes (compiles all dependencies)
**Subsequent builds:** 1-2 minutes

This is normal for Rust projects.

### Integration Tests Fail

**Cause:** Release binary not built.

**Fix:**
```bash
cargo build --release
cargo test
```

---

**Previous:** [← Development](index.md) | **Next:** [Contributing Guide →](contributing-guide.md)
