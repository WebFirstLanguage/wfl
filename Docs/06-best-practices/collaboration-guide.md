# Collaboration Guide

Working with others on WFL projects requires clear communication and consistent practices. This guide covers team development workflows.

## Code Reviews

### What to Look For

**Correctness:**
- Does code do what it claims?
- Are edge cases handled?
- Are errors caught?

**Readability:**
- Are names descriptive?
- Is logic clear?
- Are comments helpful?

**Style:**
- Follows .wflcfg?
- Consistent formatting?
- Proper indentation?

**Tests:**
- Are tests included?
- Do tests pass?
- Coverage adequate?

**Backward Compatibility:**
- Does it break existing code?
- Is deprecation warranted?

### Review Checklist

```markdown
- [ ] Code works correctly
- [ ] Tests included and passing
- [ ] Error handling present
- [ ] Names are descriptive
- [ ] No hardcoded secrets
- [ ] Style follows .wflcfg
- [ ] Documentation updated
- [ ] Backward compatible
```

## Pull Requests

### PR Description Template

```markdown
## Summary
Brief description of changes

## Motivation
Why this change is needed

## Changes
- Added feature X
- Fixed bug Y
- Updated documentation Z

## Testing
- Created test_feature.wfl
- All tests pass
- Tested manually with...

## Backward Compatibility
- [x] No breaking changes
- [ ] Breaking change (explain below)

## Checklist
- [x] Tests added
- [x] Documentation updated
- [x] cargo fmt run
- [x] cargo clippy clean
- [x] All TestPrograms pass
```

### Before Submitting PR

```bash
# Format code
cargo fmt --all

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Test
cargo test --all --verbose

# Run all TestPrograms
./scripts/run_integration_tests.ps1  # Windows
./scripts/run_integration_tests.sh   # Linux/macOS
```

## Commit Messages

### Conventional Commits

**Format:** `<type>: <description>`

**Types:**
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `test:` - Adding/updating tests
- `refactor:` - Code restructuring
- `perf:` - Performance improvements
- `chore:` - Maintenance tasks

**Examples:**
```
feat: Add wflhash512 function
fix: Correct division by zero handling
docs: Update pattern matching guide
test: Add comprehensive file I/O tests
refactor: Extract validation to separate action
```

### Good Commit Messages

**Good:**
```
feat: Add email validation pattern

Adds natural language email pattern with comprehensive tests.
Includes examples in documentation.

Closes #123
```

**Poor:**
```
update stuff
fix
changes
```

## Backward Compatibility

**Sacred rule: Never break existing WFL programs.**

### Before Making Changes

Ask:
1. Will this break existing code?
2. Can I make it additive instead?
3. If breaking, is there a deprecation path?

### Deprecation Process

If you MUST deprecate:

1. **Announce** - 1 year notice minimum
2. **Document** - Update CHANGELOG
3. **Provide alternative** - Migration guide
4. **Keep working** - Don't remove until deadline

### Adding Features

**Additive changes are safe:**

```wfl
// Adding new parameter with default (SAFE)
define action called process with parameters data and options:
    // options defaults to empty if not provided
end action
```

**Removing parameters (BREAKING):**

```wfl
// Don't do this without deprecation period!
// define action called process with parameters data:
```

## Communication

### Documentation

**Update docs when you change code:**

- Add examples for new features
- **Validate examples with MCP tools**
- Update references
- Mark deprecated features

### Dev Diary Entries

For significant changes, create Dev Diary entry:

```
Dev diary/
  2026-01-09-added-email-validation.md
```

**Include:**
- What changed
- Why it changed
- How to use it
- Migration guide (if breaking)

## Team Practices

### Consistent Style

Everyone uses same .wflcfg:

```wfl
// All team members format the same way
wfl --fix code.wfl --in-place
```

### Shared Understanding

**Document decisions:**

```wfl
// Decision: Using snake_case for all variables
// Reason: More familiar to Python/Rust developers
// Date: 2026-01-09
```

### Code Ownership

**No gatekeeping** - Anyone can contribute anywhere
**Shared responsibility** - Team owns the code together
**Review together** - Pair reviews for learning

## Best Practices

✅ **Write clear PR descriptions** - Explain what and why
✅ **Use conventional commits** - Consistent history
✅ **Review thoroughly** - Correctness, style, tests
✅ **Maintain compatibility** - Never break existing code
✅ **Update documentation** - Keep docs current
✅ **Run all checks** - fmt, clippy, tests before PR
✅ **Communicate changes** - Dev Diary for big changes

❌ **Don't break backward compatibility** - Sacred rule
❌ **Don't skip tests** - Always include tests
❌ **Don't ignore style** - Run fmt and clippy
❌ **Don't write vague commits** - Be descriptive
❌ **Don't change code without reviewing** - Team review required

## What You've Learned

✅ Code review checklist
✅ Pull request best practices
✅ Conventional commit messages
✅ Backward compatibility importance
✅ Documentation updates
✅ Team communication
✅ Dev Diary usage

---

**Best Practices Complete!**

You've completed the Best Practices section. You now know how to write quality WFL code that's readable, maintainable, secure, performant, and collaborative.

**Next:** Explore practical examples in [Guides →](../guides/)

---

**Previous:** [← Project Organization](project-organization.md) | **Next:** [Guides →](../guides/)
