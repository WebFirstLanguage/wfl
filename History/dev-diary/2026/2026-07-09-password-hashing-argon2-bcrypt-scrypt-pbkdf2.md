# Password Hashing: Argon2id, bcrypt, scrypt, PBKDF2

**Date:** 2026-07-09

## What Changed

WFL's crypto module had fast hashes (`sha256`, `hmac_sha256`, and the custom
`wflhash256`/`512`) but no way to hash a password safely. Fast hashes are the
wrong tool for password storage — they let an attacker who steals the hash
database try billions of guesses per second. The docs already told users to
"use bcrypt, argon2, or scrypt" for passwords, but the language gave them no
built-in way to do it.

This change adds ten password-hashing builtins covering all four industry
standards, with a two-function beginner path and algorithm-specific functions
for when a particular algorithm is required.

### The API

Generic (recommended default — Argon2id):

```wfl
store stored as hash_password of "correct horse battery staple"
store ok as verify_password of attempt and stored   // yes / no
```

`hash_password` uses Argon2id. `verify_password` reads the algorithm back out
of the stored hash string and dispatches to the right verifier, so it accepts
a hash produced by any of the functions below.

Algorithm-specific (secure defaults, one argument in, PHC/MCF string out):

| Algorithm | Hash | Verify | Output |
| --- | --- | --- | --- |
| Argon2id | `argon2_hash` | `argon2_verify` | `$argon2id$...` |
| bcrypt | `bcrypt_hash` | `bcrypt_verify` | `$2b$...` |
| scrypt | `scrypt_hash` | `scrypt_verify` | `$scrypt$...` |
| PBKDF2 | `pbkdf2_hash` | `pbkdf2_verify` | `$pbkdf2-sha256$...` |

Every `*_hash` function takes just the password: it generates a fresh random
16-byte salt, applies secure default cost parameters, and returns a
self-describing string. The salt and cost parameters live inside that string,
so `*_verify` needs nothing but the password and the stored string. This keeps
the beginner form and the expert form the same form (the No-Unlearning
Invariant): a beginner stores one string and verifies against it; a security
reviewer reads the same string and sees exactly which algorithm and parameters
were used.

### Design decisions

- **Secure defaults, not tunable knobs.** Rather than exposing cost parameters
  (which invite beginners to pick weak values), each function applies current
  best-practice defaults: Argon2id with memory-hard parameters, bcrypt cost 12,
  scrypt recommended parameters, and PBKDF2 at **600,000** iterations (the
  RustCrypto default of 4096 is far below OWASP guidance, so it is overridden
  explicitly). This upholds the "Built-in Security Features / secure defaults"
  principle.
- **Verification never errors.** A malformed or unrecognized stored hash makes
  the verify functions return `no` instead of raising — a corrupted database
  row can't crash a login handler.
- **`verify_password` auto-detects.** bcrypt uses the MCF `$2b$` format (not
  PHC), so it is detected by prefix and routed to bcrypt; everything else is
  parsed as a PHC string and matched by its algorithm identifier. An
  algorithm-specific verifier rejects a hash from a different algorithm.
- **Shared `password-hash` version.** `argon2`, `scrypt`, and `pbkdf2` (all
  RustCrypto) resolve to a single `password-hash` 0.5, so their `PasswordHash`,
  `SaltString`, and trait types unify and can be used interchangeably. bcrypt
  is a separate crate handled independently.
- **Input bound.** Passwords are capped at 4096 bytes before hashing. bcrypt
  additionally only considers the first 72 bytes (a property of the algorithm),
  which is documented.

## Implementation

- `src/stdlib/crypto.rs`: the ten native functions plus `random_salt`,
  `check_password_len`, and per-algorithm string helpers; registered in
  `register_crypto`.
- `src/builtins.rs`: added the ten names to the builtin registry and their
  arities (hash = 1, verify = 2).
- `src/stdlib/typechecker.rs` and `src/typechecker/mod.rs`: registered
  signatures and return types (`*_hash` → Text, `*_verify` → Boolean).
- `Cargo.toml`: `argon2 = "0.5"`, `scrypt = "0.11"` (feature `simple`),
  `pbkdf2 = "0.12"` (feature `simple`), `bcrypt = "0.15"`.

## Tests

- `tests/password_hashing_test.rs`: 21 async tests — roundtrips and prefixes
  for each algorithm, wrong-password rejection, salt uniqueness, garbage-hash
  handling, cross-algorithm auto-detection by `verify_password`, and
  algorithm-specific rejection.
- `TestPrograms/password_hashing_test.wfl`: end-to-end demo covering the
  generic path, each algorithm, and auto-detection.

## Docs

- `Docs/05-standard-library/crypto-module.md`: new "Password Hashing" section
  and updated disclaimers pointing away from fast hashes for passwords.
- `Docs/reference/builtin-functions-reference.md`: password-hashing and
  standard hashing/MAC tables.
- `Docs/06-best-practices/security-guidelines.md`: "Password Hashing" guidance.
- `Docs/05-standard-library/index.md`, `overview.md`: updated function listings.
