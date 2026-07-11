# Best Practices

Write better WFL code with proven practices for style, security, performance, testing, and collaboration.

These guides follow the [WFL foundation](../wfl-foundation.md): natural-language readability, clear errors, secure defaults, gradual learning, and habits that still work at production scale (**no-unlearning**).
## What You'll Learn

1. **[Code Style Guide](code-style-guide.md)** - Formatting and conventions
2. **[Naming Conventions](naming-conventions.md)** - Clear, descriptive names
3. **[Error Handling Patterns](error-handling-patterns.md)** - Robust error handling
4. **[Security Guidelines](security-guidelines.md)** - Secure coding practices
5. **[Performance Tips](performance-tips.md)** - Optimization strategies
6. **[Testing Strategies](testing-strategies.md)** - Quality assurance
7. **[Project Organization](project-organization.md)** - Structuring applications
8. **[Collaboration Guide](collaboration-guide.md)** - Team development

## Why Best Practices Matter

Good code is:
- **Readable** - Others (and future you) can understand it
- **Maintainable** - Easy to modify and extend
- **Reliable** - Works correctly and handles errors
- **Secure** - Protects against vulnerabilities
- **Performant** - Runs efficiently

Best practices help you write good code consistently.

## Quick Reference

**Code Style** ([full guide](code-style-guide.md)):
- 4-space indentation
- Max 100 characters per line
- Max 5 nesting levels
- Lowercase keywords
- No trailing whitespace

**Naming** ([full guide](naming-conventions.md)):
- snake_case (project default): `user_name`, `total_count`
- Actions: verb phrases (`validate_email`)
- Containers: PascalCase (`ShoppingCart`); methods snake_case
- Booleans: `is_`, `has_`, `can_`
- Collections: plural (`users`, `error_messages`)
- Avoid reserved keywords: `is_active` not `is`

**Error Handling:**
- Use try-catch for risky operations
- Always close resources (files, connections)
- Provide context in error messages

**Security:**
- Validate all input
- Sanitize subprocess commands
- Check file paths
- Multi-hash sensitive data (passwords especially); treat WFLHASH as experimental

**Performance:**
- Use async for I/O operations
- Cache expensive calculations
- Choose right data structures

**Testing:**
- Write tests first (TDD)
- Test edge cases
- Test error conditions

**Organization:**
- One responsibility per action
- Group related code
- Use configuration files

**Collaboration:**
- Clear commit messages
- Document complex code
- Maintain backward compatibility

## WFL-Specific Best Practices

### Use Natural Language

**Good:**
```wfl
store customer_balance as 500
store minimum_balance as 100

check if customer_balance is greater than minimum_balance:
    display "Transaction approved"
end check
```

**Poor:**
```wfl
// Cryptic abbreviations — hard to read at a glance
store cb as 500
store mb as 100

check if cb is greater than mb:
    display "ok"
end check
```

WFL's natural syntax is its strength—use it!

### Avoid Reserved Keywords

**Wrong:**
```wfl
// These FAIL to parse — 'is' and 'file' are reserved keywords:
// store is as yes
// store file as "data.txt"
```

**Right:**
```wfl
store is_valid as yes
store filename as "data.txt"
```

**[See complete list →](../03-language-basics/variables-and-types.md#reserved-keywords)**

### Always Validate Examples

When writing documentation or sharing code:

1. Test with MCP tools
2. Run the program
3. Verify output
4. Check edge cases

## Start Here

New to best practices? Begin with:

**[Code Style Guide →](code-style-guide.md)**
Learn WFL formatting conventions.

Or jump to what you need:
- Security? → [Security Guidelines](security-guidelines.md)
- Performance? → [Performance Tips](performance-tips.md)
- Testing? → [Testing Strategies](testing-strategies.md)

---

**Previous:** [← Typechecker Module](../05-standard-library/typechecker-module.md) | **Next:** [Code Style Guide →](code-style-guide.md)
