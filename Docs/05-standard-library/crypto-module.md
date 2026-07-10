# Crypto Module

The Crypto module provides cryptographic functions in four groups:

- **Password hashing** — `hash_password`/`verify_password` and the algorithm-specific Argon2id, bcrypt, scrypt and PBKDF2 functions. Use these to store user passwords safely.
- **Auth & session primitives** — `pbkdf2_hmac_sha256` (raw key derivation), `constant_time_equals` (timing-safe comparison), and `secure_random_bytes` (CSPRNG bytes for salts, tokens, and session IDs).
- **Standard hashing/MAC** — `sha256` and `hmac_sha256` for interoperating with external services (webhook verification, API signing).
- **WFLHASH (experimental)** — WFL's own hash family. Please try it, report results, and help us harden it. For production integrity, **pair it with a known-good hash** so a battle-tested algorithm always has your back.

> **Hashing a password? Use `hash_password`, never `sha256` or `wflhash256` alone.** Fast hashes are built to be quick, which is exactly what makes them a poor way to store passwords — an attacker can try billions of guesses per second. The password hashing functions below are deliberately slow and salted to prevent that. For sensitive material, also prefer **more than one hash** (see below).

## Multi-hash for sensitive data (recommended)

**Recommendation: for sensitive things, always hash with more than one algorithm.** Do not put full trust in a single hash primitive — especially for passwords, session secrets, recovery tokens, or high-stakes integrity tags.

Composing hashes is **defense in depth**: if one algorithm is experimental, mis-implemented, or later weakened, another still has your back.

| Sensitive thing | Multi-hash guidance |
| --- | --- |
| **Passwords** | Pre-mix with **two or more** general hashes, then always finish with a **password KDF** (`hash_password` / Argon2id, etc.). Never store only fast hashes. |
| **Integrity of secrets / files** | At least two digests in series (e.g. WFLHASH then `sha256`), or store two independent digests. |
| **Tokens / one-time secrets** | Prefer `secure_random_bytes` for generation; if you fingerprint a secret, multi-hash the fingerprint. |
| **External API signatures** | Follow the protocol (often a single specified MAC). Multi-hash only when *you* define the scheme. |

### Passwords: multi-hash *and* a password hasher

Passwords are the textbook sensitive case. Use **more than one general-purpose hash as a pre-mix**, then run the result through WFL's slow password hasher (which adds salt and cost parameters for you):

```wfl
// Sign-up — more than one hash, then a real password KDF
store step1 as wflhash256 of user_password          // experimental / first algorithm
store step2 as sha256 of step1                      // known-good second algorithm
store stored_hash as hash_password of step2         // Argon2id (slow, salted) — required final step

// Login — apply the same pre-mix, then verify
store attempt_step1 as wflhash256 of attempt
store attempt_step2 as sha256 of attempt_step1
store login_ok as verify_password of attempt_step2 and stored_hash
```

**Rules of thumb:**

1. **Always end with `hash_password` (or another password KDF).** Multi-hashing with only `wflhash` / `sha256` is still *fast* and still unsafe to store.
2. **Use at least two different algorithms** in the pre-mix when the data is sensitive (WFLHASH + `sha256` is the natural WFL pairing).
3. **Apply the exact same chain on verify** that you used on store — order and algorithms must match.
4. **`hash_password` already salts** — you do not need a separate salt in the pre-mix for correctness; domain separation is optional extra.

A minimal multi-hash without WFLHASH is fine too when you only want standards:

```wfl
// Two known-good layers in spirit: SHA-256 pre-image mix, then Argon2id
store preimage as sha256 of user_password
store stored_hash as hash_password of preimage
```

Prefer the WFLHASH + `sha256` + `hash_password` chain when you also want to exercise experimental WFLHASH behind a strong friend.

## ⚠️ WFLHASH is experimental

**WFLHASH is experimental and not externally audited.** It implements solid design ideas (sponge construction, strong round constants, secure memory cleanup), but it has not been through independent cryptanalysis the way SHA-2, SHA-3, or BLAKE3 have.

We want people to **use it and test it** — checksums, cache keys, demos, internal tools, and real workloads are all welcome feedback. When the result must hold up under real risk, give it a **strong friend**: finish with a known-good hash so production strength never rests on WFLHASH alone.

### The dual-hash (strong friend) pattern

