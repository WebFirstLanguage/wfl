# Security Guidelines

Security must be built into your WFL applications from the start. This guide covers essential security practices.

## Input Validation

**Never trust user input.** Always validate.

### Subprocess Security (Critical!)

**Dangerous:**
```wfl
store user_input as get_input()
wait for execute command "echo " with user_input  // UNSAFE!
// User could input: "; rm -rf /"
```

**Safe:**
```wfl
define action called safe_echo with parameters text:
    // Validate: no special characters
    check if contains ";" in text or contains "|" in text or contains "&" in text:
        display "Error: Invalid characters"
        return no
    end check

    wait for execute command "echo " with text
    return yes
end action
```

**Best:** Use whitelist of allowed commands, never construct commands from user input.

### File Path Security

**Prevent directory traversal:**

```wfl
define action called safe_read_file with parameters filename:
    // Check for directory traversal
    check if contains ".." in filename or contains "/" in filename:
        display "Error: Invalid file path"
        return nothing
    end check

    // Only allow files in specific directory
    store safe_path as "safe_directory/" with filename

    try:
        open file at safe_path for reading as myfile
        wait for store content as read content from myfile
        close file myfile
        return content
    catch:
        return nothing
    end try
end action
```

## WFLHASH Usage

**WFLHASH is NOT externally audited.**

### Appropriate Uses

✅ **Internal apps** - Controlled security requirements
✅ **Checksums** - File integrity verification
✅ **Caching keys** - Non-sensitive data
✅ **Development** - Testing environments

### Inappropriate Uses

❌ **Password hashing** - Use bcrypt, argon2, scrypt
❌ **FIPS compliance** - Use SHA-256, SHA-3, BLAKE3
❌ **High security** - Use proven algorithms
❌ **Production auth** - Use established standards

**[Complete crypto guidelines →](../05-standard-library/crypto-module.md)**

### Use Salts

**Always salt user-specific data:**

```wfl
define action called hash_user_data with parameters data and user_id:
    return wflhash256_with_salt of data and user_id
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
open file at "secrets.config" for reading as secretfile
wait for store api_key as read content from secretfile
close file secretfile

// Or better: use environment variables
// store api_key as get_env("API_KEY")
```

### Don't Log Secrets

```wfl
// Wrong:
display "API Key: " with api_key  // Don't log secrets!

// Right:
display "API Key: ****" with substring of api_key from length of api_key minus 4 length 4
// Only show last 4 characters
```

## Web Server Security

### Validate All Paths

```wfl
wait for request comes in on server as req

// Validate path
check if contains ".." in path:
    respond to req with "Invalid path" and status 400
    // Stop - don't process
end check
```

### Set Body Limits

```wfl
// Prevent large request attacks
check if length of request_body is greater than 1000000:  // 1MB limit
    respond to req with "Request too large" and status 413
end check
```

### Use Proper Status Codes

```wfl
// Don't leak internal errors
try:
    store data as query_database()
    respond to req with data
catch:
    // Don't reveal: "Database connection failed at 192.168.1.5:5432"
    respond to req with "Internal server error" and status 500
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
✅ **Use appropriate crypto** - WFLHASH for non-critical only
✅ **Salt user data** - Prevent rainbow tables
✅ **Never log secrets** - API keys, passwords, tokens
✅ **Use proper status codes** - Don't leak internal details
✅ **Close resources** - Files, connections
✅ **Review subprocess calls** - Never trust user input

❌ **Don't trust user input** - Ever
❌ **Don't hardcode secrets** - Use config files or env vars
❌ **Don't execute arbitrary commands** - Whitelist only
❌ **Don't use WFLHASH for passwords** - Use proper algorithms
❌ **Don't reveal internal errors** - Generic messages for users

## What You've Learned

✅ Input validation is critical
✅ Subprocess security (command injection)
✅ File path validation (directory traversal)
✅ WFLHASH appropriate usage
✅ Secrets management
✅ Web server security
✅ Memory security
✅ Error disclosure

**Next:** [Performance Tips →](performance-tips.md)

---

**Previous:** [← Error Handling Patterns](error-handling-patterns.md) | **Next:** [Performance Tips →](performance-tips.md)
