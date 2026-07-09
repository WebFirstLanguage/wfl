# Crypto Module

The Crypto module provides cryptographic functions in three groups:

- **Password hashing** — `hash_password`/`verify_password` and the algorithm-specific Argon2id, bcrypt, scrypt and PBKDF2 functions. Use these to store user passwords safely.
- **Standard hashing/MAC** — `sha256` and `hmac_sha256` for interoperating with external services (webhook verification, API signing).
- **WFLHASH** — a custom hash algorithm for data integrity, checksums, and non-critical security applications.

> **Hashing a password? Use `hash_password`, never `sha256` or `wflhash256`.** Fast hashes are built to be quick, which is exactly what makes them a poor way to store passwords — an attacker can try billions of guesses per second. The password hashing functions below are deliberately slow and salted to prevent that.

## ⚠️ Important Security Disclaimer

**WFLHASH is NOT externally audited.** While it implements cryptographically sound design principles:

✅ **Suitable for:**
- Internal applications with controlled security requirements
- Non-critical data integrity verification
- Development and testing environments
- Checksums and data deduplication

❌ **NOT recommended for:**
- Applications requiring FIPS validation
- High-security environments requiring proven algorithms
- Regulatory compliance requiring validated cryptography
- Password hashing in production systems (use the [password hashing functions](#password-hashing) instead)

**For production security applications, use standard algorithms.** WFL provides standard `sha256` and `hmac_sha256` builtins (below) for exactly this: they are required when interoperating with external services (e.g. verifying Stripe or GitHub webhook signatures), where a custom algorithm cannot be used.

## Password Hashing

These functions store passwords the right way: they are **slow by design**, add a **random salt** automatically, and return a **self-describing string** that records the algorithm and its cost parameters. You store that one string and pass it straight back to the matching verify function later — nothing else needs to be saved.

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

**Registration and login example:**

```wfl
// When a user signs up, hash their password and store the result.
store password_hash as hash_password of "correct horse battery staple"
display "Store this in your database: " with password_hash

// When they log in later, verify the password they typed against the stored hash.
store attempt as "correct horse battery staple"
store login_ok as verify_password of attempt and password_hash

check if login_ok is yes:
    display "Welcome back!"
otherwise:
    display "Incorrect password."
end check
```

**Properties:**
- **Salted automatically** — hashing the same password twice gives two different strings, so identical passwords never share a hash and precomputed (rainbow-table) attacks don't work.
- **Slow on purpose** — each hash takes meaningful CPU/memory, which barely matters for a single login but makes mass cracking impractical.
- **Self-describing** — the salt and cost parameters live inside the returned string, so `verify_password` needs nothing but the password and that string.
- **Constant-time verification** — comparisons don't leak information through timing.

**Notes and limits:**
- The maximum password length is 4096 bytes.
- bcrypt only considers the first 72 bytes of a password (a property of the algorithm itself).
- Defaults follow current OWASP guidance (e.g. PBKDF2 uses 600,000 iterations; Argon2id uses memory-hard parameters). Prefer `hash_password` unless you have a specific reason to pick another algorithm.
- A malformed or unrecognized stored hash simply makes `verify_password` return `no` — it never errors, so a corrupted record can't crash a login.

---

## Functions

### wflhash256

**Purpose:** Generate a 256-bit hash of text.

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
```

**Properties:**
- Deterministic - Same input always produces same hash
- Fixed length - Always 64 hex characters (256 bits)
- Collision resistant - Different inputs produce different hashes
- One-way - Cannot reverse the hash

**Use Cases:**
- Data integrity verification
- File checksums
- Duplicate detection
- Cache keys
- Non-sensitive data hashing

---

### wflhash512

**Purpose:** Generate a 512-bit hash of text.

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
```

**Properties:**
- Same as wflhash256 but with 512-bit output
- More collision resistant
- Longer hash string

**Use Cases:**
- Higher security requirements
- When 256 bits isn't enough
- Cryptographic applications

**When to use 256 vs 512:**
- Use 256 for most cases (faster, shorter)
- Use 512 for higher security margin

---

### wflhash256_with_salt

**Purpose:** Generate a salted 256-bit hash for domain separation.

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
- User-specific hashing
- Preventing rainbow table attacks
- Key derivation

**Example: Per-User Hashing**
```wfl
define action called hash_password with parameters password and username:
    return wflhash256_with_salt of password and username
end action

store user1_hash as hash_password of "secret" and "alice"
store user2_hash as hash_password of "secret" and "bob"

// Even with same password, hashes are different
display "Alice's hash: " with user1_hash
display "Bob's hash: " with user2_hash
```

---

### wflmac256

**Purpose:** Generate a Message Authentication Code (MAC) with HKDF key derivation.

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

check if expected_signature is received_signature:
    display "Webhook is authentic"
otherwise:
    display "Webhook signature mismatch - reject it!"
end check
```

**Use Cases:**
- Verifying incoming webhook signatures (Stripe, GitHub, Slack, ...)
- Signing outgoing API requests
- Any integration that specifies HMAC-SHA256

**Note:** For WFL-internal message authentication where interoperability is not required, `wflmac256` also works; `hmac_sha256` is the standard everyone else speaks.

---

## Complete Example

```wfl
display "=== Crypto Module Demo ==="
display ""

// Basic hashing
store data as "Sensitive information"

store hash256 as wflhash256 of data
display "256-bit hash: " with hash256

store hash512 as wflhash512 of data
display "512-bit hash (first 64 chars): " with substring of hash512 from 0 length 64
display ""

// Verify determinism
store hash_again as wflhash256 of data
check if hash256 is equal to hash_again:
    display "✓ Hashes are deterministic"
end check
display ""

// Salted hashing
store password as "user_password"
store salt1 as "user@example.com"
store salt2 as "admin@example.com"

store hash1 as wflhash256_with_salt of password and salt1
store hash2 as wflhash256_with_salt of password and salt2

display "Same password, different salts:"
display "  User hash: " with substring of hash1 from 0 length 16 with "..."
display "  Admin hash: " with substring of hash2 from 0 length 16 with "..."
display "  Hashes are different: " with hash1 is not equal to hash2
display ""

// MAC (Message Authentication Code)
store key as "secret_authentication_key"
store message as "Transfer $100 to account 12345"

store mac as wflmac256 of message and key
display "Message: " with message
display "MAC: " with substring of mac from 0 length 32 with "..."

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

### File Integrity Checking

```wfl
define action called checksum_file with parameters filename:
    try:
        open file at filename for reading as file_handle
        wait for store file_content as read content from file_handle
        close file file_handle

        store hash as wflhash256 of file_content
        return hash
    when error:
        return nothing
    end try
end action

// Create checksum
store original_hash as checksum_file of "important.txt"
display "Original checksum: " with original_hash

// Later, verify file hasn't changed
store current_hash as checksum_file of "important.txt"

check if original_hash is equal to current_hash:
    display "✓ File is unchanged"
otherwise:
    display "⚠️ File has been modified!"
end check
```

### API Request Signing

```wfl
define action called sign_request with parameters request_data and api_key:
    store timestamp as current time in milliseconds
    store payload as request_data with "|" with timestamp
    store signature as wflmac256 of payload and api_key
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

✅ **Use salts for user data:** Prevents rainbow tables

✅ **Use MAC for authentication:** Verify message integrity

✅ **Keep keys secret:** Never expose keys in logs

✅ **Use strong keys:** Long, random keys are best

✅ **Limit input size:** Hash function has 100MB limit

❌ **Don't use fast hashes (sha256/wflhash) for passwords:** Use `hash_password`/`verify_password` (Argon2id, bcrypt, scrypt, PBKDF2)

❌ **Don't use without understanding limitations:** Not externally audited

❌ **Don't rely on for high-security:** Use proven algorithms (SHA-256, SHA-3, BLAKE3)

❌ **Don't log keys or MACs:** Security sensitive data

## WFLHASH Technical Details

### Design Features

- **Sponge construction** (similar to SHA-3)
- **24-round security margin**
- **Nothing-up-my-sleeve constants** from mathematical constants
- **HKDF-based key derivation** for MAC mode
- **Secure memory management** with zeroization
- **Constant-time MAC verification** prevents timing attacks

### Limitations

- **Not standardized** - Custom algorithm
- **Not externally audited** - Internal security review only
- **Not FIPS validated** - Cannot be used where FIPS required
- **Not quantum-resistant** - Like most current hash functions

### When to Use Alternatives

**Production passwords:** Use bcrypt, argon2id, or scrypt
**Regulatory compliance:** Use SHA-256, SHA-3, or BLAKE3
**Digital signatures:** Use RSA, ECDSA, or EdDSA
**Encryption:** Use AES, ChaCha20, or similar

**WFLHASH is for non-critical applications where an unaudited hash function is acceptable.**

## What You've Learned

In this module, you learned:

✅ **hash_password / verify_password** - Safe password storage with Argon2id by default
✅ **argon2 / bcrypt / scrypt / pbkdf2** - Algorithm-specific password hashing
✅ **sha256 / hmac_sha256** - Standard hashing and MAC for interoperability
✅ **wflhash256 / wflhash512** - 256-bit and 512-bit hashing
✅ **wflhash256_with_salt** - Salted hashing
✅ **wflmac256** - Message authentication codes
✅ **Use cases** - Password storage, checksums, integrity, deduplication, signing
✅ **Limitations** - WFLHASH is not audited; use standard algorithms for production security
✅ **Best practices** - Never store passwords with fast hashes, salting, key management

## Next Steps

**[Pattern Module →](pattern-module.md)**
Pattern matching utilities.

Or return to:
**[Standard Library Index →](index.md)**

---

**Previous:** [← Random Module](random-module.md) | **Next:** [Pattern Module →](pattern-module.md)
