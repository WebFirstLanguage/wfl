# Supported Platforms & Support Boundaries

This document defines the platforms WFL supports, what "supported" means, and
the boundaries of that support. It is the reference for the **Maintenance** and
**Operations** dimensions of the production-readiness plan
([issue #610](https://github.com/WebFirstLanguage/wfl/issues/610)) and for the
mandatory release gate *"Release artifacts, checksums, installation, upgrade,
rollback, supported-platform, and known-limitations documentation are
published."*

> **Status:** WFL is currently **alpha** software (see [`SECURITY.md`](../../SECURITY.md)).
> The support tiers below describe what the project tests and stands behind
> *today*; they tighten as WFL approaches its 8/10 production-readiness gate.

## Support tiers

WFL uses three tiers. A platform's tier is defined by **what CI actually
exercises**, not by aspiration.

| Tier | Meaning | What you can rely on |
|---|---|---|
| **Tier 1 — Supported** | Built **and** tested on every PR in CI. | A release binary is built and the end-to-end `TestPrograms` + Rust integration tests (`cargo test --test '*'`) run on **every** Tier-1 platform; regressions block merges. Coverage is **not identical** across Tier-1 platforms — see *Per-platform PR CI coverage* below for the exact lanes each one runs. |
| **Tier 2 — Best-effort** | Expected to build from source; **not** covered by CI. | The code targets it and contributors run it, but breakage is possible between releases and is fixed on a best-effort basis. |
| **Unsupported** | Not built, not tested, not a goal for the 8/10 release. | May work, may not. No guarantees, no gate coverage. |

## Platform matrix

| Platform | Architecture | Tier | Evidence / notes |
|---|---|---|---|
| **Linux (glibc)** | `x86_64` | **Tier 1** | `ci.yml` builds + tests on `ubuntu-latest`: unit/integration tests, Clippy (`-D warnings`), database tests (PostgreSQL + MariaDB), and the `TestPrograms` runner. |
| **Windows** | `x86_64` (`x86_64-pc-windows-msvc`) | **Tier 1** | `ci.yml` runs the integration + `TestPrograms` matrix on `windows-latest`. The MSI installer (`cargo-wix`) and its smoke test run in `nightly.yml` **after** merge, not on PRs. |
| **macOS** | `x86_64`, `aarch64` (Apple Silicon) | **Tier 2** | Builds from source ([`installation.md`](../02-getting-started/installation.md) documents the flow) but is **not** in CI. Supported best-effort until a macOS CI lane is added. |
| **Linux (musl / non-glibc)** | any | **Tier 2** | No CI lane; static-musl builds are expected to work but unverified. |
| **Linux / other Unix** | `aarch64`, others | **Tier 2** | Pure-Rust with a Tokio runtime; expected to build where the toolchain and dependencies do. Unverified. |
| **32-bit targets** | `i686`, `armv7`, … | **Unsupported** | Not built or tested. The interpreter runs on a large (1 GiB) call stack thread and assumes 64-bit address space. |

**Promotion policy.** A Tier 2 platform is promoted to Tier 1 only when a CI lane
builds it and runs the integration + `TestPrograms` suites green — a
before-the-release-gate requirement, not a documentation change.

### Per-platform PR CI coverage (what actually runs today)

Tier-1 coverage is **not symmetric**. This table lists exactly what each Tier-1
platform runs on a pull request, per `.github/workflows/ci.yml`:

| Lane | Linux (`ubuntu-latest`) | Windows (`windows-latest`) |
|---|---|---|
| `cargo fmt --check` | ✅ | ➖ (Linux only) |
| Full `cargo test` (unit + integration) | ✅ | ➖ (Linux only) |
| LSP build + tests | ✅ | ➖ (Linux only) |
| Clippy `-D warnings` | ✅ | ➖ (Linux only) |
| Database tests (PostgreSQL + MariaDB) | ✅ | ➖ (Linux only) |
| Rust integration tests (`cargo test --test '*'`) | ✅ | ✅ |
| `TestPrograms` end-to-end runner | ✅ | ✅ |
| Release **artifact publish** (checksums, installers) | ➖ | ➖ (nightly/post-merge only) |
| Documentation-example execution | ➖ (not wired into CI yet — mandatory gate still open) | ➖ |

Known gaps that are **not** yet gated on any PR: the full unit/LSP/clippy/DB
suite runs on Linux only; installer testing is nightly and post-merge; release
artifacts are not published from PR CI; documentation examples are not executed
in CI; and the declared MSRV is not verified (see below). These are tracked
Phase 1→3 items, not guarantees.

## Toolchain requirements

| Requirement | Value | Source of truth |
|---|---|---|
| Rust channel | **stable** | All CI jobs use `dtolnay/rust-toolchain@stable`. |
| Minimum supported Rust version (MSRV) | **1.88** (declared) | `Cargo.toml` `rust-version = "1.88"`. The codebase uses `let`-chains (stabilized in 1.88), so older toolchains fail fast via `cargo`'s check. Note: CI builds on **stable**, so the 1.88 floor is *declared but not gate-tested* — an MSRV lane is a tracked follow-up. |
| Rust edition | **2024** | `Cargo.toml` `edition = "2024"`. |
| Build profiles | `debug`, `release` | Integration tests and `TestPrograms` require a `cargo build --release` binary. |
| Disallowed | `panic = "abort"` | CI asserts the release binary rejects `panic=abort` (`ci.yml`), so panics stay unwindable/catchable. |

## Runtime requirements

- **Async runtime:** Tokio (`tokio` "full"). WFL's interpreter is async and
  drives the runtime on a dedicated large-stack thread.
- **Filesystem:** required for source loading, `include from` / `load module`
  module resolution, and filesystem stdlib operations.
- **Network:** required only for programs that use HTTP (`reqwest`), the web
  server (`warp`), or databases (`sqlx`: SQLite/MySQL/PostgreSQL). No network is
  needed to run a plain script.
- **Resource ceilings:** every run is governed by the shared
  [`ExecutionBudget`](../../src/exec/budget.rs) (recursion/import depth, pattern
  steps/states, source/body/response bytes, HTTP/WebSocket queues and
  connections, and optional operation/wall-clock ceilings). Defaults are
  documented in [`configuration-reference.md`](configuration-reference.md).

## What "supported" covers — and what it does not

On a **Tier 1** platform, the project commits to WFL being *predictable,
testable, documented, and operable* for **supported language behaviour**:

- Supported language constructs behave consistently across the parser, analyzer,
  type checker, and interpreter.
- The documented CLI, `.wflcfg` configuration, and standard-library surface work
  as described. (Automated execution of documentation examples in CI is a
  mandatory release gate that is **not yet met** — examples are validated
  locally via `scripts/validate_docs_examples.py` today.)
- Release artifacts are produced by the nightly/release workflows (not from PR
  CI) and installable via the documented paths; verifiable checksums are a
  tracked Operations follow-up.

Support **does not** extend to:

- **Untrusted-code sandboxing.** WFL runs the programs you give it; the
  `ExecutionBudget` bounds resource *exhaustion*, but WFL is **not** a sandbox
  for hostile code. See [`SECURITY.md`](../../SECURITY.md) → *Known Security
  Limitations*.
- **Aspirational / unimplemented syntax.** Anything marked planned/future in the
  docs is explicitly outside the supported surface until implemented.
- **Tier 2 / Unsupported platforms**, per the matrix above.
- **End-of-life versions**, per the version-support matrix in
  [`SECURITY.md`](../../SECURITY.md) → *Supported Versions*.

## Versioning, compatibility & lifecycle

- **Versioning:** calendar-based `YY.MM.BUILD` (e.g. `26.7.36`). The major
  component stays `< 256` for Windows MSI compatibility.
- **Security-update lifecycle:** the current minor line receives prioritized
  fixes; older lines get critical-only or no updates. The authoritative matrix
  lives in [`SECURITY.md`](../../SECURITY.md).
- **Compatibility & breaking changes:** backward compatibility for supported
  language behaviour is protected by [`GOVERNANCE.md`](../../GOVERNANCE.md)
  (§3.1/§3.2 stability policy; §2.2 breaking-change authority). Breaking a
  supported program requires the documented deprecation path.

## Reporting a platform problem

- **Build/runtime bug on a Tier 1 platform:** file a normal issue with the
  platform, architecture, Rust version (`rustc --version`), and a minimal
  reproduction.
- **Security issue:** do **not** open a public issue — follow
  [`SECURITY.md`](../../SECURITY.md) (private advisory or email).

---

*Maintained as part of the production-readiness effort (issue #610, Phase 1).
Update the platform matrix whenever a CI lane is added or removed, and keep the
tiers in sync with `.github/workflows/`.*
