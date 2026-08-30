#!/usr/bin/env bash
# Cloud Agent install script for WFL.
#
# Idempotent repository bootstrap run after the source tree is checked out.
# It prepares the exact toolchain and build state a Cloud Agent needs to build,
# test, lint, and run WFL end to end:
#
#   * a stable Rust toolchain that satisfies the crate's rust-version (>= 1.94,
#     required by edition 2024 and the sqlx 0.9 dependency), with the rustfmt
#     and clippy components the CI gates use;
#   * all Cargo dependencies fetched against the committed Cargo.lock;
#   * the release `wfl` binary, which the integration and web-server test
#     runners (scripts/run_integration_tests.sh, scripts/run_web_tests.sh)
#     require to exist.
#
# It starts no long-running processes: WFL has no always-on dev server (the web
# server is launched per-program by `listen`), so nothing belongs in `start`.
set -euo pipefail

# Run from the repository root regardless of the caller's working directory.
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Minimum Rust version required by Cargo.toml (`rust-version`).
required_rust_minor=94

echo "==> Ensuring a stable Rust toolchain (>= 1.${required_rust_minor}) is installed"
if ! command -v rustup >/dev/null 2>&1; then
    echo "error: rustup is not available on PATH; the base image must provide the Rust toolchain installer." >&2
    exit 1
fi

# The `default` profile pulls in rustfmt and clippy, matching the CI toolchain
# (dtolnay/rust-toolchain@stable). Re-running is a cheap no-op once installed.
rustup toolchain install stable --profile default --no-self-update
rustup default stable

# Fail fast with a clear message if the resolved stable is somehow older than
# the crate's MSRV, instead of dying deep in a compile.
rust_minor="$(rustc +stable --version | sed -E 's/^rustc 1\.([0-9]+)\..*/\1/')"
if [ "${rust_minor:-0}" -lt "$required_rust_minor" ]; then
    echo "error: stable Rust is 1.${rust_minor}, but WFL requires >= 1.${required_rust_minor}." >&2
    exit 1
fi
echo "==> Using $(rustc --version)"

echo "==> Fetching Cargo dependencies (locked to Cargo.lock)"
cargo fetch --locked

# Build the release binary the integration/web test runners depend on. The
# release profile keeps debuginfo (Cargo.toml sets debug = true), so this is the
# largest single step; running it here bakes the binary into the environment so
# fresh agents start ready to run WFL programs.
echo "==> Building the release wfl binary"
cargo build --release --locked

echo "==> WFL install complete: $(./target/release/wfl --version)"