Hash with WFLHASH first, then hash that digest with a proven algorithm (`sha256` today; any other standard hash you trust works the same way):

```wfl
// Experimental first pass — try WFLHASH freely
store wfl_digest as wflhash256 of file_content

// Strong friend — known-good algorithm backs up the result
store integrity_tag as sha256 of wfl_digest
```

**Why this is allowed in production:** if WFLHASH were ever weaker than expected, the outer `sha256` (or another audited hash) still provides the security properties of that standard algorithm. You get to exercise and stress-test WFLHASH while production integrity still rests on a proven primitive.

You can also store **both** digests when you want to compare WFLHASH behavior over time without giving up a standard checksum:

```wfl
store wfl_digest as wflhash512 of payload
store standard_digest as sha256 of payload
// Prefer standard_digest for interop and compliance; keep wfl_digest for testing WFLHASH
```

### When to use what

| Goal | Recommendation |
| --- | --- |
| Try / test / feedback on WFLHASH | `wflhash256` / `wflhash512` alone — please do |
| Production integrity (files, caches, internal digests) | **Multi-hash:** WFLHASH then `sha256` (or another known-good hash) |
| Interop with Stripe, GitHub, other APIs | `sha256` / `hmac_sha256` only (they will not speak WFLHASH) |
| Passwords (sensitive) | **Multi-hash pre-mix** (e.g. WFLHASH then `sha256`) **then** `hash_password` — never store fast hashes alone |
| FIPS / regulatory validated crypto | Standard algorithms only (`sha256`, etc.) |

✅ **Encouraged:**
- Experimentation, benchmarks, and community testing of WFLHASH
- Production use **when dual-hashed** with a known-good algorithm
- Checksums, deduplication, and cache keys (dual-hash for production)
- Development and teaching examples

