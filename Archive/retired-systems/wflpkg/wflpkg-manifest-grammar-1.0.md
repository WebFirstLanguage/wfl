# wflpkg Manifest & Lockfile — Frozen Data-Literal Grammar

**Spec ID:** `wflpkg-data-literal-grammar`
**Spec version:** `1.0.0-draft` — SemVer, versioned **independently of the WFL language** (which uses `YY.MM.BUILD`).
**Normative pins:** `UNICODE_VERSION = 16.0.0` · `NORMALIZATION = NFC (UAX #15)` · `WIRE = UTF-8, LF, no-BOM` · `HASH = SHA-256`
**Status:** Condition-1 launch-gate artifact for **Decision 1** (canonical manifest format = WFL-native typed data literal). Produced 2026-07-08.

> This document is the artifact required by **Decision 1, condition 1** of `wflpkg-open-decisions-resolved.md`: *"a frozen, documented, data-literal-only grammar, versioned independently of the language, so language evolution never silently becomes a manifest-format (and therefore supply-chain) change."* It specifies the **exact admitted node set** and the **rejection rules**, plus the versioning governance, identity-field policy, and conformance apparatus that make the freeze real.

---

## 1. Purpose and scope

A wflpkg manifest (`project.wfl`) and lockfile (`project.lock`) are **supply-chain metadata**: signed, hashed, resolved by machines, and read by humans and AI agents. A manifest is a **value, never a program.** It is *deserialized*, never *executed*.

This grammar defines a small, restricted, **frozen** data-literal dialect of WFL. It shares WFL's lexer byte-for-byte (so there is one place where tokenization bugs can live), but admits **only** named key/value record blocks and a closed set of literal values. It forbids everything that would make a manifest computable: actions, control flow, arithmetic, string interpolation, variable references, imports, indexing, calls, and I/O.

This document is **in scope** for:

- the concrete on-disk syntax a manifest/lockfile may use;
- the abstract node set a conforming parser may construct;
- the canonical normal form and its two derived hashes;
- the rejection rules, as an exhaustive, deny-by-default taxonomy;
- the independent-versioning governance and the CI drift oracle;
- the identity-field character policy and Unicode-data pin;
- the conformance and fuzzing obligations.

This document is **out of scope** for (these belong to Decision 2 and the registry/signing layer, and are only cross-referenced here): the registry wire format, signing/attestation/transparency-log design, the `ast_hash` content-address normalizer over the *language* AST, MVS resolution, and the choice of signing primitives beyond the SHA-256 baseline this grammar fixes for its own hashes.

---

## 2. Design law — the three-layer architecture

Everything below follows from four rules the review panel established and that the reader must keep in mind, because a casual reading of "admitted node set" will otherwise reintroduce the exact bugs this grammar exists to prevent.

### 2.1 Subtractive over the shared lexer

WFL has **one** lexer (the `logos` tokenizer in `src/lexer/token.rs`). The compiler and every first-party tool tokenize with that same lexer — that is the real substance of condition 5. The frozen grammar is an **acceptance predicate layered on top of the shared token stream.** It may only ever **reject** token sequences the lexer produces; it may **never require the lexer to emit a token the lexer does not already emit.** A subtractive layer can never demand a lexer change, which is precisely what lets the manifest grammar be versioned independently (condition 1) while sharing one lexer (condition 5) without the two version streams ever deadlocking.

Consequence, made explicit: this grammar admits **strictly less** than the WFL language. For every byte sequence it accepts, the full WFL parser also accepts it and produces an equivalent parse tree. `L(manifest) ⊂ L(WFL)`.

### 2.2 The acceptance layer reads `(token, source-span)`, not just tokens

This is the subtle, load-bearing point. Three security-relevant distinctions are **erased by the lexer before the token stream exists**, so a rejection layer that is a pure function of `Vec<Token>` is blind to them:

| Distinction | How the lexer erases it | Verified in source |
|---|---|---|
| boolean **case/spelling** | `#[regex("(?i:yes\|no\|true\|false)")] BooleanLiteral(bool)` — collapses `yes`,`YES`,`true`,`TRUE`,… to one `bool` | `token.rs:443` |
| null **spelling** | `#[token("nothing")] #[token("missing")] #[token("undefined")] NothingLiteral` — three spellings, one token | `token.rs:452–454` |
| integer **leading zeros** | `#[regex("[0-9]+", parse::<i64>)] IntLiteral(i64)` — `007` and `7` both become `7` | `token.rs:463` |

Therefore the frozen acceptance layer is defined over the **`(token, span)` stream** and its rejections **read the raw span bytes**. It remains subtractive (it emits no new token), but it can see the surface spelling the token payload discarded. Any implementation that inspects only token payloads will silently re-admit `TRUE`, `missing`, and `007`.

### 2.3 The artifact specifies three layers and two mappings

The security property does not live in any single layer — it lives in the **mappings between them being total and unambiguous.**

