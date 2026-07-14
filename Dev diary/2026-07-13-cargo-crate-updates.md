# Dev Diary — 2026-07-13: Cargo dependency refresh

## Context

A sweep of the workspace dependencies (`Cargo.toml` at the root, `wfl-lsp/`, and
`crates/wflpkg/`) to move every crate to its latest release, verify the build and
test suites still pass, and fix the breaking changes the upgrades introduced.

The lockfile was already at the newest semver-compatible versions (`cargo update`
found nothing to bump), so every remaining update was a major-version jump that
required a manifest change and, in several cases, source changes.

## What changed

### Straightforward version bumps (no code changes)

`regex 1.13`, `log 0.4.33`, `tokio 1.52`, `serde_json 1.0.150`, `uuid 1.23`,
`bytes 1.12`, `futures-util 0.3.32`, `codespan-reporting`* , `chrono 0.4.45`,
`once_cell 1.21.4`, `time 0.3.53`, `zeroize 1.9`, `tempfile 3.27`, `libc 0.2.186`,
`rpassword 7.5.4`, `tokio-test 0.4.5`, `rustyline 12 → 18`, `criterion 0.4 → 0.8`,
`dashmap 5 → 6`, `env_logger 0.10 → 0.11`.

### Migrations that required source changes

- **`bcrypt 0.15 → 0.19`** — public API (`hash`, `verify`, `DEFAULT_COST`)
  unchanged; upgraded in isolation from the rest of the crypto stack.
- **`rand 0.9 → 0.10`** — the core trait `RngCore` was renamed to `Rng` (it now
  carries `fill_bytes`). Updated the three local `use rand::RngCore;` imports in
  `src/stdlib/crypto.rs` to `use rand::Rng;`. `src/stdlib/random.rs` already
  pulls the prelude, so it needed no change.
- **`logos 0.15 → 0.16`** — 0.16 adds an unbounded-greedy-repetition lint that
  flags the line-comment skip pattern (`[^\r\n]*`). The pattern is intentional
  and bounded to a single line, so the skip attribute now uses the group form
  with `allow_greedy = true`.
- **`codespan-reporting 0.11 → 0.13`** — `term::emit` is deprecated; switched the
  call sites (one in `src/diagnostics/mod.rs`, six in `src/repl.rs`) to
  `term::emit_to_write_style`, the color-preserving replacement (both
  `StandardStream` and `Buffer` satisfy the new `WriteStyle` blanket impl via
  `termcolor::WriteColor`).
- **`reqwest 0.11 → 0.13`** — no source changes; the client/request/response/
  multipart APIs used in `src/interpreter/mod.rs` and the `wflpkg` registry are
  unchanged. The default TLS backend is now rustls via the platform verifier.
- **rustls crypto provider** — with reqwest 0.13 (aws-lc-rs) and sqlx 0.9
  (`tls-rustls-ring`) both pulling `rustls 0.23`, that single rustls is compiled
  with *two* providers. reqwest and sqlx each configure their own provider
  explicitly, so HTTPS and DB TLS work today (verified with a live HTTPS
  request), but rustls 0.23 panics if any code builds a config from the *ambient*
  default while more than one provider is present. As a defensive measure `main`
  now installs a process-level default once at startup
  (`rustls::crypto::ring::default_provider().install_default()`), added as a
  direct `rustls` dependency (ring feature only). This only affects the `wfl`
  binary, which is the one linking both providers.
- **`sqlx 0.8 → 0.9`** — this bump also **raises the workspace MSRV to 1.94**:
  `sqlx 0.9.0` declares `rust-version = "1.94.0"`, and it is the only updated
  crate that needs more than 1.85, so it sets the effective floor. WFL previously
  advertised MSRV 1.88 (dev 1.91.1); building on that toolchain would now fail in
  cargo. The declared MSRV was raised to 1.94 in `Cargo.toml` and in the docs
  (`CLAUDE.md`, `AGENTS.md`, `Docs/development/building-from-source.md`,
  `Docs/reference/supported-platforms.md`). CI uses `dtolnay/rust-toolchain@stable`
  (currently ≥1.94), so no workflow change was needed. Three breaking changes were
  handled in `src/interpreter/database.rs`:
  1. The `runtime-tokio-rustls` feature was split into a separate runtime and TLS
     backend; the manifest now uses `runtime-tokio` + `tls-rustls`
     (`tls-rustls` aliases the ring-backed rustls stack as before).
  2. `Database::Arguments` lost its lifetime parameter (GAT removal); the
     `bind_param!` macro now writes `<$db as sqlx::Database>::Arguments`.
  3. `sqlx::query()` is now gated behind the `SqlSafeStr` trait. WFL runs
     program-author SQL with all values passed via `.bind()`, so the six call
     sites wrap the query text in `sqlx::AssertSqlSafe(...)`.
