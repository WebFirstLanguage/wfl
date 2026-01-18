# Troubleshooting Guide

Common problems and solutions for WFL development.

## Installation Issues

### "wfl: command not found"

**Problem:** Shell can't find WFL binary.

**Solution (Windows):**
1. Close and reopen terminal
2. Check PATH: `echo %PATH%`
3. Add WFL manually or reinstall MSI

**Solution (Linux/macOS):**
```bash
export PATH="$PATH:/path/to/wfl/target/release"
echo 'export PATH="$PATH:/path/to/wfl/target/release"' >> ~/.bashrc
source ~/.bashrc
```

---

### "cargo: command not found"

**Problem:** Rust not installed.

**Solution:** Install Rust from https://rustup.rs/

---

### Build Fails: Compilation Errors

**Problem:** Rust version too old.

**Solution:**
```bash
rustup update
cargo clean
cargo build --release
```

Need Rust 1.75+.

---

## Runtime Errors

### "Variable 'x' is not defined"

**Problem:** Using undefined variable.

**Solution:**
```wfl
// Wrong:
display user_name  // Not defined yet

// Right:
store user_name as "Alice"
display user_name
```

---

### "Type mismatch: cannot add Number and Text"

**Problem:** Trying to add incompatible types.

**Solution:**
```wfl
// Wrong:
store age as 25
store name as "Alice"
display age plus name  // Error!

// Right:
display "Name: " with name with ", Age: " with age
```

---

### "Expected identifier, found Keyword..."

**Problem:** Using reserved keyword as variable name.

**Solution:**
```wfl
// Wrong:
store is as yes        // 'is' is reserved
store file as "data"   // 'file' is reserved

// Right:
store is_valid as yes
store filename as "data"
```

**[Quick keyword lookup →](../reference/keyword-reference.md)** | **[Why can't I use this keyword? →](../reference/reserved-keywords.md#contextual-keywords)**

---

### "Cannot pop from empty list"

**Problem:** Popping from list with no items.

**Solution:**
```wfl
check if length of list is greater than 0:
    store item as pop from list
otherwise:
    display "List is empty"
end check
```

---

## Parser Errors

### "Expected 'end' after if block"

**Problem:** Missing `end check`.

**Solution:**
```wfl
// Wrong:
check if condition:
    code
// Missing end check!

// Right:
check if condition:
    code
end check
```

---

### Parse Error with Conditionals

**Problem:** Using flat `otherwise check if` (doesn't exist).

**Solution:**
```wfl
// Wrong:
check if a:
    code
otherwise check if b:  // This syntax doesn't exist
    code
end check

// Right (use nesting):
check if a:
    code
otherwise:
    check if b:
        code
    end check
end check
```

---

## Editor/LSP Issues

### Syntax Highlighting Not Working

**Problem:** VS Code not recognizing .wfl files.

**Solution:**
```powershell
scripts/install_vscode_extension.ps1
```

Then reload VS Code.

---

### LSP Not Providing Suggestions

**Solution:**
1. Check wfl-lsp installed: `wfl-lsp --version`
2. Enable in settings: `"wfl.enableLSP": true`
3. View Output → "WFL Language Server" for errors
4. Restart VS Code

---

### MCP Not Working with Claude Desktop

**Solution:**
1. Verify config path correct (`%APPDATA%\Claude\claude_desktop_config.json`)
2. Use absolute path for `wfl-lsp` command
3. Verify `cwd` points to project
4. Restart Claude Desktop completely

---

## Integration Test Failures

### "Path Not Found" Error

**Problem:** Integration tests need release binary.

**Solution:**
```bash
cargo build --release
cargo test
```

**Or use scripts:**
```bash
./scripts/run_integration_tests.ps1  # Windows
./scripts/run_integration_tests.sh   # Linux/macOS
```

---

## Performance Issues

### Program Runs Slowly

**Solution:**
1. Check for infinite loops
2. Use `wfl --time file.wfl` to measure
3. Profile with timing
4. Optimize algorithm (see [Performance Tips](../06-best-practices/performance-tips.md))

---

### Memory Usage High

**Solution:**
1. Check for memory leaks (large lists growing)
2. Close files after use
3. Clear large lists when done

---

## Web Server Issues

### Server Not Responding

**Problem:** Can't connect to server.

**Solution:**
1. Check if server actually started (look for error messages)
2. Try different port: `listen on port 8081`
3. Check firewall isn't blocking
4. Verify path handling in routes

---

### "Address already in use"

**Problem:** Port already taken.

**Solution:**
```bash
# Find what's using port 8080:
# Windows: netstat -ano | findstr :8080
# Linux: lsof -i :8080

# Use different port:
listen on port 8081 as server
```

---

## Getting Help

### Documentation

- [Language Basics](../03-language-basics/index.md)
- [Standard Library](../05-standard-library/index.md)
- [FAQ](faq.md)

### Community

- **GitHub Issues:** https://github.com/WebFirstLanguage/wfl/issues
- **Discussions:** https://github.com/WebFirstLanguage/wfl/discussions
- **Email:** info@logbie.com

### Debug Mode

```bash
wfl --debug program.wfl > debug.txt 2>&1
```

Captures detailed execution information.

---

**Previous:** [← Cookbook](cookbook.md) | **Next:** [FAQ →](faq.md)
