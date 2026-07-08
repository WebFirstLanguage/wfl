# wflpkg: Frozen Data-Literal Manifest Grammar, Shared Lexer, and Tooling

**Date:** 2026-07-08

## What Changed

The WFL package manager (`wflpkg`) moved off its old hand-rolled *prose*
manifest parser (`requires http-client 26.1 or newer`, `needs file-access`)
and onto a **frozen data-literal grammar**: a small, restricted, deny-by-default
subset of WFL that shares the compiler's one `logos` lexer byte-for-byte. This
implements the three resolved design documents:

- `wflpkg-open-decisions-resolved.md` — Decision 1 (WFL-native typed data
  literal as the canonical manifest format) and Decision 2 (name-based identity
  with an AST-hash anchor; the `ast_hash` field committed to the lockfile
  schema now).
- `wflpkg-manifest-grammar-1.0.md` — the frozen grammar `1.0.0`: admitted node
  set, the Gate B/L/S/I rejection taxonomy with stable `MG-*` codes, the
  canonical `wfl fmt` form, the JCS JSON projection, and the two SHA-256 hashes.
- `wflpkg-adr-001-binary-and-crate-structure.md` — one primary `wfl` binary,
  the `wflpkg` library, a thin `wflpkg` alias, and **exactly one parser** built
  on the shared lexer.

A manifest is now a **value, never a program**: it is deserialized, never
executed.

### 1. Shared lexer extracted into `wfl_core`

The `logos` lexer moved from `src/lexer/` into a new `crates/wfl_core` crate
that sits *below* both `wfl` and `wflpkg`, so the package manager reads
manifests with the exact tokenizer the compiler uses (Decision 1, condition 5).
The root crate keeps a one-line re-export shim (`pub use wfl_core::lexer::*`),
so all ~60 existing `crate::lexer::…` call sites and `wfl::lexer::…` (LSP) keep
resolving unchanged.

**Normative bug-fix (grammar §5):** the lexer's `IntLiteral` callback was
`parse::<i64>().unwrap()`, which *panicked* on an over-`i64` integer literal —
a crash the manifest parser must not inherit. It now returns `None` on
overflow, so an over-long integer is a graceful lexer error, not an abort. This
also removes a latent crash in the compiler at large (an over-`i64` literal
already had no valid meaning).

### 2. The frozen grammar (`crates/wflpkg/src/datalit/`)

A subtractive acceptance layer over the shared token stream:

- **Gate B** (`bytegate.rs`) — UTF-8 well-formedness (overlong/surrogate
  classified precisely), no BOM, LF-only, NFC (reject-don't-normalize), size.
- **Gate L + Gate S** (`parser.rs`) — one pass over the lexer's `(token, span)`
  stream. Every spelling-sensitive rejection reads the **raw span bytes**, not
  the token payload, because the lexer erases boolean case, null spelling, and
  integer leading zeros before the token exists. Records are
  `create map <kind>: … end map`; values are string / whole-number / `yes`/`no`
  / list-of-scalars only. Comments, floats, nulls, references, arithmetic,
  duplicate keys, and bare reserved-keyword keys are all rejected.
- **Gate I** (`identity.rs`) — `scope`/`name` are constrained to the ASCII-Only
  allowlist `[a-z][a-z0-9]*(-[a-z0-9]+)*`; `version` to the SemVer / constraint
  grammar; a UTS #39 tripwire is wired but vacuous under ASCII-only.
- **Canonical form / interop** (`fmt.rs`, `json.rs`, `hash.rs`) — `wfl fmt`
  byte-deterministic output; the RFC 8785 (JCS) JSON projection; `file_hash`
  (over the `fmt` bytes) and `content_hash` (over the JCS projection), both
  SHA-256, algorithm-tagged.

Every `MG-*` error code is a stable part of the versioned spec and is exercised
by an in-repo malicious-input reject corpus (grammar §10), plus determinism,
idempotence, round-trip, collapse-freedom, and a **drift oracle** that pins the
lexer's admitted token surface so a lexer change turns the build red.

### 3. Schema, manifest, and lockfile

`ProjectManifest` and `LockFile` are now *schema views* over a parsed
`datalit::Document`. Manifests are `wflpkg` (version envelope) + `package` +
repeated `dependency` records; lockfiles are `wflpkg` + repeated `locked`
records carrying `hash` and the day-one-committed `ast_hash` field. Reading and
writing both go through the one shared parser and `wfl fmt`.

### 4. CLI

`wfl fmt [file] [--check]`, `wfl manifest [file] --json`, and
`wfl manifest [file] --hash` are served from one library path and mirrored by
the thin `wflpkg` alias. A test asserts the alias carries no parsing or
verification logic of its own (ADR-001 §5.4).

## Notable Finding: a pre-existing lexer stack-overflow (DoS)

Writing the §10 reject corpus surfaced a **pre-existing** bug in the shared
`logos` lexer: it recurses roughly once per byte of a single token, so a
sufficiently long token (e.g. a ~200 KB string literal) overflows the stack and
aborts the process — in the *compiler*, not just the manifest parser. A
minimal `Token::lexer("\"" + "a"×200000 + "\"").next()` reproduces it.

For the manifest — which the grammar spec requires to terminate in bounded
steps and never hang (§10.3) — this is fixed defensively: the document is capped
at 256 KiB (Gate B) and the parse runs on a dedicated 64 MiB-stack thread, so no
manifest can overflow regardless of the caller's stack. The underlying lexer
recursion in general WFL source is flagged here for a separate follow-up.

## Scope Deliberately Deferred (per the design docs)

The registry wire protocol, Sigstore/OIDC/transparency-log/sumdb, the MVS
resolver rewrite, CAFS, and the `ast_hash` *normalizer* over the language AST
are all explicitly out of this v1 slice. The resolver/cache/registry keep their
existing calendar `Version` internally; the manifest *format*, validation,
tooling, and hashing are the delivered surface. The `ast_hash` lockfile *field*
is committed now (the regret-minimizing move) even though the normalizer that
fills it is a tracked follow-up.

## Deviations from the spec examples (recorded honestly)

- The spec's illustrative examples indent entries four spaces; the frozen
  Appendix A ABNF — the byte-stable canonical form that is signed and hashed —
  has no leading indentation on `entry-line`. `wfl fmt` follows the ABNF (no
  indent). Indentation remains accepted on *input*.
- The `port is 1 plus 1` corpus example is rejected at `MG-S05` (bare reserved
  keyword `port` as a key), not `MG-S01`; the arithmetic-value case is tested
  with a non-keyword key (`retries is 1 plus 1` → `MG-S01`). `hex`/`exponent`
  numeric forms decompose into an int + identifier and reject as `MG-S01`
  rather than `MG-L10` — still fail-closed.

## Tests

`cargo test -p wfl_core` (lexer + overflow regression), `cargo test -p wflpkg`
(249 tests: grammar gates, corpus, determinism, drift oracle, schema, CLI, alias
invariant, and the migrated integration suites), and
`cargo test -p wfl --test manifest_subset_equivalence` (proves
`L(manifest) ⊂ L(WFL)` and that both parsers agree on record counts) all pass.
The root crate's 418 unit tests are unaffected by the lexer change.