- **Layer 1 — Concrete syntax** (over the `(token, span)` stream): which token sequences, with which span-level constraints, are admitted.
- **Layer 2 — Admitted node set** (abstract syntax): which data nodes a conforming parser may construct. *Necessary but not sufficient* — reading only this layer loses every rejection in Layer 1 and every determinism guarantee in Layer 3.
- **Layer 3 — Normal form**: the single canonical byte form (for `wfl fmt` / `file_hash`) and the single canonical JSON projection (for interop / `content_hash`).
- **Mapping A — concrete → node** must be **total and unambiguous**: every accepted input maps to exactly one node tree. *This is where the collapses die* (boolean case, null spelling, leading zeros, raw newlines, duplicate keys, extra separators). Omit this mapping from the spec and the collapses return.
- **Mapping B — node → normal form** must be **deterministic and total**: the same node tree yields byte-identical output. *This is where content identity is fixed* and where the `yes`/`no` → `true`/`false` projection happens.

### 2.4 Reject, don't repair

On any ambiguity, duplicate key, non-canonical spelling, disallowed byte, overlong encoding, or limit breach, the parser returns a **typed error with a byte offset** — never a silently repaired value, never a panic, never a hang. Silent repair *is itself* a differential: a repairing consumer and a non-repairing consumer disagree about the same bytes. `wfl fmt` is the **only** sanctioned transformer, and it operates on author intent *before* signing — never on the verification path.

---

## 3. The document model (v1 — flat)

> **Owner decision (2026-07-08):** v1 manifests are **flat**. There is no inline record-in-record nesting. WFL has no inline record/map literal today; rather than add one to the language now, v1 stays a strict subset of *today's* WFL and expresses structured or repeated data as a **sequence of named top-level record blocks**. An inline nestable record literal is a deferred, coordinated language change (see §11).

A conforming manifest or lockfile is a **sequence of one or more top-level record blocks.** A record block is WFL's existing `create map <name>: … end map` statement, with its value grammar **narrowed to the literal set of §4** and all rejection rules of §6 applied.

Because the file is deserialized and never executed (condition 2), the block **name is read as the record's *kind* tag**, not as an executable binding, and **repetition of a kind is permitted** (three `requires` blocks are three dependency records, not a variable shadowed twice). The relationships between records are defined by the **schema layer** (which kinds exist, which are singular, how they relate) — never by in-document references, which are forbidden.

Maximum structural depth is **2 by construction**: a record contains entries; an entry's value may be a scalar or a **list of scalars**; lists do not contain lists or records. There is no deeper nesting to bound.

**Reserved-keyword rule (a real constraint, verified against the lexer).** A record-kind NAME must be a **non-reserved** WFL identifier. WFL has ~180 reserved keywords, and one lexes as its keyword token rather than as an `Identifier` — so `create map requires:` or `create map port:` does **not** parse (`requires`, `port` are keywords), and such input is rejected at `MG-S05` (the parser sees a keyword where an identifier is required). The canonical record kinds are chosen to avoid this: `package`, `dependency`, `locked`, and the version envelope `wflpkg` are all non-reserved. An **entry key** that must collide with a keyword can be written as a **quoted string** (`"requires" is …`), because keys admit strings; a *bare* keyword in key position is rejected the same way. Schema authors and the `wfl add` writer must respect this; `wfl fmt` never emits a bare reserved keyword as a name or key.

### 3.1 Illustrative manifest (`project.wfl`)

```wfl
create map package:
    name is "greeting"
    version is "26.2.1"
    description is "A friendly command-line greeter"
    authors is ["Alice Smith", "Bob Jones"]
    license is "MIT"
    keywords is ["cli", "greeting"]
    notes is "Human-readable annotations live here, inside the hash."
end map

create map dependency:
    name is "http-client"
    version is "26.1 or newer"
end map

create map dependency:
    name is "json-parser"
    version is "25.12 or newer"
    scope is "dev"
end map
```

### 3.2 Illustrative lockfile (`project.lock`)

```wfl
create map locked:
    name is "http-client"
    version is "26.1.3"
    hash is "sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
    deps is ["text-utils"]
end map

create map locked:
    name is "text-utils"
    version is "25.11.2"
    hash is "sha256:2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae"
    deps is []
end map
```

Both files are valid WFL programs consisting solely of `create map` statements whose values are literals — so the shared lexer and the full WFL parser accept them and produce the identical parse tree. The manifest tool simply **deserializes** that tree instead of executing it, and rejects any block whose value is not a literal.

---

## 4. Admitted node set (Layer 2 — deny-by-default)

Everything not in this table is rejected. The "WFL source" column is the verified mapping to the real AST/lexer; the manifest admits a strict subset.

