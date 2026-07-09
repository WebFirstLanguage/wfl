# wflpkg: The Two Open Decisions — Resolved

*Decision record. Produced 2026-07-08 via a focused adversarial panel (Neil Hargrove / Pragmatist, Dr. Sylvia Okafor / Compliance Architect, Marcus Venn / Frontier Engineer, Priya Ashworth / Systems Disruptor), grounded by Zara Lenn (Researcher). Continues `wflpkg-brainstorm-results.md`, which settled the rest of the design.*

---

## Purpose of this document

The earlier brainstorm settled the entire spine of the WFL package manager and deliberately left exactly two questions for the owner, because they trade WFL's aesthetic against ecosystem interoperability and are values calls rather than technical unknowns:

1. **Canonical manifest format** — TOML/JSON, or a WFL-native typed data literal?
2. **Content-addressed identity** — adopt hash-of-AST as the identity primitive, or not?

Brad supplied his priorities, the four experts argued both questions, and a neutral researcher checked the two feasibility questions that came out of the debate. This document records the resolutions, the reasoning, the conditions attached to each, and the honest dissent that remains.

---

## Brad's stated priorities (the inputs that anchored the panel)

Three owner-level calls framed the debate:

- **Ecosystem scope: "public but self-sufficient."** wflhub.org will be open and public, but WFL intends to ship its *own* first-party tooling (SBOM, audit, scanning) rather than depend on the existing third-party supply-chain ecosystem.
- **Aesthetic vs interop: "aesthetic wins."** When WFL's natural-language aesthetic conflicts with slotting into existing structured-format tooling, the WFL-native form should win by default.
- **Content-addressing: "let the panel debate this."** No thumb on the scale — Brad wanted a genuine expert argument.

The first two priorities substantially weaken the strongest argument that previously favored TOML/JSON (free third-party tooling), because Brad is choosing to build that tooling himself and to prioritize coherence over borrowing the existing ecosystem.

---

## Research Summary (the facts that grounded the ruling)

Two feasibility questions surfaced during the debate. Zara resolved both on 2026-07-08.

