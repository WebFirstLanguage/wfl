# Security Guidelines

Security must be built into your WFL applications from the start. This guide covers essential security practices.

## Input Validation

**Never trust user input.** Always validate.

### Subprocess Security (Critical!)

**Dangerous:**
```wfl
store user_input as "; rm -rf /"                  // imagine this arrived from a user
store command_text as "echo " with user_input     // UNSAFE: the user controls the command string
display "Would run: " with command_text           // building commands from raw input invites injection
```

**Safe:**
```wfl
define action called safe_echo with parameters user_text:
    // Validate: reject shell metacharacters
    check if user_text contains ";" or user_text contains "|" or user_text contains "&":
        display "Error: Invalid characters"
        return no
    end check

    // Pass user input as an argument, never concatenated into the command string.
    // Note: `execute command` does NOT go through a shell, so this Unix/macOS
    // form runs the standalone "echo" executable. On Windows "echo" is a shell
    // built-in with no executable, so there you would instead run:
    //   execute command "cmd" with arguments ["/c", "echo", user_text]
    wait for execute command "echo" with arguments [user_text]
    return yes
end action
```

**Best:** Use whitelist of allowed commands, never construct commands from user input.

### File Path Security

**Prevent directory traversal:**

```wfl
define action called safe_read_file with parameters filename:
    // Reject any path component that could escape the safe directory
    // ("\\" is the WFL escape for a single backslash, covering Windows separators)
    check if filename contains ".." or filename contains "/" or filename contains "\\":
        display "Error: Invalid file path"
        return nothing
    end check

    // Build the path safely instead of concatenating strings
    store safe_path as path_join of "safe_directory" and filename

    // Keep the handle and result in outer variables so we can close the file
    // AFTER end try and only then return — returning inside the try would skip
    // cleanup, and if the read throws, the handle would otherwise leak.
    store file_handle as nothing
    store succeeded as no
    store file_content as ""
    try:
        open file at safe_path for reading as myfile
        change file_handle to myfile
        change file_content to read content from myfile
        change succeeded to yes
    when error:
        display "Error: Could not read file"
    end try
    check if file_handle is not nothing:
        close file file_handle
    end check
    check if succeeded is yes:
        return file_content
    otherwise:
        return nothing
    end check
end action
```

## Multi-hash for sensitive data

**Recommendation: for sensitive things — passwords especially — always hash with more than one algorithm.** Do not put full trust in a single hash primitive.

| Case | Do this |
| --- | --- |
| Passwords | Multi-hash pre-mix (e.g. WFLHASH then `sha256`), then **always** `hash_password` |
| High-stakes integrity | At least two hashes in series (e.g. WFLHASH then `sha256`) |
| External protocols | Use exactly the algorithm the protocol requires |

Multi-hash is defense in depth. Fast multi-hashes alone are **not** enough for passwords — the final step must be a slow, salted password KDF.

