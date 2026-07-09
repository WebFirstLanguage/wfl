# Auth & Session Crypto Primitives (section 13.1, launch-blocking)

**Date:** 2026-07-09

## What Changed

The password-hashing builtins gave WFL a safe way to *store* a password, but
auth and session code still had gaps that pushed developers toward dangerous
hand-rolled code. This change closes the three launch-blocking gaps in section
13.1 and adds a static-analysis rule to keep a fourth foot-gun out of security
code.

Three new native builtins in `src/stdlib/crypto.rs`:

```wfl
// Raw PBKDF2-HMAC-SHA256 key derivation (caller controls salt/iterations/length)
store key as pbkdf2_hmac_sha256 of "password" and salt and 600000 and 32

// Timing-safe comparison — for MACs, tokens, session IDs, reset codes
store ok as constant_time_equals of expected and received

// CSPRNG bytes as hex — for salts, session IDs, CSRF/reset tokens
store salt as secure_random_bytes of 16
```

And one analyzer rule (`ANALYZE-SECURITY`): `random_seed` is now an error in any
file that also performs cryptographic, authentication, or session work.

## Why

Each of these was a concrete way for correct-looking WFL to be insecure:

- **`pbkdf2_hmac_sha256`** — under the serialized-response server model, running a
  600k-iteration KDF in interpreted WFL on `/login` is a whole-site DoS. Moving
  the iteration loop into Rust bounds the per-hash cost. Unlike `pbkdf2_hash`
  (which self-selects salt/iterations and returns a PHC string), this is the raw
  KDF: it takes an explicit salt, iteration count, and output length, so it can
  derive keys and interoperate with PBKDF2 hashes produced elsewhere. Validated
  against the standard PBKDF2-HMAC-SHA256 test vectors.

- **`constant_time_equals`** — WFL previously exposed only short-circuiting
  comparison (`is`), which leaks, through timing, how much of a secret a caller
  guessed correctly. That left no correct way to verify a MAC or token — a
  No-Unlearning violation. The `subtle` crate was already a dependency. The docs'
  webhook-verification example (which compared signatures with `is`) is updated
  to use `constant_time_equals`.

- **`secure_random_bytes`** — composing tokens from numeric `random_int` risks
  modulo bias and under-entropy. This exposes the OS CSPRNG directly for salts,
  session IDs, and CSRF/reset tokens.

- **`random_seed` lint** — seeding the general-purpose RNG makes its output
  predictable. Seeding it inside auth/session/crypto code is almost always a
  mistake, so the analyzer now flags it. The heuristic: if a file calls any
  crypto builtin *and* calls `random_seed`, every `random_seed` call site is
  reported as an error. `wfl --analyze` exits non-zero, so CI can block it.
  Seeding remains allowed in ordinary code (simulations, reproducible demos).

## Implementation Notes

- Output encoding is lowercase hex for all three builtins, matching the rest of
  the crypto module (`sha256`, `hmac_sha256`, `generate_csrf_token`).
- Bounds guard against runaway cost/allocation: iterations ≤ 100,000,000,
  derived-key length ≤ 1024 bytes, `secure_random_bytes` n ≤ 4096. Zero is
  rejected for iterations, length, and n.
- Derived-key and random buffers are zeroized after encoding.
- `constant_time_equals` short-circuits on unequal length (length is not secret)
  and otherwise compares via `subtle`'s `ct_eq`.
- The analyzer walks statement bodies (actions, loops, if/try, test blocks,
  websocket/event handlers, container methods) so seeding is caught inside
  nested scopes, not just at the top level.

## Tests

- `tests/crypto_kdf_test.rs` — RFC/NIST-style PBKDF2 vectors (c=1/2/4096,
  dkLen=32/40), constant-time equality (equal/unequal/length-mismatch, HMAC
  verification), and `secure_random_bytes` (length, uniqueness, bounds).
- `src/analyzer/static_analyzer.rs` unit tests — the lint fires with crypto
  present (top-level and inside an action body) and stays silent without it.
- `TestPrograms/crypto_auth_primitives_test.wfl` — end-to-end coverage including
  a password-storage round-trip using the raw KDF + constant-time compare.

## Docs

- `Docs/05-standard-library/crypto-module.md` — new "Auth & session primitives"
  group and full function entries; webhook example fixed to constant-time compare.
- `Docs/reference/builtin-functions-reference.md` — new reference rows.