**1. Homoglyph / confusable control is buildable on maintained dependencies — do not hand-roll it.**
The `unicode-security` crate (unicode-rs org; MIT/Apache-2.0; ~106k downloads/month; used by `rustc`'s `confusable_idents` and `mixed_script_confusables` lints) implements the full set of UTS #39 pieces the panel wanted: the confusables skeleton algorithm (§4), mixed-script detection (§5.1), and restriction-level detection (§5.2), plus Identifier-Status filtering. Its release cadence is slow (latest 0.1.2), so its embedded Unicode confusables data may trail the newest Unicode release — pin it and periodically re-check its `UNICODE_VERSION` against policy. But the control does not have to be hand-written. (The alternative, `unicode_skeleton`, is effectively unmaintained since 2017 and covers only §4 — do not use it as the primary dependency.)

**2. WFL has no canonical AST serialization today — the structure-hash normalizer is net-new (but bounded) work.**
Inspection of the mounted WFL codebase found no comment/whitespace-insensitive, deterministic normal form of the AST. AST nodes in `src/parser/ast.rs` derive only `Debug, Clone, PartialEq` (no `Serialize`, no `Hash`) and embed `line`/`column` fields, so any naive hash would be layout-sensitive. The `--parse` dump uses Rust's `{:#?}` Debug formatter (not a stability contract), and the `src/fixer/` auto-formatter is AST-driven and close, but config-dependent and not proven byte-deterministic. Producing a real structure-hash requires a focused new pass: strip source positions and comments, canonicalize order-insensitive constructs, emit a stable versioned encoding, and add determinism/idempotence tests. The AST is already a clean typed tree, so this is small-to-moderate effort — but it is net-new, not a drop-in. **Consequence for Decision 2:** committing the lockfile *schema field* is free; producing the hash that fills it is a bounded v1 build item, which is exactly why full content-addressed identity must not be the day-one foundation.

---

## Decision 1 — Canonical manifest format

### Resolution

**Adopt the WFL-native typed data literal as the canonical source of truth — implemented as a small, restricted, frozen data-literal subset of WFL, not the full language parser — with a deterministic JSON projection for interop.**

This was **unanimous (4/4)**, and the convergence crossed the conservative/liberal line: Neil and Sylvia (the cautious pair) landed in the same place as Marcus and Priya (the radical pair). That is the strongest possible signal.

### Why

The debate's key move was rejecting a false binary. The choice was never "TOML/JSON" versus "expose the entire Turing-complete WFL parser to untrusted registry metadata." Both of those are wrong. The right answer is a **third thing**: a data-literal dialect that *shares WFL's lexer* (one tokenizer, one place where encoding bugs live) but admits **only** key/value bindings and string/number/boolean/null/list/record literals — and **forbids** actions, control flow, arithmetic, interpolation, references, imports, and I/O. A manifest is a *value*, never a *program*. It is deserialized, never executed.

That single design decision reconciles every constraint at once:

- **Brad's aesthetic-wins priority** is honored — the manifest reads as ordinary WFL binding syntax, English honest all the way down, and the `wfl add` / `wfl why` view is native.
- **The No-Unlearning Invariant** holds — the beginner and expert read and write the same WFL syntax; there is no second dialect (TOML) to learn.
- **Security is *better* than the alternatives**, not worse. Marcus and Priya's "one parser to fuzz" is, correctly analyzed, a genuine security win: it eliminates the parser-differential attack class (where a compiler and an auditor disagree about what the same bytes mean and an adversary hides a payload in the gap) — *provided* it is literally one shared parser. Because the subset is data-only and non-computing, its attack surface is *smaller* than TOML's, not larger. The naive "just point the full parser at the manifest" version would have been disqualifying; the restricted subset is not.
- **Self-sufficiency does not mean isolation.** A deterministic `wfl manifest --json` projection (canonical key ordering, canonical number/string forms, stable schema version) gives any external tool a byte-stable structured view. The WFL literal is the source of truth; the JSON is a derived, lossless export. The door to the wider ecosystem stays open; WFL simply doesn't *depend* on anyone walking through it.

### Conditions (non-negotiable, from Sylvia's security ruling)

These are launch gates, not fast-follows:

1. **Frozen, documented, data-literal-only grammar** — a separate grammar artifact, versioned *independently of the language*, so language evolution never silently becomes a manifest-format (and therefore supply-chain) change. No expressions, no interpolation, no calls, no imports, no conditionals, no references.
2. **No Turing-complete evaluation** — the manifest parser returns a data tree or an error; it cannot invoke the interpreter.
3. **Canonical byte normalization: NFC on ingest**, with non-NFC input *rejected*, not silently repaired (silent normalization is itself a differential).
4. **Confusable/homoglyph rejection** on all identity-bearing fields (scope, package, version) — restrict to an explicit codepoint allowlist and reject mixed-script/whole-script confusables per UTS #39, built on the maintained `unicode-security` crate (see Research Summary).
5. **A single shared, continuously-fuzzed parser** used byte-identically by the compiler *and* every first-party tool — no "good enough" reimplementations, enforced by a differential fuzzing harness asserting identical parse trees. (Brad's self-sufficiency makes this enforceable by fiat: he owns all the parsers.)
6. **Reject-don't-repair** on any ambiguity, duplicate key, or overlong encoding, plus a **canonical manifest formatter (`wfl fmt`)** shipped day one so the on-disk form is byte-deterministic — which is what makes content hashing and human diff review sane.

### The one liability to write down

Neil's warning, on the record: **"we'll build our own SBOM/audit tooling" is a forever-commitment, not a one-time cost.** SBOM means tracking SPDX/CycloneDX revisions, ingesting CVE feeds, and maintaining VEX — a perpetual treadmill WFL is now choosing to own with a small team, in exchange for coherence and control. This is an accepted, deliberate liability, not an oversight. It should be budgeted as ongoing, and the fuzzing budget for condition 5 is part of the launch cost.

---

## Decision 2 — Content-addressed identity

### Resolution

**Hybrid. Identity stays `scope/name@version` (name-based). The normalized-AST hash is the lockfile and resolution trust anchor from v1 — the schema carries it from day one. Full content-addressed identity (hash *as* identity, a content-addressable store, fetch-by-hash) is a deferred, additive, non-breaking future step, gated on explicit conditions — it is NOT the day-one foundation.**

Brad asked for a real debate, and he got one. The striking result is that all four experts — including Marcus, the idea's champion — converged on the *same day-one answer*. The only spread is how enthusiastically each would eventually promote the anchor to full content-addressing, and that spread does not affect what gets built now.

### Why

The reconciling insight is a **two-layer trust doctrine** (Priya, echoed by Sylvia): separate **integrity** from **honesty**.

- **Integrity** — "the bytes I received are the bytes the world witnessed" — is delivered *cheaply and totally* by a content hash plus the transparency log and client-side sumdb, and is verifiable **without trusting the registry**. This kills version-swap and post-publication tampering and blunts slopsquatting (a pinned hash can't be substituted; a hallucinated name resolves to nothing). Nearly all of content-addressing's *defensive value* lives here.
- **Honesty** — "this source is not malicious" — is something a hash **cannot** provide. The hash of a backdoor is a perfectly valid hash; TanStack and Miasma both shipped malware with valid provenance. Honesty requires human review, per-release approval, and behavioral/anomaly detection. Nearly all of content-addressing's *risk* lives in trying to make it the identity primitive.

Because the value concentrates in the anchor and the risk concentrates in the identity primitive, the correct move is to **take the value and refuse the risk**: use the AST-hash as the lockfile anchor, keep `scope/name@version` as identity.

Three further facts settle the framing:

- **The Deno lesson.** Pure content-addressing *failed operationally* (dedup and trust) until Deno layered a central, semver-aware index (JSR) over it. So the honest "yes" was always "content-addressing *plus* a central index" — meaning you build the boring thing anyway *and* the novel thing on top. Even the advocates accept this.
- **Windows-first reality.** A content-addressable store wants reflink→hardlink→copy dedup; on WFL's primary platform, reflinks need ReFS/Dev Drive, NTFS falls back to hardlinks (with their own quirks), and the copy fallback silently erases the dedup benefit for exactly the users least able to debug it. Shipping full CAFS on day one lands novelty risk on the weakest platform.
- **The agent-first threat model cuts against name-free identity.** AI agents hallucinate *names*; a name-free (digest-only) identity layer gives agents and human reviewers nothing stable to be corrected against, and complicates the human review that the honesty layer depends on. Names+scopes+versions are what agents resolve and reviewers approve — keep them as identity.

### What "hybrid" concretely commits the data model to

- **Identity** = `scope/name@version` — human- and agent-legible, the unit of review and resolution.
- **Lockfile** records `scope/name@version → ast_hash`. **MVS resolves over names but pins to hashes.** The registry index is authoritative for name→version→hash but is **not trusted for integrity** — sumdb + transparency log are.
- **Put the `ast_hash` field in the lockfile/resolution schema on day one.** This is the regret-minimizing move: it is the single piece the data model actually gates on, and including it now prevents the churn (re-issuing every lockfile) that deferring it would cause later. Per the research, the *schema commitment* is free; the *normalizer* that produces the hash is a bounded, net-new v1 build item (strip line/column and comments, versioned canonical encoding, determinism tests).
- **A bonus the anchor buys:** because the hash is over the *normalized* AST, two releases with identical structure but different formatting are detectably equivalent, and a version whose bytes change without changing its AST is a signal worth surfacing to the honesty layer.

### Conditions on the deferred Phase 2 (full content-addressing)

Promotion of the hash to primary identity (CAFS store, hash-keyed resolution, fetch-by-hash) is additive and non-breaking, and is gated on all three of:

1. **Frozen canonical AST normalization**, proven deterministic and idempotent (this couples directly to Decision 1's canonical serialization and the `wfl fmt` work).
2. **A central, semver-aware index built *first*** — the JSR lesson, designed in from the start rather than bolted on after.
3. **A Windows CAFS story that degrades reflink→hardlink→copy without breaking correctness**, validated on **NTFS without Developer Mode first**.

### Honest dissent (flagged, not flattened)

Everyone agrees on the day-one answer. The residual disagreement is purely about the *destination*:

- **Neil (coldest):** name+version+MVS with hash-as-anchor is the base; full content-addressing should be revisited *only* as a future feature and "never the foundation." He is skeptical it should ever be promoted.
- **Marcus (warmest):** yes to hash-as-identity *as the destination* — ship the anchor first (Phase 1), earn the promotion (Phase 2) once the three conditions are met.
- **Sylvia and Priya (center):** the door stays open but nothing is a committed destination — full content-addressing is an additive option to be justified on its own merits when the time comes, riding *on top of* the semver index, never underneath it.

This is a roadmap-framing difference, not a data-model conflict — the hybrid keeps the path open either way, so the disagreement does not need to be resolved to proceed.

---

## Net effect on the build

Both resolutions preserve WFL's aesthetic without paying for it in safety, and both keep the day-one implementation shippable while leaving the ambitious options open:

- **Manifest:** WFL-native, but as a locked-down data-literal subset with six hard controls and a JSON projection. Aesthetic win *and* smaller attack surface than TOML.
- **Identity:** name-based, AST-hash-anchored from v1, full content-addressing deferred behind three explicit gates. Most of the structural-trust benefit now, at a fraction of the novelty risk, with a clean upgrade path.

Everything else in `wflpkg-brainstorm-results.md` was already settled. With these two decisions closed, the data model is no longer blocked.

---

## Next steps

1. **Write the frozen data-literal grammar** as a standalone, independently-versioned artifact (Decision 1, condition 1). Specify the exact admitted node set and the rejection rules.
2. **Build the single shared manifest parser** (data-literal subset over the shared lexer), wire it into a differential fuzzing harness, and add the NFC-normalization and reject-don't-repair behavior (conditions 2, 3, 6).
3. **Ship `wfl fmt` for manifests** (byte-deterministic canonical form) and the deterministic `wfl manifest --json` projection.
4. **Add the confusable/homoglyph control** on scope/package/version fields using `unicode-security` (skeleton + mixed_script + restriction_level); pin it and record its `UNICODE_VERSION` against policy (condition 4).
5. **Add the `ast_hash` field to the lockfile and resolution schema now** — even before the normalizer exists — so the data model is committed and churn-free.
6. **Build the normalized-AST structure-hash pass** (strip `line`/`column` and comments, canonicalize order-insensitive constructs, stable versioned encoding, determinism + idempotence tests). This is the bounded net-new work the research identified.
7. **Wire MVS to resolve over names but pin to hashes**, verified client-side against sumdb + transparency log.
8. **Record the three Phase-2 gates** (frozen AST normalization; central semver index first; Windows reflink→hardlink→copy validated on NTFS-without-Dev-Mode) as an explicit, tracked roadmap milestone — do not let it block launch.
9. **Budget the first-party SBOM/audit tooling as an ongoing commitment**, per Neil's flagged liability, and fund the manifest fuzzing as a launch gate.
