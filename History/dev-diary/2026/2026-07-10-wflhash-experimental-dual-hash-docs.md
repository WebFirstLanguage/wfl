# Dev Diary — 2026-07-10: WFLHASH documented as experimental (dual-hash production path)

## What changed

Documentation for WFLHASH and sensitive hashing was updated so the language surface matches the project's real security story:

- **WFLHASH is experimental** — not externally audited; community testing is encouraged.
- **Production use is allowed when dual-hashed** with a known-good algorithm (WFLHASH first, then e.g. `sha256`).
- That outer standard hash is the "strong friend": production integrity never rests on the experimental primitive alone.
- **Multi-hash recommendation for sensitive data** — especially passwords: pre-mix with more than one general hash, then always finish with `hash_password` (Argon2id). Never store only fast hashes.
- External-interop paths remain on dedicated APIs (`sha256` / `hmac_sha256`).

## Why

Custom hashes are valuable for brand, teaching, and iterative hardening, but they should not be the sole load-bearing integrity guarantee. Dual-hashing lets people run WFLHASH in real workloads (more testing feedback) while production strength still comes from a battle-tested algorithm. Extending that to "always more than one hash for sensitive things" makes defense-in-depth the default lesson for passwords and high-stakes digests.

Note: WFL currently ships `sha256` (not a separate `sha512` builtin). Docs use `sha256` as the known-good friend in examples; any other proven hash works the same pattern.

## Files touched

- `Docs/05-standard-library/crypto-module.md` — main policy, dual-hash pattern, examples
- `Docs/06-best-practices/security-guidelines.md` — WFLHASH usage section
- `Docs/reference/builtin-functions-reference.md` — experimental labels + security note
- `Docs/05-standard-library/index.md`, `overview.md`
- `Docs/06-best-practices/index.md`
- `Docs/README.md`, `Docs/05-standard-library/typechecker-module.md` (brief mentions)

## Not changed

- Runtime implementation of WFLHASH (`src/stdlib/crypto.rs`) — docs-only policy update
- Package/registry checksum design (still prefers standard hashes for cross-trust boundaries)
