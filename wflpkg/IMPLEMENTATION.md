# wflpkg Implementation Notes (frozen grammar `1.0.0`)

This document maps the three resolved design documents to the code that
implements them, and records the decisions and known issues from the build.

**Design sources** (the normative record; each is cited throughout the code as
`grammar §N`, `ADR-001 §N`, `Decision N`):

- `wflpkg-brainstorm-results.md` — the overall rebuild rationale.
- `wflpkg-open-decisions-resolved.md` — Decision 1 (manifest format) and
  Decision 2 (content-addressed identity).
- `wflpkg-manifest-grammar-1.0.md` — the frozen data-literal grammar `1.0.0`.
- `wflpkg-adr-001-binary-and-crate-structure.md` — binary/crate topology.

## Crate topology (ADR-001)

```
wfl_core   (crates/wfl_core)      the single shared logos lexer + version const
   ▲   ▲
   │   └── wflpkg (crates/wflpkg) library: datalit grammar, schema, commands
   │            ▲          ▲
   │            │          └── [[bin]] wflpkg   thin alias (no parsing logic)
   └── wfl (root)  re-exports wfl_core::lexer as crate::lexer; wfl fmt/manifest
```

There is **exactly one parser**. The lexer lives below both `wfl` and `wflpkg`
so neither can ship a divergent tokenizer. The root crate's `src/lexer.rs` is a
one-line re-export shim, so every pre-existing `crate::lexer::…` path is
unchanged.

## Where each spec section lives

| Spec | Code |
|------|------|
| Design law: subtractive, `(token, span)`, reject-don't-repair (§2) | `datalit/mod.rs`, `datalit/parser.rs` |
| Gate B — bytes (§6) | `datalit/bytegate.rs` |
| Gate L + Gate S — lexical/span + structure (§6) | `datalit/parser.rs` |
| Gate I — identity/version (§6, §8) | `datalit/identity.rs`, `datalit/version.rs` |
| Admitted node set / document model (§3, §4) | `datalit/mod.rs` (`Document`/`Record`/`Entry`/`Value`/`Scalar`) |
| Error codes `MG-*` (§6) | `datalit/error.rs` |
| Limits (§7) | `datalit/limits.rs` |
| Canonical `wfl fmt` (§7.1) | `datalit/fmt.rs` |
| JCS JSON projection (§7.2) | `datalit/json.rs` |
| `file_hash` / `content_hash` (§7.3) | `datalit/hash.rs` |
| Schema layer (§3) | `manifest/schema.rs`, `manifest/parser.rs`, `manifest/writer.rs`, `lockfile/*` |
| Conformance / fuzz corpus / drift oracle (§9, §10) | `datalit/tests.rs`, `tests/manifest_subset_equivalence.rs` (root) |
| CLI `wfl fmt` / `wfl manifest --json/--hash` | `commands/tooling.rs`, `src/main.rs`, `crates/wflpkg/src/main.rs` |

## The manifest & lockfile format

```wfl
create map wflpkg:
    grammar is "1.0.0"
end map

create map package:
    name is "greeting"
    version is "26.2.1"
    description is "A friendly greeter"
    authors is ["Alice", "Bob"]
    permissions is ["file-access"]
end map

create map dependency:
    name is "http-client"
    version is "26.1 or newer"
end map
```

- Every file begins with the `wflpkg` version envelope (grammar §9).
- Values: quoted string, whole number (`0..2^53-1`), `yes`/`no`, or a
  comma-separated `[…]` list of those. No null (omit the key), no float, no
  comments (use a hashed `notes` field), no arithmetic/references.
- A `dependency` with `scope is "dev"` is a development dependency; its
  `version` is a natural-language constraint. The `package` `version` is an
  exact `MAJOR.MINOR.PATCH`.
- Lockfile `locked` records carry `hash` (integrity) and the schema-committed
  optional `ast_hash` (Decision 2).

## CLI

```
wfl fmt [file] [--check]          # canonicalize (or verify) a manifest/lockfile
wfl manifest [file] --json        # deterministic JCS JSON projection
wfl manifest [file] --hash        # file_hash + content_hash
```
The `wflpkg` alias exposes the same verbs from the same library path.

## Decisions and deviations (recorded honestly)

1. **Canonical form has no entry indentation.** The Appendix-A ABNF —
   the byte-stable form that is signed and hashed — defines
   `entry-line = key SP is SP value LF` with no indent, so `wfl fmt` emits none.
   Indentation is still *accepted* on input. (Changing to indented output is a
   coordinated grammar + ABNF change.)
2. **Version semantics are split, deliberately and temporarily.** Gate I
   validates version strings with the new SemVer grammar
   (`datalit/version.rs`); the resolver/cache/registry keep their existing
   calendar `Version` internally. Both accept the normal `YY.MM.BUILD`-shaped
   strings identically; unifying resolution onto SemVer/MVS is the deferred
   resolver rebuild.
3. **`ast_hash` field committed, normalizer deferred.** The lockfile schema
   carries `ast_hash` from day one (regret-minimizing); the normalized-AST hash
   that fills it (Decision 2's bounded item) is a tracked follow-up.
4. **Corpus code-precision nuances.** `port is 1 plus 1` rejects at `MG-S05`
   (bare reserved keyword `port` as a key) — the arithmetic-value case is tested
   with a non-keyword key (`MG-S01`). `hex`/`exponent` numeric forms decompose
   to int + identifier and reject as `MG-S01` rather than `MG-L10`. All remain
   fail-closed.
5. **UTS #39 tripwire is vacuous under ASCII-only** (grammar §8). The ASCII
   allowlist makes identity security independent of the Unicode pin. The
   `unicode-security` crate + vendored confusables tables are a follow-up needed
   only if identity fields are ever relaxed off ASCII.

## Known issue found during the build

The shared `logos` lexer recurses ~once per byte of a single token, so a very
long token (e.g. a ~200 KB string literal) overflows the stack and aborts — a
**pre-existing** DoS affecting the compiler, not introduced here. For the
manifest, `datalit::parse` caps documents at 256 KiB (Gate B) and runs on a
64 MiB-stack worker thread, guaranteeing bounded termination (§10.3). The
general lexer recursion is flagged for a separate fix.

## Deferred (out of this v1 slice, per the design docs)

Registry wire protocol, Sigstore/OIDC/transparency-log/sumdb, the MVS resolver
rewrite, CAFS, and the language-AST `ast_hash` normalizer. See
`wflpkg-open-decisions-resolved.md` "Next steps" and ADR-001 "Follow-up
actions" for the tracked roadmap.
