# Crypto Module

The Crypto module provides cryptographic hashing functions using WFLHASH, a custom hash algorithm. Use for data integrity, checksums, and non-critical security applications.

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
- Password hashing in production systems

**For production security applications, use SHA-256, SHA-3, or BLAKE3.**

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
define action called hash password with parameters password and username:
    return wflhash256_with_salt of password and username
end action

store user1_hash as hash password with "secret" and "alice"
store user2_hash as hash password with "secret" and "bob"

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
define action called checksum file with parameters filename:
    try:
        open file at filename for reading as file
        wait for store content as read content from file
        close file file

        store hash as wflhash256 of content
        return hash
    catch:
        return nothing
    end try
end action

// Create checksum
store original_hash as checksum file with "important.txt"
display "Original checksum: " with original_hash

// Later, verify file hasn't changed
store current_hash as checksum file with "important.txt"

check if original_hash is equal to current_hash:
    display "✓ File is unchanged"
otherwise:
    display "⚠️ File has been modified!"
end check
```

### API Request Signing

```wfl
define action called sign request with parameters data and api_key:
    store timestamp as current time in milliseconds
    store payload as data with "|" with timestamp
    store signature as wflmac256 of payload and api_key
    return signature
end action

store api_data as "action=transfer&amount=100"
store api_key as "secret_api_key_xyz"

store signature as sign request with api_data and api_key
display "Request signature: " with signature
```

### Data Deduplication

```wfl
create list seen_hashes
end list

create list unique_items
end list

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

❌ **Don't use for passwords in production:** Use bcrypt, argon2, or scrypt

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

✅ **wflhash256** - 256-bit hashing
✅ **wflhash512** - 512-bit hashing
✅ **wflhash256_with_salt** - Salted hashing
✅ **wflmac256** - Message authentication codes
✅ **Use cases** - Checksums, integrity, deduplication, signing
✅ **Limitations** - Not audited, use alternatives for production security
✅ **Best practices** - Salting, key management, appropriate use cases

## Next Steps

**[Pattern Module →](pattern-module.md)**
Pattern matching utilities.

Or return to:
**[Standard Library Index →](index.md)**

---

**Previous:** [← Random Module](random-module.md) | **Next:** [Pattern Module →](pattern-module.md)
