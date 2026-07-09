# wflpkg ADR-001 — Binary & Crate Structure

**ADR ID:** `wflpkg-adr-001`
**Status:** Accepted
**Date:** 2026-07-08
**Owner:** Brad
**Relates to:** `wflpkg-brainstorm-results.md`, `wflpkg-open-decisions-resolved.md`, `wflpkg-manifest-grammar-1.0.md`

> **Decision in one line:** wflpkg ships as **one primary `wfl` binary** with package management exposed as **positional subcommands** (`wfl add`, `wfl why`, …); all logic lives in the **`wflpkg` library crate**; a **standalone `wflpkg` binary is retained as a thin alias** into that same library; and there is **exactly one manifest/lockfile parser**, built on WFL's single shared lexer. No second parser, ever.

---

## 1. Context

The three prior design documents settled *what* wflpkg is (a from-scratch package manager with a frozen WFL-native data-literal manifest, SemVer resolution via MVS, scoped namespaces, Sigstore/OIDC/transparency-log trust, and interpreter-enforced capabilities). They deliberately left one packaging question open: **how should the tool be shaped and distributed — as part of the `wfl` binary, or as a separate program?**

Two facts from the settled design constrain the answer:

1. **Condition 5 (Decision 1) is non-negotiable:** there must be *one shared, continuously-fuzzed parser used byte-identically by the compiler and every first-party tool.* The whole security value of the frozen data-literal grammar depends on the manifest being read by the **same** `logos` lexer the compiler uses, with only a subtractive acceptance layer on top. A second, independent parser reintroduces the parser-differential attack class the grammar exists to eliminate.
2. **The manifest grammar is versioned independently of the language** (SemVer `1.0.0` inside the file's version envelope, vs the language's `YY.MM.BUILD`). The distribution shape must not disturb that independence.

**Current repository reality (verified 2026-07-08):**

| Fact | State today | Implication |
|---|---|---|
| `crates/wflpkg` exists | Library crate **and** a `[[bin]] name = "wflpkg"` (`src/main.rs`) | Alias binary already exists structurally |
| Root `wfl` crate | Depends on `wflpkg` via `path = "crates/wflpkg"`; calls `wflpkg::commands::*` | Package logic is already library-first |
| Command surface | Positional subcommands (`wfl add`, `remove`, `create`, `build`, `run`, `share`, `search`, `info`, `login`, `check security`, …) | Flat-verb layout already in place |
| Standalone `wflpkg` main | Comment: "Delegates to the same library functions used by `wfl` subcommands" | Alias intent already present |
| **Parser** | `crates/wflpkg/src/manifest/parser.rs` is a **hand-rolled line parser** for the *old prose* dialect (`requires …`, `needs …`); crate depends on **neither `logos` nor `wfl_core`** | **Violates condition 5** — this is a second parser and must be replaced |
| `crates/wfl_core` | Currently holds only `version.rs`; the lexer lives in the root crate's `src/lexer/` | The shared lexer is not yet reachable by `wflpkg` without a dependency cycle |

So the *topology* the design wants is largely already present; the *trust-critical core* (a single shared parser) is not. This ADR records the target and the gap.

---

## 2. Decision

**2.1 One primary binary.** Package management is part of the `wfl` binary, exposed as positional subcommands. There is one tool for a user to learn and one executable on `PATH`. This is the No-Unlearning-friendly choice and matches how the codebase is already wired.

**2.2 Command surface = flat verbs, not a flag.** Use `wfl add`, `wfl why`, `wfl fmt`, `wfl manifest --json` (Cargo-style flat verbs), as the design docs already assume. A `wfl --pkg` *flag* is **rejected**: a flag that switches an entire command family does not compose with sub-verbs. (If grouping is ever wanted, `wfl pkg <verb>` is the sanctioned alternative — but flat verbs are the default.)

**2.3 All logic lives in the `wflpkg` library crate.** `crates/wflpkg` is the single home for manifest/lockfile handling, the MVS resolver (behind a trait boundary), hashing, confusable checks, and the registry client. The `wfl` subcommands are thin wrappers over this library — as they already are.

