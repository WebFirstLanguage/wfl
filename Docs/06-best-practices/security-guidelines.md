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

    // Pass user input as an argument, never concatenated into the command string
    wait for execute command "echo" with arguments [user_text] as command_result
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

    try:
        open file at safe_path for reading as myfile
        store file_content as read content from myfile
        close file myfile
        return file_content
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
define action called hash_user_data with parameters user_data and user_id:
    return wflhash256_with_salt of user_data and user_id
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