| # | Node | Admitted concrete form | Constraints (normative) | WFL source |
|---|---|---|---|---|
| **N0** | **Document** | ≥1 Record blocks, newline-separated | only records and newlines at top level; ≤ limits (§7) | statement sequence |
| **N1** | **Record** | `create map` NAME `:` NL Entry* `end map` | NAME is a **non-reserved** lowercase-ASCII identifier = the record *kind*; block-delimited; **not** indentation-significant; repetition of a kind allowed | `Statement::MapCreation` (`ast.rs:406`, `collections.rs:205`) |
| **N2** | **Entry** | KEY `is` VALUE NL | exactly one `is`; duplicate KEY within one Record ⇒ reject | `(key, value)` pair (`collections.rs:257`) |
| **N3** | **Key** | lowercase-ASCII identifier **or** quoted string | identifier `[a-z][a-z0-9_]*`; string key obeys N4-String; keys are unique per record | `Identifier` / `StringLiteral` (`collections.rs:247,261`) |
| **N4** | **Value** | exactly one of {String, Integer, Boolean, List} | **no** null, **no** float, **no** nested record, **no** references/operators/calls/patterns | narrowed from `parse_expression` |
| **N5** | **String** | `"` … `"` | NFC; escapes `\n \t \\ \"` only; ≤ 64 KiB decoded; see §6 reject-list | `Literal::String` (`ast.rs:788`) |
| **N6** | **Integer** | `[0-9]+` | unsigned; no leading zero (except `0`); `0 ≤ n ≤ 2^53−1` | `Literal::Integer` (`ast.rs:789`) |
| **N7** | **Boolean** | `yes` \| `no` | **lowercase only**; canonical surface = `yes`/`no`; JSON projection = `true`/`false` | `Literal::Boolean` (`ast.rs:790`) |
| **N8** | **List** | `[` (Scalar (`,` Scalar)*)? `]` or `[]` | **comma separator only**; scalars only (String/Integer/Boolean); no trailing comma; ≤ 4 096 elements | `Literal::List` (`ast.rs:793`) |

**Identity-bearing fields** (subject to §8 policy): the string values of the keys `scope`, `name`, and `version`, wherever a record carries them.

**Explicitly excluded** (each would otherwise ride in through the shared grammar): the **Null** literal `nothing`/`missing`/`undefined` (N-none — absence is expressed by *omitting the key*); `Literal::Float` (§6, MG-L10); `Literal::Pattern`; any variable/identifier used as a *value*; any binary/unary operator; string interpolation; any statement other than a literal-valued top-level `create map`; and any nested record value (v1 flat model, §3).

---

## 5. Value forms in detail

**Strings (N5).** Double-quoted. The admitted escape set is exactly `\n`, `\t`, `\\`, `\"`. WFL's lexer also accepts `\r` and `\0`; the manifest **drops both** (`\r` reintroduces CR/LF ambiguity; `\0` is a truncation/parser-confusion vector). No `\u` escape exists in WFL and the manifest does **not** add one (see §11). Non-ASCII in free-text fields (`description`, `authors`, `notes`) is admitted as literal **NFC** UTF-8, minus the always-rejected invisibles/controls/bidi of §6. Identity fields get no such latitude (§8).

**Integers (N6).** Unsigned decimal, no leading zeros except the single digit `0`, range `0 … 2^53−1` (I-JSON safe-integer range — lossless for any JSON consumer). A leading `-` is a separate `Minus` token in WFL, i.e. an *expression*; negative numbers are therefore not expressible and are rejected. **Floats are forbidden entirely.** This deletes IEEE-754 nondeterminism, the RFC 8785 shortest-round-trip / Ryū machinery, and the `1` vs `1.0` canonical-collision (both project to `1`). Every legitimate manifest number — ports, counts, timeouts, limits — is a non-negative integer.

> **Implementation note (normative bug-fix).** WFL's `IntLiteral` callback is `parse::<i64>().unwrap()`, which **panics** on an integer above `i64::MAX`. A conforming manifest ingest MUST guard the byte-length/range of an integer literal *before* it can panic, and return `MG-L11` (§6) instead. "Reject, don't panic" is part of reject-don't-repair.