**2.4 The standalone `wflpkg` binary is retained as a thin alias.** It stays a ~entry-point-only `[[bin]]` that calls the *same* `wflpkg` library. It is an alias for environments that expect a `wflpkg` on `PATH`, **never a fork**. Because both entry points compile the same library and the same lexer, the security guarantee holds regardless of which one is invoked.

**2.5 Exactly one parser, built on the shared lexer.** All manifest and lockfile reading — in the compiler, in `wfl` subcommands, and in the `wflpkg` alias — goes through the single WFL `logos` lexer plus the subtractive frozen-grammar acceptance layer. To make the lexer reachable by `wflpkg` **without a dependency cycle** (today `wfl` → `wflpkg`), the shared lexer must live in a crate *below both* — the existing `crates/wfl_core` is its natural home. `wflpkg` then depends on `wfl_core`, and its current hand-rolled `manifest/parser.rs` and `lockfile/parser.rs` are deleted and replaced by the shared path.

---

## 3. Consequences

**Positive.**

- The "same source" guarantee becomes structural, not a discipline: with one shared library and one shared lexer crate, no build of `wfl` or `wflpkg` can ship a stale or divergent parser.
- The distribution shape (one primary tool, optional alias) is achievable at near-zero cost because the workspace already has the library, the subcommands, and the alias bin.
- Independent grammar versioning is unaffected: bundling into `wfl` only means they *release together*; the grammar's own SemVer and the CI drift oracle still govern the manifest format.

**Required refactor (the gap to close).**

1. **Extract the WFL lexer into `wfl_core`** (or another crate below both `wfl` and `wflpkg`) so both can share it with no dependency cycle.
2. **Delete `crates/wflpkg/src/manifest/parser.rs` and `.../lockfile/parser.rs`** (hand-rolled, old-prose-dialect, comment-skipping) and route parsing through the shared lexer + the frozen data-literal acceptance layer from `wflpkg-manifest-grammar-1.0.md`.
3. **Add `wfl_core` (and the acceptance layer) as `wflpkg` dependencies**; remove any bespoke tokenizing.
4. **Wire the differential fuzz harness** (grammar doc §10) across *both* entry points so the compiler-embedded parser and the standalone tool are proven to return byte-identical trees and identical error codes.

**Liability to keep on the record.** Retaining the `wflpkg` alias binary is cheap but not free: it is a second entry point to keep tested. It earns its place only as an alias; the moment it grows any parsing or verification logic of its own, this ADR is violated.

---

## 4. Alternatives considered

**A. A separate, self-contained `wflpkg` program (its own parser).** Rejected. This is the "second parser" that condition 5 forbids; it recreates the parser-differential attack surface the entire grammar was designed to remove.

**B. `wfl --pkg` flag.** Rejected. Does not compose with sub-verbs; awkward (`wfl --pkg add …`) and non-idiomatic.

**C. wflpkg written as a WFL script (dogfooding).** Rejected for the trust-critical core: it would either reimplement the parser in WFL (a second parser) or shell back into the Rust parser (a redundant runtime with no gain). WFL scripting is acceptable only for non-trust orchestration/UX that never re-parses a manifest or re-verifies a hash — and even that is optional.

**D. Drop the standalone `wflpkg` binary entirely.** Viable, but the alias is nearly free given the existing layout and buys compatibility for environments expecting a `wflpkg` command. Kept as an optional thin alias rather than removed.

---

## 5. Follow-up actions

1. Extract the lexer into `wfl_core`; make `wfl` and `wflpkg` both depend on it.
2. Replace `wflpkg`'s hand-rolled manifest/lockfile parsers with the shared-lexer acceptance layer (frozen grammar `1.0.0`).
3. Add the differential fuzz harness across both entry points (subset + equivalence, byte-identical parse, identical error codes).
4. Keep `[[bin]] wflpkg` as an alias-only entry point; add a test asserting it carries no parsing/verification logic of its own.
5. Confirm `wfl fmt` and `wfl manifest --json` are served from the same library path.

---

*Provenance: recorded 2026-07-08 following the wflpkg design discussion. Grounded in the mounted WFL tree (`crates/wflpkg`, `crates/wfl_core`, root `Cargo.toml`, `src/main.rs`). Continues the three wflpkg design documents; supersedes nothing — it fills the one packaging-topology question those documents deliberately left open.*