See [Crypto Module → Multi-hash for sensitive data](../05-standard-library/crypto-module.md#multi-hash-for-sensitive-data-recommended).

## Password Hashing

**Never store passwords with only a fast hash (`sha256`, `wflhash256`).** Fast hashes let attackers try billions of guesses per second against a stolen database. Use dedicated password hashing functions (slow, salted, self-describing), and for sensitive accounts prefer a **multi-hash pre-mix** first.

```wfl
// Sign-up: more than one general hash, then password KDF
store step1 as wflhash256 of user_password
store step2 as sha256 of step1
store stored_hash as hash_password of step2

// Login: same pre-mix order, then verify
store attempt_step1 as wflhash256 of attempt
store attempt_step2 as sha256 of attempt_step1
store login_ok as verify_password of attempt_step2 and stored_hash
check if login_ok is yes:
    display "Login successful"
otherwise:
    display "Invalid credentials"
end check
```

`hash_password` uses Argon2id by default and `verify_password` auto-detects the algorithm from the stored hash. If you need a specific algorithm, use `argon2_hash`/`bcrypt_hash`/`scrypt_hash`/`pbkdf2_hash` with their matching `*_verify` functions. See the [Crypto Module → Password Hashing](../05-standard-library/crypto-module.md#password-hashing).

## WFLHASH Usage (experimental)

**WFLHASH is experimental and not externally audited.** We want you to try it — the more real-world testing, the better. When integrity matters in production, **pair it with a known-good hash** so a battle-tested algorithm always backs you up.

### Dual-hash (strong friend) for production

```wfl
// Experimental first pass — exercise WFLHASH
store wfl_digest as wflhash256 of file_content

// Known-good backup (sha256 today; any proven hash works the same way)
store integrity_tag as sha256 of wfl_digest
```

If WFLHASH were ever weaker than expected, the outer standard hash still provides that algorithm's security properties. You get production strength *and* help harden WFLHASH.

### Appropriate Uses

✅ **Testing and feedback** — please use it and report results  
✅ **Production integrity** — when dual-hashed with `sha256` (or another known-good hash)  
✅ **Checksums / cache keys** — dual-hash for production; WFLHASH alone is fine for demos  
✅ **Domain separation** — `wflhash256_with_salt` for different contexts (still dual-hash when stakes are high)

### Inappropriate Uses

❌ **WFLHASH alone as the only integrity guarantee** for high-stakes data  
❌ **Password hashing** — Use `hash_password`/`verify_password` (Argon2id, bcrypt, scrypt, PBKDF2)  
❌ **External protocols** that require a specific standard (`hmac_sha256` for Stripe/GitHub, etc.)  
❌ **FIPS-only / formally validated crypto paths** — use the standard algorithm alone  

**[Complete crypto guidelines →](../05-standard-library/crypto-module.md)**

### Domain separation (with strong friend)

```wfl
define action called hash_user_data with parameters user_data and user_id:
    store wfl_digest as wflhash256_with_salt of user_data and user_id
    return sha256 of wfl_digest
end action
```

## Secrets Management

### Never Hardcode Secrets

**Wrong:**
```wfl
store api_key as "sk_live_abc123..."  // NEVER DO THIS!
store db_password as "secretpassword"
```

**Right:**
```wfl
// Read from environment or config file
try:
    open file at "secrets.config" for reading as secretfile
    store api_key as read content from secretfile
    close file secretfile
catch:
    display "Config file not found - create secrets.config"
end try

// Or better: use environment variables
// store api_key as get_env("API_KEY")
```

### Don't Log Secrets

```wfl
store api_key as "EXAMPLE_not_a_real_key"

// Wrong:
display "API Key: " with api_key  // Don't log secrets!

// Right:
store key_length as length of api_key
store start_index as key_length minus 4
// substring is (text, start, length) — take 4 characters from start_index
store last_four as substring of api_key and start_index and 4
display "API Key: ****" with last_four
// Only show last 4 characters
```

## Web Server Security

### Validate All Paths

```wfl
// Assumes a running web server (see the web server guide)
wait for request comes in on web_server as incoming_request

// Validate path
store request_path as path of incoming_request
check if request_path contains "..":
    respond to incoming_request with "Invalid path" and status 400
    // Stop - don't process
end check
```

### Set Body Limits

```wfl
// Prevent large request attacks
store request_body as body of incoming_request
check if length of request_body is greater than 1000000:  // 1MB limit
    respond to incoming_request with "Request too large" and status 413
end check
```

### Use Proper Status Codes

```wfl
// Don't leak internal errors
try:
    store response_data as query_database of request_path
    respond to incoming_request with response_data
catch:
    // Don't reveal: "Database connection failed at 192.168.1.5:5432"
    respond to incoming_request with "Internal server error" and status 500
    // Log detailed error internally
end try
```

## Memory Security

### Zeroize Sensitive Data

WFL (built on Rust) uses zeroization in crypto module. When handling sensitive data:

- Close files after reading sensitive data
- Don't store passwords in memory longer than needed
- Use MAC verification (constant-time with `subtle` crate)

**Note:** Automatic in WFLHASH functions.

## Best Practices

✅ **Validate all input** - User data, file paths, commands
✅ **Use whitelists** - Allow only known-good values
✅ **Sanitize file paths** - Prevent directory traversal
✅ **Limit request sizes** - Prevent DoS
✅ **Use appropriate crypto** - Multi-hash sensitive data; dual-hash experimental WFLHASH for production integrity; standards for interop
✅ **Multi-hash passwords** - Pre-mix with 2+ algorithms, then always `hash_password`
✅ **Salt / domain-separate** when hashing per-user or per-context data
✅ **Never log secrets** - API keys, passwords, tokens
✅ **Use proper status codes** - Don't leak internal details
✅ **Close resources** - Files, connections
✅ **Review subprocess calls** - Never trust user input

❌ **Don't trust user input** - Ever
❌ **Don't hardcode secrets** - Use config files or env vars
❌ **Don't execute arbitrary commands** - Whitelist only
❌ **Don't store passwords with only fast hashes** - Multi-hash pre-mix is fine; final step must be a password KDF
❌ **Don't rely on a single hash alone** for sensitive data
❌ **Don't rely on experimental WFLHASH alone** for high-stakes integrity
❌ **Don't reveal internal errors** - Generic messages for users

## What You've Learned

✅ Input validation is critical
✅ Subprocess security (command injection)
✅ File path validation (directory traversal)
✅ Multi-hash sensitive data (passwords, high-stakes digests)
✅ WFLHASH is experimental; dual-hash with a known-good algorithm for production
✅ Secrets management
✅ Web server security
✅ Memory security
✅ Error disclosure

**Next:** [Performance Tips →](performance-tips.md)

---

**Previous:** [← Error Handling Patterns](error-handling-patterns.md) | **Next:** [Performance Tips →](performance-tips.md)
