# Best Practices

Write better WFL code with proven practices for style, security, performance, testing, and collaboration.

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

**Code Style:**
- 4-space indentation
- Max 100 characters per line
- Max 5 nesting levels
- Lowercase keywords

**Naming:**
- snake_case: `user_name`, `total_count`
- Descriptive: `customer_balance` not `cb`
- Avoid reserved keywords: `is_active` not `is active`

**Error Handling:**
- Use try-catch for risky operations
- Always close resources (files, connections)
- Provide context in error messages

**Security:**
- Validate all input
- Sanitize subprocess commands
- Check file paths
- Use WFLHASH appropriately

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
check if customer_balance is greater than minimum_balance:
    approve_transaction for customer
end check
```

**Poor:**
```wfl
check if cb > min:
    approve(c)
end check
```

WFL's natural syntax is its strength—use it!

### Avoid Reserved Keywords

**Wrong:**
```wfl
store is as yes               // 'is' is reserved
store file as "data.txt"      // 'file' is reserved
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
