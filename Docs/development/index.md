# Development

Guides for building WFL itself, contributing, and integrating editor and AI tooling.

These pages support people who work **on** WFL (compiler, LSP, docs, tooling). For writing WFL *programs*, start with [Getting Started](../02-getting-started/index.md) and the [Code Style Guide](../06-best-practices/code-style-guide.md).

Development work still follows the [WFL foundation](../wfl-foundation.md): clear, natural-language-friendly design, no cliffs between beginner and expert paths, and high quality bars (tests, docs, compatibility).

## Guides

1. **[Building from Source](building-from-source.md)** — Install Rust, clone, build, test, run
2. **[Contributing Guide](contributing-guide.md)** — How to propose and land changes
3. **[Architecture Overview](architecture-overview.md)** — Compiler pipeline and major components
4. **[LSP Integration](lsp-integration.md)** — Language Server for editors
5. **[MCP Integration](mcp-integration.md)** — AI assistant tools (parse, analyze, typecheck, lint)
6. **[Compiler Internals](compiler-internals.md)** — Deeper implementation notes

## Design notes

- **[Route construct design](route-construct-design.md)** — HTTP routing design history
- **[Stdlib higher-order functions](stdlib-higher-order-functions.md)** — Design notes for list transforms

## Quality gates

Before merging compiler or language changes:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --verbose
cargo build --release   # required for many integration / TestPrograms runs
```

See root `CONTRIBUTING.md`, `GOVERNANCE.md`, and `AI_POLICY.md` for community and review rules.

## Related user docs

- [Editor setup](../02-getting-started/editor-setup.md)
- [Configuration reference](../reference/configuration-reference.md)
- [Best practices](../06-best-practices/index.md)

---

**Previous:** [← Language Specification](../reference/language-specification.md) | **Next:** [Building from Source →](building-from-source.md)