❌ **Not appropriate:**
- Relying on WFLHASH **alone** as the sole integrity guarantee for high-stakes data
- Password storage (use [password hashing](#password-hashing))
- External protocols that require a specific standard algorithm
- Environments that demand only FIPS-validated or formally audited primitives (use the standard alone)

**Interoperability still needs standards alone.** WFL's `sha256` and `hmac_sha256` builtins are required when talking to external services (e.g. Stripe or GitHub webhook signatures). A custom algorithm cannot stand in there.

## Password Hashing

These functions store passwords the right way: they are **slow by design**, add a **random salt** automatically, and return a **self-describing string** that records the algorithm and its cost parameters. You store that one string and pass it straight back to the matching verify function later — nothing else needs to be saved.

**Recommended for sensitive credentials:** use **more than one hash** — a multi-algorithm pre-mix, then always finish with `hash_password`. See [Multi-hash for sensitive data](#multi-hash-for-sensitive-data-recommended).

**The two you'll usually want:**

| Function | Signature | Returns |
| --- | --- | --- |
| `hash_password` | `hash_password of <password>` | Text — a hash string to store |
| `verify_password` | `verify_password of <password> and <stored_hash>` | Boolean — `yes` if the password matches |

`hash_password` uses **Argon2id**, the current best-practice default. `verify_password` looks at the stored hash and automatically uses whichever algorithm produced it, so it verifies hashes made by any of the functions below.

**Choosing a specific algorithm.** If you need a particular algorithm (for interoperability, policy, or migration), use the matching pair. Every `*_hash` function takes just the password and applies secure defaults; every `*_verify` function takes the password and the stored hash.

| Algorithm | Hash | Verify | Output format |
| --- | --- | --- | --- |
| Argon2id | `argon2_hash of <password>` | `argon2_verify of <password> and <hash>` | `$argon2id$...` |
| bcrypt | `bcrypt_hash of <password>` | `bcrypt_verify of <password> and <hash>` | `$2b$...` |
| scrypt | `scrypt_hash of <password>` | `scrypt_verify of <password> and <hash>` | `$scrypt$...` |
| PBKDF2 | `pbkdf2_hash of <password>` | `pbkdf2_verify of <password> and <hash>` | `$pbkdf2-sha256$...` |

**Registration and login example (recommended multi-hash chain):**

```wfl
// When a user signs up: multi-hash pre-mix, then password KDF
store password as "correct horse battery staple"
store step1 as wflhash256 of password
store step2 as sha256 of step1
store password_hash as hash_password of step2
display "Store this in your database: " with password_hash

// When they log in: same pre-mix order, then verify
store attempt as "correct horse battery staple"
store attempt_step1 as wflhash256 of attempt
store attempt_step2 as sha256 of attempt_step1
store login_ok as verify_password of attempt_step2 and password_hash

check if login_ok is yes:
    display "Welcome back!"
otherwise:
    display "Incorrect password."
end check
```

**Minimal example (password KDF only):** fine for learning; for production-sensitive accounts prefer the multi-hash chain above.

```wfl
store password_hash as hash_password of "correct horse battery staple"
store login_ok as verify_password of "correct horse battery staple" and password_hash
```

**Properties:**
- **Salted automatically** — hashing the same password twice gives two different strings, so identical passwords never share a hash and precomputed (rainbow-table) attacks don't work.
- **Slow on purpose** — each hash takes meaningful CPU/memory, which barely matters for a single login but makes mass cracking impractical.
- **Self-describing** — the salt and cost parameters live inside the returned string, so `verify_password` needs nothing but the password and that string.
- **Constant-time verification** — comparisons don't leak information through timing.
- **Multi-hash friendly** — you may pre-mix with other hashes before `hash_password`; never replace the password KDF with fast hashes alone.

**Notes and limits:**
- The maximum password length is 4096 bytes.
- bcrypt only considers the first 72 bytes of a password (a property of the algorithm itself). Pre-hashing with `sha256` (64 hex chars) fits under that limit if you ever choose bcrypt.
- Defaults follow current OWASP guidance (e.g. PBKDF2 uses 600,000 iterations; Argon2id uses memory-hard parameters). Prefer `hash_password` unless you have a specific reason to pick another algorithm.
- A malformed or unrecognized stored hash simply makes `verify_password` return `no` — it never errors, so a corrupted record can't crash a login.
- If you multi-hash on store, you **must** multi-hash with the same algorithms and order on every verify.

---

## Functions

### wflhash256

**Purpose:** Generate a 256-bit **experimental** WFLHASH digest of text. For production integrity, pass the result through a known-good hash (see [dual-hash pattern](#the-dual-hash-strong-friend-pattern)).

**Signature:**
```wfl
wflhash256 of <text>
```

**Parameters:**
- `text` (Text): The text to hash

**Returns:** Text - 64-character hexadecimal hash string

**Example:**
```wfl
store hash1 as wflhash256 of "Hello, World!"
display "Hash: " with hash1
// Output: Hash: a1b2c3d4... (64 hex characters)

store hash2 as wflhash256 of "Hello, World!"
display "Same input, same hash: " with hash1 is equal to hash2
// Output: Same input, same hash: yes

store hash3 as wflhash256 of "Hello, World"
display "Different input: " with hash1 is equal to hash3
// Output: Different input: no

// Production integrity: pair with a known-good hash
store integrity_tag as sha256 of hash1
```

**Properties:**
- Deterministic - Same input always produces same hash
- Fixed length - Always 64 hex characters (256 bits)
- One-way - Cannot reverse the hash
- Experimental - Please test and report findings

**Use Cases:**
- Experiments, benchmarks, and community testing
- Data integrity (dual-hash for production)
- File checksums (dual-hash for production)
- Duplicate detection and cache keys

---

### wflhash512

**Purpose:** Generate a 512-bit **experimental** WFLHASH digest of text. Same dual-hash guidance as `wflhash256`.

**Signature:**
```wfl
wflhash512 of <text>
```

**Parameters:**
- `text` (Text): The text to hash

**Returns:** Text - 128-character hexadecimal hash string

**Example:**
```wfl
store hash as wflhash512 of "Important data"
display "512-bit hash: " with hash
// Output: 128 hex characters

// Production integrity: pair with a known-good hash
store integrity_tag as sha256 of hash
```

**Properties:**
- Same as wflhash256 but with 512-bit output
- Longer hash string for extra margin in experiments

**Use Cases:**
- Experiments that want a wider digest
- When you prefer a longer intermediate before dual-hashing

**When to use 256 vs 512:**
- Use 256 for most cases (faster, shorter)
- Use 512 when you want a longer experimental digest

---

### wflhash256_with_salt

**Purpose:** Generate a salted 256-bit **experimental** WFLHASH digest for domain separation.

**Signature:**
```wfl
wflhash256_with_salt of <text> and <salt>
```

**Parameters:**
- `text` (Text): The text to hash
- `salt` (Text): The salt/personalization string

**Returns:** Text - 64-character hexadecimal hash

**Example:**
```wfl
store hash1 as wflhash256_with_salt of "password" and "user1"
store hash2 as wflhash256_with_salt of "password" and "user2"

display "User 1 hash: " with hash1
display "User 2 hash: " with hash2
display "Same password, different hashes: " with hash1 is not equal to hash2
// Output: Same password, different hashes: yes
```

**Use Cases:**
- Domain separation (different contexts)
- User-specific integrity tags (not passwords)
- Per-tenant or per-feature digests
- Experiments that need distinct hash domains

**Example: Domain-separated digests (with strong friend for production)**
```wfl
define action called integrity_tag_for_user with parameters data and username:
    // WFLHASH with salt separates domains; sha256 is the known-good backup
    store wfl_digest as wflhash256_with_salt of data and username
    return sha256 of wfl_digest
end action

store alice_tag as integrity_tag_for_user of "profile-v1" and "alice"
store bob_tag as integrity_tag_for_user of "profile-v1" and "bob"

// Same data, different users → different tags
display "Alice's tag: " with alice_tag
display "Bob's tag: " with bob_tag
```

> **Not for passwords.** Use `hash_password` / `verify_password` for credentials.

---

### wflmac256

**Purpose:** Generate an **experimental** Message Authentication Code (MAC) with HKDF key derivation. Prefer `hmac_sha256` for external services and production auth that must interoperate with standards.

**Signature:**
```wfl
wflmac256 of <message> and <key>
```

**Parameters:**
- `message` (Text): The message to authenticate
- `key` (Text): The secret key

**Returns:** Text - 64-character hexadecimal MAC

**Example:**
```wfl
store secret_key as "my_secret_key_12345"
store message as "Important message"

store mac as wflmac256 of message and secret_key
display "MAC: " with mac

// Verify message hasn't changed
store received_message as "Important message"
store received_mac as wflmac256 of received_message and secret_key

check if mac is equal to received_mac:
    display "Message is authentic"
otherwise:
    display "Message has been tampered with!"
end check
```

**Use Cases:**
- Message authentication
- Data integrity with secret key
- API request signing
- Secure cookies

**Security features:**
- HKDF-based key derivation
- Constant-time MAC verification
- Secure memory management (zeroization)

---

### sha256

**Purpose:** Generate a standard SHA-256 hash (FIPS 180-4). Use whenever you need interoperability with other systems that expect SHA-256.

**Signature:**
```wfl
sha256 of <text>
```

**Parameters:**
- `text` (Text): The text to hash

**Returns:** Text - 64-character lowercase hexadecimal hash string

**Example:**
```wfl
store hash as sha256 of "hello world"
display "SHA-256: " with hash
// Output: SHA-256: b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
```

**Use Cases:**
- Checksums that other tools must be able to verify
- Content addressing and deduplication across systems
- Interoperating with external APIs that expect SHA-256 digests

---

### hmac_sha256

**Purpose:** Generate a standard HMAC-SHA256 message authentication code (RFC 2104). This is what most third-party services (Stripe, GitHub, Slack, AWS) use to sign webhooks and API requests.

**Signature:**
```wfl
hmac_sha256 of <message> and <key>
```

**Parameters:**
- `message` (Text): The message to authenticate
- `key` (Text): The secret key

**Returns:** Text - 64-character lowercase hexadecimal MAC

**Example (webhook verification, Stripe-style):**
```wfl
// Stripe signs webhooks with HMAC-SHA256(secret, "timestamp.payload")
store webhook_secret as "whsec_test_secret"
store event_time as "1614556800"
store payload as "{\"id\": \"evt_123\"}"

store signed_payload as event_time with "." with payload
store expected_signature as hmac_sha256 of signed_payload and webhook_secret

// In a real webhook handler this comes from the Stripe-Signature header
store received_signature as hmac_sha256 of signed_payload and webhook_secret

// Compare signatures with constant_time_equals, never `is` — see the note below.
check if constant_time_equals of expected_signature and received_signature is yes:
    display "Webhook is authentic"
otherwise:
    display "Webhook signature mismatch - reject it!"
end check
```

**Use Cases:**
- Verifying incoming webhook signatures (Stripe, GitHub, Slack, ...)
- Signing outgoing API requests
- Any integration that specifies HMAC-SHA256

**Note:** `wflmac256` is part of experimental WFLHASH. For production authentication, prefer `hmac_sha256`, or dual-protect: compute a WFL MAC for testing and still verify with a standard HMAC when talking to real clients.

> **Always compare signatures and tokens with [`constant_time_equals`](#constant_time_equals), not `is`.** A normal `is` comparison stops at the first differing byte, and the time it takes leaks how much of a secret an attacker guessed correctly. `constant_time_equals` takes the same time regardless, closing that side channel.

---

### pbkdf2_hmac_sha256

**Purpose:** Derive a key from a password using PBKDF2-HMAC-SHA256 with a caller-supplied salt, iteration count, and output length. Runs the iteration loop in native code, so the per-call cost is bounded and predictable.

**Signature:**
```wfl
pbkdf2_hmac_sha256 of <password> and <salt> and <iterations> and <length>
```

**Parameters:**
- `password` (Text): The password or passphrase.
- `salt` (Text): A unique salt. Use [`secure_random_bytes`](#secure_random_bytes) to generate one and store it alongside the derived key.
- `iterations` (Number): Iteration count. Follow current guidance (OWASP recommends **600,000** for PBKDF2-HMAC-SHA256). Must be at least 1.
- `length` (Number): Derived-key length in bytes (1–1024). The result is `2 × length` hex characters.

**Returns:** Text — the derived key as a lowercase hexadecimal string.

**When to use which PBKDF2 function:**
- **Storing a user's password?** Prefer [`hash_password`](#password-hashing) (or `pbkdf2_hash`). Those generate a random salt, pin a safe iteration count, and return a self-describing string you store as-is.
- **Deriving a key, or matching a PBKDF2 hash created by another system?** Use `pbkdf2_hmac_sha256`, which gives you full control over salt, iterations, and length.

**Example:**
```wfl
// Generate and store a salt once, then derive the key.
store salt as secure_random_bytes of 16
store derived_key as pbkdf2_hmac_sha256 of "correct horse battery staple" and salt and 600000 and 32
display "Store the salt and this key: " with derived_key

// To verify a login, re-derive with the SAME salt and compare in constant time.
store attempt_key as pbkdf2_hmac_sha256 of "correct horse battery staple" and salt and 600000 and 32
store login_ok as constant_time_equals of derived_key and attempt_key
// login_ok is yes
```

**Notes:**
- Deterministic — the same password, salt, iterations, and length always produce the same key.
- Store the salt with the derived key; you need it to re-derive on the next login.
- More iterations means more work for both you and an attacker. Do not lower the count to speed up logins.

---

### constant_time_equals

**Purpose:** Compare two strings in constant time. Use this whenever you compare a secret against attacker-supplied input — MACs, CSRF tokens, session IDs, password-reset codes.

**Signature:**
```wfl
constant_time_equals of <a> and <b>
```

**Parameters:**
- `a` (Text): First value.
- `b` (Text): Second value.

**Returns:** Boolean — `yes` if the two strings are byte-for-byte equal, otherwise `no`.

**Why not just use `is`?** A normal comparison returns as soon as it finds a difference, so it finishes faster the earlier the mismatch is. An attacker who can measure that timing can recover a secret one byte at a time. `constant_time_equals` always examines the full input, so the time it takes reveals nothing about *where* the values differ. (String length is not secret, so inputs of different lengths return `no` immediately.)

**Example:**
```wfl
store expected as hmac_sha256 of signed_payload and webhook_secret
store received as hmac_sha256 of signed_payload and webhook_secret

check if constant_time_equals of expected and received is yes:
    display "Signature valid"
otherwise:
    display "Signature invalid - reject"
end check
```

**Use Cases:**
- Verifying HMAC / webhook signatures
- Checking CSRF tokens and session identifiers
- Comparing password-reset or email-verification codes

---

### secure_random_bytes

**Purpose:** Generate cryptographically secure random bytes from the operating system's CSPRNG, returned as a hex string. Use this for salts, session identifiers, CSRF tokens, and password-reset tokens.

**Signature:**
```wfl
secure_random_bytes of <n>
```

**Parameters:**
- `n` (Number): Number of random bytes to generate (1–4096). The result is `2 × n` hexadecimal characters.

**Returns:** Text — `n` random bytes encoded as a lowercase hexadecimal string.

**Example:**
```wfl
store salt as secure_random_bytes of 16      // 32 hex chars, for a password salt
store session_id as secure_random_bytes of 32 // 64 hex chars, for a session cookie
display "New session id: " with session_id
```

**Why not build tokens from `random_int`?** Composing a token out of numeric random values risks *modulo bias* (some values become more likely than others) and *under-entropy* (fewer truly random bits than the length suggests). `secure_random_bytes` draws uniform bytes straight from the OS CSPRNG, avoiding both.

> **Never call `random_seed` in authentication, session, or cryptographic code.** Seeding the general-purpose random generator makes its output predictable, which would compromise anything built on it. WFL's static analyzer (`wfl --analyze`) reports this as an error, so CI can block it. `secure_random_bytes` is unaffected by `random_seed`, but seeding signals a dangerous pattern in security code.

**Use Cases:**
- Password salts (pair with [`pbkdf2_hmac_sha256`](#pbkdf2_hmac_sha256))
- Session identifiers and cookies
- CSRF tokens, password-reset tokens, API keys

---

## Complete Example

```wfl
display "=== Crypto Module Demo ==="
display ""

// Experimental WFLHASH — try it freely
store data as "Example integrity payload"

store hash256 as wflhash256 of data
display "WFLHASH-256: " with hash256

store hash512 as wflhash512 of data
display "WFLHASH-512 (first 64 chars): " with substring of hash512 from 0 length 64
display ""

// Dual-hash: WFLHASH then a known-good friend for production integrity
store production_tag as sha256 of hash256
display "Production integrity tag (sha256 of WFLHASH): " with production_tag
display ""

// Verify determinism
store hash_again as wflhash256 of data
check if hash256 is equal to hash_again:
    display "✓ WFLHASH is deterministic"
end check
display ""

// Salted domain separation (not for passwords)
store payload as "shared-value"
store salt1 as "user@example.com"
store salt2 as "admin@example.com"

store hash1 as wflhash256_with_salt of payload and salt1
store hash2 as wflhash256_with_salt of payload and salt2

display "Same payload, different salts:"
display "  User hash: " with substring of hash1 from 0 length 16 with "..."
display "  Admin hash: " with substring of hash2 from 0 length 16 with "..."
display "  Hashes are different: " with hash1 is not equal to hash2
display ""

// MAC demo (experimental WFLHASH MAC — use hmac_sha256 for external services)
store key as "secret_authentication_key"
store message as "Transfer $100 to account 12345"

store mac as wflmac256 of message and key
display "Message: " with message
display "WFL MAC: " with substring of mac from 0 length 32 with "..."

// Tampered message
store tampered as "Transfer $999 to account 12345"
store tampered_mac as wflmac256 of tampered and key

check if mac is equal to tampered_mac:
    display "Message authentic"
otherwise:
    display "⚠️ Message has been tampered with!"
end check
display ""

display "=== Demo Complete ==="
```

## Common Patterns

### File Integrity Checking (dual-hash for production)

```wfl
define action called checksum_file with parameters filename:
    try:
        open file at filename for reading as file_handle
        wait for store file_content as read content from file_handle
        close file file_handle

        // Experimental pass — helps test WFLHASH in the wild
        store wfl_digest as wflhash256 of file_content
        // Strong friend — known-good hash backs production integrity
        store integrity_tag as sha256 of wfl_digest
        return integrity_tag
    when error:
        return nothing
    end try
end action

// Create checksum
store original_hash as checksum_file of "important.txt"
display "Original integrity tag: " with original_hash

// Later, verify file hasn't changed
store current_hash as checksum_file of "important.txt"

check if original_hash is equal to current_hash:
    display "✓ File is unchanged"
otherwise:
    display "⚠️ File has been modified!"
end check
```

### API Request Signing

For external APIs, use standard HMAC. For internal WFL-only experiments you may use `wflmac256`, but prefer the standard when anything leaves your process:

```wfl
define action called sign_request with parameters request_data and api_key:
    store timestamp as current time in milliseconds
    store payload as request_data with "|" with timestamp
    // Known-good MAC for production / interop
    store signature as hmac_sha256 of payload and api_key
    return signature
end action

store api_data as "action=transfer&amount=100"
store api_key as "secret_api_key_xyz"

store signature as sign_request of api_data and api_key
display "Request signature: " with signature
```

### Data Deduplication

```wfl
store seen_hashes as []
store unique_items as []

store items as ["apple", "banana", "apple", "cherry", "banana"]

for each item in items:
    // WFLHASH alone is fine for in-memory dedup experiments
    store hash as wflhash256 of item
    store hash_index as indexof of seen_hashes and hash

    check if hash_index is equal to -1:
        // First time seeing this hash
        push with seen_hashes and hash
        push with unique_items and item
    end check
end for

display "Unique items: " with unique_items
```

## Security Best Practices

✅ **Multi-hash sensitive data:** For passwords and other high-stakes material, use **more than one** hash algorithm (see [multi-hash](#multi-hash-for-sensitive-data-recommended))

✅ **Passwords: multi-hash then password KDF:** e.g. WFLHASH → `sha256` → `hash_password` — never store only fast hashes

✅ **Test WFLHASH:** Use it in real code and share feedback — experimental means we want exercise, not shelf-ware

✅ **Dual-hash for production integrity:** WFLHASH then `sha256` (or another known-good hash) so a proven algorithm always backs you up

✅ **Use salts for domain separation:** `wflhash256_with_salt` keeps contexts apart

✅ **Use standard MACs for external auth:** `hmac_sha256` for webhooks and third-party APIs

✅ **Keep keys secret:** Never expose keys in logs

✅ **Use strong keys:** Long, random keys are best

✅ **Limit input size:** Hash functions enforce a 100MB limit

❌ **Don't store passwords with only fast hashes (sha256/wflhash):** Multi-hash pre-mix is good; final step must be `hash_password` / Argon2id / bcrypt / scrypt / PBKDF2

❌ **Don't rely on a single hash alone** for sensitive data

❌ **Don't rely on experimental WFLHASH alone** for high-stakes integrity — always pair with a known-good hash

❌ **Don't log keys or MACs:** Security-sensitive data

## WFLHASH Technical Details

### Status

**Experimental.** Community testing welcome. Production use is supported when you follow the dual-hash (strong friend) pattern above.

### Design Features

- **Sponge construction** (similar to SHA-3)
- **24-round security margin**
- **Nothing-up-my-sleeve constants** from mathematical constants
- **HKDF-based key derivation** for MAC mode
- **Secure memory management** with zeroization
- **Constant-time MAC verification** prevents timing attacks

### Limitations

- **Experimental** — API and security claims may evolve as testing continues
- **Not standardized** — Custom algorithm, no external interop by itself
- **Not externally audited** — Internal review and community testing only so far
- **Not FIPS validated** — Use standard algorithms where FIPS is required
- **Not quantum-resistant** — Like most current hash functions

### When to Use Alternatives (alone)

**Production passwords:** multi-hash pre-mix recommended, then always `hash_password` (Argon2id) or the algorithm-specific helpers  
**Regulatory / FIPS-only paths:** `sha256` (or another validated standard) without depending on WFLHASH  
**External webhooks / API signing:** `hmac_sha256`  
**Digital signatures / encryption:** Not provided by this module — use appropriate external tooling

**WFLHASH is experimental: test it freely; for production integrity and sensitive data, bring a strong friend (`sha256` or another known-good hash) — and for passwords, finish with a password KDF.**

## What You've Learned

In this module, you learned:

✅ **Multi-hash for sensitive data** - Always prefer more than one hash (passwords especially)
✅ **hash_password / verify_password** - Required password KDF (Argon2id by default)
✅ **argon2 / bcrypt / scrypt / pbkdf2** - Algorithm-specific password hashing
✅ **sha256 / hmac_sha256** - Standard hashing and MAC for interoperability
✅ **wflhash256 / wflhash512** - Experimental WFLHASH (test it!)
✅ **Dual-hash production pattern** - WFLHASH then a known-good hash
✅ **wflhash256_with_salt** - Salted / domain-separated hashing
✅ **wflmac256** - Experimental message authentication codes
✅ **Use cases** - Passwords, checksums, integrity, deduplication, signing
✅ **Limitations** - Experimental, unaudited alone; pair with standards for production
✅ **Best practices** - Multi-hash sensitive material; never store passwords with only fast hashes

## Next Steps

**[Pattern Module →](pattern-module.md)**
Pattern matching utilities.

Or return to:
**[Standard Library Index →](index.md)**

---

**Previous:** [← Random Module](random-module.md) | **Next:** [Pattern Module →](pattern-module.md)