**Booleans (N7).** Canonical on-disk surface is **`yes` / `no`, lowercase only.** *(Owner decision, 2026-07-08: `yes`/`no` chosen over `true`/`false` for WFL-native aesthetics and No-Unlearning consistency — WFL's booleans literally are `yes`/`no`.)* Because the lexer is case-insensitive over four spellings, "one canonical spelling" is enforced as a **span-level rejection**: any bytes other than exactly `yes` or `no` (including `true`, `false`, `Yes`, `NO`, `YeS`) are rejected (`MG-L07`). Mapping B projects `yes → true`, `no → false` in the JSON normal form, so external consumers and the `content_hash` see standard JSON booleans regardless of the surface choice. The Norway problem is structurally impossible here: identity/string fields are mandatorily quoted, so a bareword can only ever occupy a boolean-typed position against the closed set `{yes, no}`.

**Null.** There is no null value node. Absence of a value is expressed **only** by omitting the key. This deletes the absent-key-vs-`nothing` equivalence-class ambiguity at the root. Optionality is a schema concern, not a grammar one.

**Lists (N8).** Bracketed, **comma-separated only.** WFL's list literal accepts three interchangeable element separators — `,`, `and`, and `:` (verified in `src/parser/expr/primary.rs:53–55`) — which is a latent multi-byte-form differential; the manifest admits the comma only and rejects `and`/`:` as separators (`MG-S03`). No trailing comma. Elements are scalars (String/Integer/Boolean); lists do not nest.

**Versions and version constraints.** Always **quoted strings** (a bare `26.2.1` lexes as `Float(26.2) · Dot · Integer(1)` — mis-tokenized). A `version` field in a `locked` record is an exact version validated against the version grammar `MAJOR.MINOR.PATCH` (each `[0-9]+`, no leading zeros except `0`, `MAJOR < 256` per WFL's MSI rule, optional `-prerelease` / `+build` of `[0-9A-Za-z-]`). A `version` field in a `requires` record is a constraint string validated against the constraint grammar (`"26.1 or newer"`, `"26.1.3 exactly"`, `"between 25.12 and 26.2"`, `"any version"`, `"26.1 or newer but below 27"`, `"above 25.6"`, `"below 27"`). Both are ASCII-only.

---

## 6. Rejection rules (Mapping A — exhaustive, typed, fail-closed)

Ingest is three ordered gates. A byte fails at the first gate that catches it. Every code below is a **stable part of this versioned spec** (error codes are an API): identical input MUST yield the identical code in every conforming implementation.

### Gate B — byte-level, before the shared lexer

| Code | Reject when… | Adversary / cite |
|---|---|---|
| **MG-B01** | bytes are not well-formed UTF-8 | RFC 3629 |
| **MG-B02** | overlong UTF-8 encoding (`C0 80`, `E0 80 80`, `F0 80 80 80`, …) | Unicode D92; CVE-2008-2938 lineage |
| **MG-B03** | surrogate-range or > U+10FFFF byte sequence | I-JSON RFC 7493 |
| **MG-B04** | Byte-Order Mark U+FEFF anywhere (including offset 0) | Trojan-Source / invisible byte |
| **MG-B05** | content is not in NFC (UAX #15) — **reject, do not normalize** | condition 3; canonicalization differential |
| **MG-B06** | document size exceeds the §7 limit | resource exhaustion |
| **MG-B07** | line ending other than LF used as structure (CR, CRLF) | canonicalization differential |

> **NFC ruling (condition 3).** We **reject** non-NFC input; we never silently normalize. This dissolves the "hash raw bytes / normalize / preserve — pick two" trilemma: by constraining the accepted domain to already-NFC bytes, *preserve ≡ normalize* on the accepted set, so we keep byte-stable hashing **and** an NFC guarantee, and simply reject anything where they would differ. Every consumer runs the identical boolean `NFC(bytes) == bytes` — there is no transform to diverge on.

### Gate L — lexical / span (shared lexer runs; then a span post-pass)

| Code | Reject when… | Notes |
|---|---|---|
| **MG-L01** | any comment is present (`//` or `#`) | detected via **gap coverage**: every inter-token gap must match `^[ \t]*$`; any skipped comment region ⇒ reject. A `//` or `#` *inside a string* is inside the StringLiteral span, not a gap, so URLs and `#tags` in string values are unaffected. |
| **MG-L02** | inter-token whitespace contains form-feed `\f` or any C0 other than the LF entry separator | WFL's lexer silently skips `\f` (`token.rs:4`) |
| **MG-L03** | string contains a raw C0 control (U+0000–001F), DEL (U+007F), or C1 (U+0080–009F) | WFL's string regex admits these (`token.rs:457`) |
| **MG-L04** | string contains a raw newline (CR or LF) | manifest strings are single-line; use `\n` |
| **MG-L05** | string contains a bidi control (U+202A–202E, U+2066–2069, U+200E/200F) | Trojan Source, CVE-2021-42574 |
| **MG-L06** | string contains a zero-width / invisible char (U+200B–200D, U+2060, U+FEFF) | invisible confusable |
| **MG-L07** | boolean bytes are not exactly `yes` or `no` (rejects `true`,`false`,`YES`,`Yes`,…) | span-level; the lexer erased the spelling |
| **MG-L08** | escape other than `\n \t \\ \"` (rejects `\r`, `\0`, `\uXXXX`, unknown escapes, trailing `\`) | §5 |
| **MG-L09** | null literal `nothing` / `missing` / `undefined` in value position | absence = omit the key |
| **MG-L10** | float literal `[0-9]+\.[0-9]+`, or any signed/hex/exponent numeric form | §5 |
| **MG-L11** | integer has a leading zero (length > 1), or exceeds 2^53−1 / would overflow i64 | span-level; prevents the lexer's `unwrap()` panic |
| **MG-L12** | string decoded length > 64 KiB, or key length > 256 bytes | resource exhaustion |

### Gate S — structural (restricted manifest parser over the token stream)

| Code | Reject when… | Notes |
|---|---|---|
| **MG-S01** | any value node outside {String, Integer, Boolean, List} — a reference, operator, call, index, interpolation, pattern, or nested record | condition 2; the `create map` value production is normally `parse_expression()` (`collections.rs:281`) — this narrows it |
| **MG-S02** | duplicate key within one Record | I-JSON RFC 7493 / TOML 1.0 — hard error, never last-wins |
| **MG-S03** | list separator is `and` or `:`, or a trailing comma, or a list element that is itself a list or record | the three-separator differential; scalars-only |
| **MG-S04** | a top-level node that is not a `create map … end map` record block | ordering/determinism |
| **MG-S05** | unterminated or malformed block: missing `:`, missing `is`, missing `end map`, `end` not followed by `map`, or a **reserved WFL keyword used as a bare record NAME or bare entry KEY** (it lexes as a keyword, not an identifier) | reject, don't guess; keyword-colliding keys must be quoted strings |
| **MG-S06** | a `create map` value is anything the full expression grammar would accept but this grammar does not | belt-and-suspenders subset assertion |
| **MG-S07** | any §7 limit exceeded (depth, entries, list length, node count) | resource exhaustion |

### Gate I — identity / semantic (post-parse)

| Code | Reject when… | Notes |
|---|---|---|
| **MG-I01** | a `scope` / `name` field violates the identity allowlist `[a-z][a-z0-9]*(-[a-z0-9]+)*` (≤ 64) | §8 |
| **MG-I02** | a `version` field fails the exact-version or constraint grammar | §5 |
| **MG-I03** | an identity field fails the UTS #39 tripwire (mixed-script / restriction-level) under the pinned Unicode data | §8 — vacuous under ASCII-only, but wired so a future relaxation cannot skip it |

---

## 7. Canonical form, `wfl fmt`, and the two hashes

### 7.1 Canonical on-disk form

`wfl fmt` is shipped **day one** (condition 6) and is the only writer. The canonical form is: UTF-8, NFC, `LF` line endings, no BOM, no comments; one record block per logical record separated by a single blank line; exactly one space around `is` and after `:`; one entry per line; list elements separated by `, ` (comma + single space); keys emitted in schema-canonical order. Because every non-canonical form is *rejected* (not collapsed), the concrete→node mapping is **injective over accepted inputs**: byte-identity ≡ node-identity ≡ semantic-identity. `fmt` is idempotent (`fmt(fmt(x)) == fmt(x)`) and round-trips (`parse(fmt(x)) == parse(x)`).

### 7.2 The JSON projection (Mapping B → JCS)

The deterministic interop projection is the parsed value rendered as **I-JSON** (RFC 7493) and canonicalized per **RFC 8785 (JCS)**: records → objects, lists → arrays, `yes`/`no` → `true`/`false`, strings per JCS escaping, integers as minimal decimal, object keys sorted by UTF-16 code-unit order. Two properties make this footgun-free *because of decisions already locked*: floats are banned, so the `1`/`1.0` collision and the Ryū/V8 number path never arise; and **all keys are ASCII** (identifiers or ASCII string keys), so JCS's UTF-16-code-unit key-sort — which disagrees with Rust/Python/Go's default code-point sort only for non-BMP keys — cannot fire. `wfl manifest --json` emits this projection; it is a lossless derived export, and the WFL literal remains the source of truth.

### 7.3 Two hashes, two questions

These are **different layers, not competitors.** Both use **SHA-256**.

| Hash | Definition | Answers |
|---|---|---|
| **`file_hash`** | `SHA-256` over the canonical on-disk bytes (`fmt` output) | "did these exact bytes arrive intact / is this what was signed on disk?" — transport, signing, reproducible download |
| **`content_hash`** | `SHA-256` over the JCS projection of the parsed value | "are these two manifests the same package spec regardless of formatting?" — dedup, cache keys, dependency identity |

They can never disagree about *content* because both are projections of the same node set and `fmt` is deterministic. **Signatures cover `file_hash` bytes**; verification does **no** canonicalization step (canonicalize-in-the-verify-path is a documented signature-bypass class). A separate `ast_hash` is *not* introduced by this artifact: over accepted inputs `file_hash` already uniquely identifies the byte form, and Decision 2's normalized-AST `ast_hash` is a distinct concern defined against the language AST.

> **Algorithm ruling.** SHA-256 is the cross-ecosystem interop hash (crates.io, OCI, Git's SHA-256 transition, TUF, Sigstore/Rekor, SPDX/CycloneDX SBOMs). Digests are stored **algorithm-tagged** (`sha256:…`) so the ecosystem is rotation-ready, but SHA-256 is the sole mandatory-to-implement algorithm in v1. **WFLHASH is explicitly excluded from the integrity/identity path** — an unaudited custom primitive on a cross-trust-boundary channel has zero external interop and no payoff here. (Final signing/transport policy is Decision 2's; this artifact fixes only the hashes over its own canonical forms.)

---

## 8. Identity-field policy (condition 4)

**Codepoint allowlist (the hard boundary, in the frozen grammar).**

- **scope, name:** `[a-z][a-z0-9]*(-[a-z0-9]+)*` — lowercase ASCII, single internal hyphens, no leading/trailing/double hyphen, **underscore forbidden** (kills the `-`/`_` collision class), length ≤ 64.
- **version:** ASCII `MAJOR.MINOR.PATCH` with optional `-prerelease` / `+build`; no Unicode at all.

**Restriction level: ASCII-Only.** This is the strongest UTS #39 level and it *subsumes* Single-Script and Moderately-Restrictive. It matches the de-facto standard of every major registry — npm, crates.io, and PyPI are all ASCII-only and stricter (with normalization/collapsing); none permits Unicode package names. ASCII confusables are Unicode-version-stable, so the identity path's security **does not depend on the Unicode pin at all** — a decisive de-risking of the crate-currency problem below.

**Where confusable checking runs.**

- *This grammar (per-file):* enforces the ASCII allowlist at ingest (`MG-I01`). The UTS #39 mixed-script / restriction-level check runs as a **tripwire** (`MG-I03`) that is vacuous under ASCII-only but is already wired, so any future relaxation off ASCII cannot silently skip it.
- *Registry (whole-namespace):* the UTS #39 skeleton/confusables collision check ("is this new name confusable with an existing one?") is inherently namespace-scoped and is the registry's job, not the grammar's. Both layers use the **same pinned** Unicode data.

**Unicode data pin (normative).** Because the released crates cannot agree on a Unicode version today — `unicode-security 0.1.2` embeds Unicode **16.0.0** while `unicode-normalization 0.1.25` embeds **17.0.0**, and the 16→17 confusables delta is material (~249 mapping-line changes: 39 skeleton flips + 210 new sources) — trust anchors are **pinned, not floated on crate cadence**:

1. **Vendor** `confusables.txt`, `IdentifierStatus.txt`, and Identifier_Type data at **Unicode 16.0.0** (matches rustc's `unicode-security 0.1.2` lints, so WFL agrees with the Rust toolchain, and is the newest *published* confusable table set). Hash the vendored files and freeze them in-repo; identity/confusable decisions read the **vendored** tables, never a crate's bundled tables.
2. **One crate suffices:** `unicode-security 0.1.2` exposes §5.1 mixed-script (`MixedScript`), §5.2 restriction levels (`RestrictionLevel` / `detect_restriction_level`), and `Identifier_Type`; whole-script confusable is derivable. Dependency surface = one crate + vendored data.
3. **CI, fail-closed:** assert the vendored-file hashes match the frozen values; assert each crate's `UNICODE_VERSION` const equals its expected value (documenting the NFD-17 / confusables-16 split under an explicit waiver); and — because `skeleton()` is defined over NFD and is normalization-independent (NFC-first-then-skeleton yields the identical result for all scripts) — carry a **property test** asserting skeleton stability across that split over the accepted-input alphabet, so the fact is a continuously-verified in-repo assertion rather than a citation.
4. A Unicode-data upgrade is a **grammar MAJOR-version bump**, reviewed for confusable-set deltas and applied to all consumers atomically through the drift oracle (§9).

---

## 9. Versioning and governance (condition 1 vs condition 5)

The apparent conflict — "version the grammar independently of the language" (1) vs "share the lexer byte-identically" (5) — is resolved by separating the **lexer** from the **acceptance grammar** and gating drift in CI.

- **Shared, byte-identical (condition 5):** the token-production function `bytes → tokens` (the WFL `logos` lexer), consumed identically by the compiler and every first-party tool. There is literally one implementation, not two that must agree. The single restricted **manifest parser** over that token stream is likewise one shared implementation.
- **Independently versioned (condition 1):** the **acceptance predicate over `(token, span)`** — this grammar plus every span-level and structural rejection. It carries its own SemVer (`wflpkg-data-literal-grammar 1.0.0`), evolves on its own cadence (raise a limit, add a field type), and requires **no language release**.
- **The seam is a fail-closed CI drift oracle.** The grammar pins, as normative data, the exact set of admitted token kinds and the byte-surface of each. A conformance test re-derives each admitted token's byte-surface from the *current* lexer and asserts it is byte-identical to the pinned snapshot, and asserts the manifest parser rejects every non-allowlisted token/node. A companion test asserts every *accepted* corpus input also parses under the current full WFL parser and yields an equivalent tree (the subset guard). **Any drift — a lexer change touching an admitted token, a new escape, a changed literal regex — turns the build red and forces a conscious grammar-version decision** (adjust the language change, or cut `1.1` / `2.0` and migrate). Language evolution may freely *add* tokens/nodes (the manifest simply rejects them); it can never *silently change* the manifest, because the manifest's surface is an explicit allowlist, not a live pointer at the parser.

A lockfile stamped `1.0.0` is read by the `1.0.0` grammar forever, regardless of where the language has moved. That is what independent versioning buys.

Each file **declares its grammar version** as a required first record field: `create map wflpkg: grammar is "1.0.0" end map` (the version envelope is itself part of the frozen skeleton; a reader selects the grammar version explicitly).

---

## 10. Conformance and fuzzing (conditions 5 & 6)

The formal grammar in Appendix A is **executable** and serves as the oracle. A continuous, coverage-guided **differential fuzz harness** (libFuzzer/AFL over the ABNF-derived generator, structured inputs via `arbitrary`) MUST assert:

1. **Subset + equivalence** — every input the manifest *accepts* is also accepted by the shared WFL lexer+parser and yields a structurally equivalent data tree (same keys, same scalar values, same list order).
2. **Byte-identical shared parse** — the compiler-embedded manifest parser and the standalone `wfl` tool return byte-identical trees and identical accept/reject decisions **and identical error codes** on every input.
3. **Typed, bounded rejection** — every rejected input returns a §6 code; never a panic (including the integer-overflow path), never a hang; termination in bounded steps.
4. **Idempotence & round-trip** — `fmt(fmt(x)) == fmt(x)`; `parse(fmt(x)) == parse(x)` for accepted `x`.
5. **NFC closure** — accepted ⟹ `NFC(bytes) == bytes`; non-NFC ⟹ `MG-B05`.
6. **Collapse-freedom** — no two distinct accepted byte-strings share a node tree (proves Mapping A killed the boolean-case, null-spelling, leading-zero, raw-newline, duplicate-key, and separator collapses).
7. **Purity** — parse is a pure function of input bytes (no locale/env/time/thread dependence).
8. **Drift lock green** — pinned token/node surface and vendored `UNICODE_VERSION` unchanged, else fail closed.

**Malicious-input reject corpus** (seed; each MUST reject with the stated code; versioned with the grammar; adding a bypass is the first step of any incident response): BOM-prefixed (`MG-B04`); overlong `C0 80` / `E0 80 80` / `F0 80 80 80` (`MG-B02`); lone surrogate `ED A0 80` (`MG-B03`); non-NFC decomposed `é` (`MG-B05`); bare CR / CRLF (`MG-B07`); `//` and `#` comments, including a bidi-in-comment payload (`MG-L01`); form-feed between tokens (`MG-L02`); raw NUL / C0 / DEL in a string (`MG-L03`); raw newline in a string (`MG-L04`); RLO U+202E and other bidi in a string — Trojan Source (`MG-L05`); zero-width U+200B/200C/200D/FEFF (`MG-L06`); `true` / `TRUE` / `Yes` booleans (`MG-L07`); `\r` / `\0` / `A` escapes (`MG-L08`); `nothing` / `missing` / `undefined` value (`MG-L09`); float / signed / hex / exponent (`MG-L10`); `007` and a 20-digit integer (`MG-L11`); over-length string/key (`MG-L12`); reference-as-value `name is greeting` (`MG-S01`); arithmetic `port is 1 plus 1` (`MG-S01`); duplicate key (`MG-S02`); list with `and` / `:` separator or trailing comma or nested list (`MG-S03`); a top-level non-record statement (`MG-S04`); missing `end map` (`MG-S05`); Cyrillic-homoglyph name — CVE-2021-42694 (`MG-I01`); underscore in name (`MG-I01`); bare version `26.2.1` (`MG-L10`/`MG-I02`); and the **now-rejected prose forms** `version is 26.2.1` / `requires http-client 26.1 or newer` (`MG-S01`/`MG-I02`) so regression of the old dialect is caught.

---

## 11. Deferred / roadmap (explicitly out of v1)

These are recorded so the freeze is honest about what it is *not* doing:

- **Inline nestable record literal.** A future `map … end map` expression-position literal, restricted to literal leaves, adopted **identically** by the WFL language and this grammar, would close a No-Unlearning gap the language has today and allow true nesting (e.g. a `dependencies:` block). It is a **coordinated language change** gated by the §9 drift oracle — not a manifest-only dialect. Deferred by owner decision (2026-07-08); v1 is flat.
- **`\u{…}` string escape.** Adding a braced-scalar escape is *additive* to the shared lexer and so is forbidden under the subtractive invariant until it lands in WFL core through normal language evolution — at which point escape-decoded output must be re-run through the same C0/C1/bidi/zero-width rejection so an escape can never smuggle what a raw byte cannot. Forbidden in v1; also closes a homoglyph-smuggling channel.
- **Full content-addressed identity** (CAFS, hash-as-identity, fetch-by-hash) — Decision 2's deferred Phase 2, gated on its own three conditions.
- **The `ast_hash` normalizer** over the language AST — Decision 2's bounded net-new build item; distinct from this grammar's `file_hash`/`content_hash`.

---

## 12. Provenance and the forks that shaped this artifact

Produced 2026-07-08 via a structured adversarial review — four expert reviewers (Neil Hargrove/Pragmatist, Dr. Sylvia Okafor/Compliance Architect, Marcus Venn/Frontier Engineer, Priya Ashworth/Systems Disruptor), grounded by Zara Lenn (Researcher) with web + source verification — over a generation round, a peer-review round, a vote (2 submit / 2 contest, both contests resolving F3 identically), and two owner reconciliation calls. It continues `wflpkg-brainstorm-results.md` and `wflpkg-open-decisions-resolved.md`.

**Source-verified findings that shaped the rules** (all against the mounted WFL tree): WFL's list literal accepts **three** interchangeable separators `,`/`and`/`:` (`primary.rs:53–55`) → comma-only; `create map` values run the **full** expression parser (`collections.rs:281`) → narrowed to literals; the lexer skips `//`, `#`, and `\f` and accepts `\0` and raw control chars / raw newlines in strings (`token.rs:4,457`) → explicit rejects; booleans are case-insensitive over four spellings and null has three spellings, both lexer-collapsed (`token.rs:443,452`) → span-level single-spelling rejects; `IntLiteral` uses `parse::<i64>().unwrap()` (`token.rs:463`) → overflow guard; duplicate map keys are silently kept in a `Vec` (`collections.rs:257`) → hard reject.

**How the five live forks resolved:**

| Fork | Resolution | Basis |
|---|---|---|
| **F1 — records / nesting** | **Flat v1**: sequence of named `create map … end map` records, literal-only values, no inline nesting (depth ≤ 2); inline record literal deferred to a coordinated language change | **Owner decision**; panel had converged 4/4 on the inline literal as the alternative |
| **F2 — boolean spelling** | Surface **`yes` / `no`** (lowercase; reject all else); JSON projection `true`/`false` | **Owner decision** (aesthetic-wins / No-Unlearning); panel majority had preferred `true`/`false` |
| **F3 — comments** | **Forbid** `//` and `#`; human annotation via a first-class **hashed** `notes` field | Unanimous after review (Sylvia & Priya contested to force this; Neil & Marcus moved to it) — unhashed comments are a review-differential / agent prompt-injection channel |
| **F4 — `\u` escape** | **Forbid** in v1 | Subtractive invariant (adding it is a lexer change) + homoglyph-smuggling; Marcus moved |
| **F5 — hash target** | **Two SHA-256 hashes** at named layers: `file_hash` (on-disk bytes) + `content_hash` (JCS projection); WFLHASH excluded | Reconciled as different layers, not a fork; ASCII keys + float-ban make JCS clean |

**Honest residual dissent (not flattened):** Sylvia and (originally) the panel majority would have shipped the **inline record literal** now to close the No-Unlearning gap immediately; the owner chose the flat, strict-subset path for v1 and deferred the literal. Priya alone argued the boolean surface should track whatever WFL-the-language does; the owner fixed `yes`/`no`, which is consistent with WFL's current lexer. Neither dissent affects the frozen node set or rejection rules — only what a *future* v2 might add.

---

## Appendix A — Formal grammar (ABNF, RFC 5234; executable for differential fuzzing)

Character-level ABNF for the **canonical `wfl fmt` output form** — the byte-stable form that is signed, hashed, and fuzz-round-tripped. The restricted parser accepts a slightly larger pre-`fmt` set (extra horizontal whitespace, blank lines between entries) that `fmt` collapses to this. Terminals are bytes; the *authoritative tokenization* remains the one shared `logos` lexer, and the §9 pinned token allowlist is the contract between this ABNF and the live lexer. If this ABNF and the lexer's admitted byte-surface ever disagree, that is a build-breaking event by design.

```abnf
; ===== wflpkg-data-literal-grammar 1.0.0 ; UNICODE=16.0.0 ; wire=UTF-8/LF/no-BOM =====

manifest      = version-record 1*( blank record )         ; N0 ; no BOM (MG-B04)
version-record= %s"create" SP %s"map" SP %s"wflpkg" ":" LF
                   entry-line                              ; grammar is "x.y.z"
                %s"end" SP %s"map" LF
record        = %s"create" SP %s"map" SP name ":" LF
                   1*entry-line
                %s"end" SP %s"map" LF                      ; N1 ; kind = name; repeats allowed
name          = lcalpha *( lcalpha / DIGIT / "_" )         ; record-kind tag
entry-line    = key SP %s"is" SP value LF                  ; N2 ; dup key in record => MG-S02
key           = ident-key / string                         ; N3
ident-key     = lcalpha *( lcalpha / DIGIT / "_" )
value         = string / integer / boolean / list          ; N4  (else MG-S01/-L09/-L10)

string        = DQUOTE *schar DQUOTE                        ; N5 ; NFC ; <=64 KiB decoded
schar         = unescaped / escape
unescaped     = %x20-21 / %x23-5B / %x5D-10FFFF             ; excl " (22) and \ (5C);
                                                           ; C0/C1/DEL/BOM/bidi/zero-width => MG-L03/05/06
escape        = "\" ( "n" / "t" / %x5C / DQUOTE )           ; \n \t \\ \"  ONLY (else MG-L08)

integer       = "0" / ( %x31-39 *DIGIT )                    ; N6 ; unsigned; no lead zero; <=2^53-1 (MG-L11)
boolean       = %s"yes" / %s"no"                            ; N7 ; lowercase only (else MG-L07)
list          = "[" [ scalar *( "," SP scalar ) ] "]"       ; N8 ; comma only; no trailing (MG-S03)
scalar        = string / integer / boolean                  ; lists hold scalars only

; ----- identity sub-grammars (§8) ; ASCII-Only restriction level -----
scope         = lcalpha *( lcalpha / DIGIT / hyphenseg )    ; <=64 ; no "_" (MG-I01)
pkgname       = scope
hyphenseg     = "-" 1*( lcalpha / DIGIT )                   ; single internal hyphens
version-exact = num "." num "." num [ "-" alnumdot ] [ "+" alnumdot ]   ; MAJOR<256 (MG-I02)
num           = "0" / ( %x31-39 *DIGIT )
alnumdot      = 1*( ALPHA / DIGIT / "-" / "." )

; ----- lexical constants -----
SP            = %x20                                        ; canonical single space; pre-fmt allows 1*( SP / HTAB )
HTAB          = %x09
LF            = %x0A                                        ; sole line ending (CR/CRLF => MG-B07)
DQUOTE        = %x22
lcalpha       = %x61-7A                                     ; a-z
blank         = LF                                          ; canonical: one blank line between records

; ----- REJECTED, never produced above (do not appear in any production) -----
;   float  = 1*DIGIT "." 1*DIGIT                            => MG-L10
;   null   = %s"nothing" / %s"missing" / %s"undefined"      => MG-L09
;   comment= ( "//" / "#" ) *non-LF                         => MG-L01 (gap coverage)
;   list separators "and" / ":"                             => MG-S03
;   every WFL keyword outside {create, map, is, end, wflpkg, yes, no}, and
;   + - * / % . = { } ( )                                   => MG-S01
;   Gate B (pre-lex) enforces: UTF-8 well-formedness (no overlong), NFC, no BOM.
```

**Note on the deliberate gap.** WFL's language `string` production (`token.rs:457`) admits raw control chars, raw newlines, and the `\r`/`\0` escapes; this frozen `string`/`escape` is a strict subset of it. WFL's language `boolean` is case-insensitive over four spellings; this frozen `boolean` admits only `yes`/`no`. These are legal subsettings — stricter than, and always accepted by, the shared lexer — and they do not violate No-Unlearning: the beginner and expert read and write the same WFL, only the *manifest* is tightened.

---

*End of `wflpkg-data-literal-grammar 1.0.0-draft`.*