- **`rcgen 0.13 → 0.14`** *(dev-dep)* — `CertifiedKey.key_pair` was renamed to
  `signing_key` in `tests/web_server_tls_test.rs`.
- **`tokio-tungstenite 0.21 → 0.30`** *(dev-dep)* — `Message::Text` now carries a
  `Utf8Bytes` instead of `String`; updated `tests/websocket_test.rs` to build
  frames with `.into()` and read them with `.to_string()`.
- **`criterion 0.8`** *(dev-dep)* — `criterion::black_box` is deprecated; the
  benches now use `std::hint::black_box`.

## Deliberately held back

- **warp — held at 0.3.7.** warp 0.4 removed the `tls` feature entirely (its TLS
  code is gated behind a `tls` feature that is no longer declared, so HTTPS is not
  compilable). WFL's web server supports `listen ... secured with certificate ...
  and key ...`, exercised by `tests/web_server_tls_test.rs`. Upgrading would drop
  HTTPS support — a backward-compatibility break. warp 0.4 also makes
  `server`/`websocket` opt-in and moves to hyper 1.0, which would be a large
  migration of the web-server/WebSocket code on top of the TLS regression.
- **The RustCrypto password-hash wave — held.** `sha2 0.10`, `hmac 0.12`,
  `hkdf 0.12`, `pbkdf2 0.12`, `scrypt 0.11`, `argon2 0.5`. `src/stdlib/crypto.rs`
  shares a single `PasswordHasher`/`PasswordVerifier` trait (imported from
  `argon2::password_hash`) across Argon2, Scrypt, and PBKDF2, and shares the
  `digest` traits across sha2/hmac/hkdf/pbkdf2. Moving pbkdf2/scrypt to the
  `digest 0.11` / `password-hash 0.6` wave requires argon2 to move too, but
  **there is no `argon2 0.6`** — its latest release (0.5.3) is still on
  `password-hash 0.5`. Upgrading piecemeal would split the tree into two
  incompatible `password-hash` versions in a security-sensitive module. Revisit
  when argon2 ships a `password-hash 0.6` release. (`bcrypt`, which uses its own
  MCF format rather than `password-hash`, was upgraded independently.)
- **`num-bigint-dig` — held at 0.8.6.** It is a forced direct pin that exists only
  to steer the resolver for the transitive `rsa` dependency (via `sqlx-mysql`).
  `rsa 0.9.10` requires `num-bigint-dig 0.8`; forcing 0.9 would just add a
  redundant unused second copy alongside rsa's 0.8. Revisit when `rsa`/`sqlx`
  adopt 0.9.

## Verification

- `cargo fmt --all -- --check` — clean.
- `cargo clippy -p wfl --all-targets --all-features -- -D warnings` — clean (all
  the migration changes live in the `wfl` package).
- `cargo test --workspace` — **1480 passed, 0 failed, 25 ignored** (95 binaries).
- `TestPrograms/*.wfl` against the release binary — **107 passed, 0 failed,
  22 skipped** (skips are the web/websocket programs handled by dedicated scripts
  plus CI-SKIP directives).

### Note on pre-existing clippy lints in test files

With the MSRV now at 1.94, clippy 1.94 flags some **pre-existing** style lints
(`field_reassign_with_default`, `manual_map`, stricter `dead_code`) in
**test files that this change does not touch** — `crates/wflpkg/tests/…` and
`wfl-lsp/tests/…`. These reproduce identically on a clean checkout (verified by
stashing this change and re-running clippy on `HEAD`), so they are not migration
regressions and were left out of this dependency-focused change; they are worth a
separate small cleanup PR now that 1.94 is the supported toolchain.

*`codespan-reporting` is a source-changed migration; see the list above.
