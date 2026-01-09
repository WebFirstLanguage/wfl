# Contributing Guide

Help improve WFL! This guide covers how to contribute code, documentation, tests, and examples.

## Getting Started

1. **Fork** the repository
2. **Clone** your fork
3. **Create branch:** `git checkout -b feature/my-feature`
4. **Make changes**
5. **Test thoroughly**
6. **Submit PR**

## Development Workflow

### 1. Write Tests First (TDD)

WFL requires test-driven development:

```bash
# 1. Write failing test
echo 'display "Test"' > TestPrograms/my_feature_test.wfl

# 2. Verify it fails (expected)
cargo run -- TestPrograms/my_feature_test.wfl

# 3. Implement feature in src/

# 4. Test passes
cargo run -- TestPrograms/my_feature_test.wfl
```

### 2. Run All Quality Checks

```bash
# Format
cargo fmt --all

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Test
cargo test --all --verbose

# Integration tests
cargo build --release
./scripts/run_integration_tests.ps1  # Windows
./scripts/run_integration_tests.sh   # Linux/macOS
```

### 3. Update Documentation

**If adding feature:**
- Add to appropriate Docs/ section
- **Validate ALL code examples with MCP tools**
- Add to TestPrograms/docs_examples/
- Update manifest.json

**Validation required:**
```bash
python scripts/validate_docs_examples.py --file path/to/example.wfl
```

### 4. Create PR

**PR should include:**
- Clear description of changes
- Tests for new features/fixes
- Documentation updates
- All quality checks passing

## Commit Messages

Use conventional commits:

```
feat: Add new feature
fix: Fix bug
docs: Update documentation
test: Add tests
refactor: Refactor code
perf: Performance improvement
chore: Maintenance
```

**Example:**
```
feat: Add wflhash512 function

Implements 512-bit variant of WFLHASH.
Includes comprehensive tests and documentation.

Closes #123
```

## Code Guidelines

✅ **Follow Rust style** - cargo fmt
✅ **Pass clippy** - No warnings
✅ **Write tests** - TDD mandatory
✅ **Document code** - Inline comments for complex logic
✅ **Maintain compatibility** - Never break existing WFL programs
✅ **Add to TestPrograms/** - End-to-end test for features

**[Coding style →](../06-best-practices/code-style-guide.md)**

## Areas to Contribute

### Code

- New standard library functions
- Performance improvements
- Bug fixes
- Error message improvements
- LSP features

### Documentation

- Fix typos
- Add examples
- Improve explanations
- Create tutorials
- **Validate all examples with MCP!**

### Tests

- Add TestPrograms examples
- Improve test coverage
- Add edge case tests
- Benchmark performance

### Examples

- Real-world programs
- Tutorial code
- Code snippets

## Backward Compatibility

**Sacred rule:** Never break existing WFL code.

Before making changes:
1. Ask: "Will this break existing programs?"
2. If yes: Can it be additive instead?
3. If must break: Deprecation period (1+ year)

## PR Checklist

Before submitting:

```markdown
- [ ] Tests added/updated
- [ ] All tests pass
- [ ] cargo fmt run
- [ ] cargo clippy clean
- [ ] Documentation updated
- [ ] Examples validated with MCP
- [ ] TestPrograms all pass
- [ ] Backward compatible
- [ ] Clear commit messages
```

## Getting Help

**Questions?**
- GitHub Discussions
- Email: info@logbie.com
- Open draft PR for feedback

**Thank you for contributing to WFL!** 🎉

---

**Previous:** [← Building from Source](building-from-source.md) | **Next:** [Architecture Overview →](architecture-overview.md)
