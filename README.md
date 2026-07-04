# WFL — the WebFirst Language

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/WebFirstLanguage/wfl)

**Programming that reads like plain English.**

WFL is a natural-language programming language: you write what you want to do in
words, and it runs. It aims to be a genuine *first* language for newcomers while
staying powerful enough for production — with no cliff in between.

```wfl
store name as "World"
display "Hello, " with name with "!"
```

```text
Hello, World!
```

## Why WFL?

- **Reads like English.** `check if score is greater than 90:` — no cryptic
  symbols to memorize.
- **Safe by default.** Static typing with inference, clear Elm-style error
  messages, and secure defaults (output escaping, sandboxed subprocesses).
- **Batteries included.** Text, math, lists, filesystem I/O, crypto, time,
  async/await, pattern matching, containers (objects), an HTTP client, and a
  built-in web server.
- **No unlearning.** The beginner form and the expert form are the same form —
  advanced features extend what you already know rather than replacing it.

See the [19 founding principles](Docs/wfl-foundation.md) for the full philosophy.

## A quick taste

```wfl
define action called area with parameters width and height:
    return width times height
end action

store shapes as [3, 5, 8]
for each side in shapes:
    display "Square " with side with " has area " with area of side and side
end for
```

```text
Square 3 has area 9
Square 5 has area 25
Square 8 has area 64
```

More runnable, output-verified examples live in the
[**Friendly WFL Tour**](TestPrograms/docs_examples/tour/README.md) — eight small
programs from *Hello, World!* to objects, each checked against the current build.

## Install & build

WFL is written in Rust (edition 2024, which requires Rust 1.85 or newer).

```bash
# Build the compiler/runtime
cargo build --release

# Run a program
target/release/wfl path/to/program.wfl

# Start the interactive REPL
target/release/wfl
```

Windows users can install from the MSI in [Releases](https://github.com/WebFirstLanguage/wfl/releases).
Full instructions: [Docs/02-getting-started/installation.md](Docs/02-getting-started/installation.md).

### Handy CLI flags

| Command | What it does |
|---|---|
| `wfl <file>` | Run a program |
| `wfl --lint <file>` | Lint; add `--fix --in-place` to auto-fix |
| `wfl --analyze <file>` | Static analysis |
| `wfl --test <file>` | Run `describe`/`test` blocks |
| `wfl --parse <file>` / `wfl --lex <file>` | Dump the AST / tokens |

## Documentation

The complete guide lives in [**`Docs/`**](Docs/README.md), organized as a
learning path:

1. [Introduction](Docs/01-introduction/index.md) — what WFL is and why
2. [Getting Started](Docs/02-getting-started/index.md) — install and first programs
3. [Language Basics](Docs/03-language-basics/index.md) — variables, control flow, actions
4. [Advanced Features](Docs/04-advanced-features/index.md) — async, containers, web, patterns
5. [Standard Library](Docs/05-standard-library/index.md) — every built-in module
6. [Best Practices](Docs/06-best-practices/index.md) — style, testing, security

Reference: [keywords](Docs/reference/keyword-reference.md) ·
[operators](Docs/reference/operator-reference.md) ·
[built-in functions](Docs/reference/builtin-functions-reference.md) ·
[error codes](Docs/reference/error-codes.md).

### Documentation is tested

Every ` ```wfl ` block in the docs is extracted and — unless it is a placeholder
template, an illustrative fragment, or a long-running server demo — executed
against the release binary by [`scripts/test_docs_code_blocks.py`](scripts/test_docs_code_blocks.py);
where a doc shows an **Output:** block, the script compares real stdout against
it. The current state — including a per-file list of anything that doesn't run —
is tracked in [`TestPrograms/docs_examples/DOC_CODE_AUDIT.md`](TestPrograms/docs_examples/DOC_CODE_AUDIT.md).

```bash
python3 scripts/test_docs_code_blocks.py           # run the whole audit
python3 scripts/test_docs_code_blocks.py --filter 03-language-basics --show-errors
```

## Repository layout

| Path | Contents |
|---|---|
| `src/` | Compiler & runtime (lexer, parser, analyzer, type checker, interpreter) |
| `Docs/` | User documentation (the six sections above + guides & reference) |
| `TestPrograms/` | End-to-end WFL programs (must all pass on the release build) |
| `tests/`, `benches/` | Rust integration tests and Criterion benchmarks |
| `wfl-lsp/`, `vscode-extension/` | Language Server and editor integration |
| `examples/`, `Nexus/` | Example and experimental WFL programs |

## Contributing

Contributions are welcome. Please read
[Docs/development/contributing-guide.md](Docs/development/contributing-guide.md)
and note the project conventions:

- **Test-driven:** write failing tests first (`tests/` for Rust, `TestPrograms/`
  for WFL end-to-end).
- **Backward compatibility is sacred** — existing WFL programs must keep working.
- Before a PR: `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`.

## License

Licensed under the [Apache License 2.0](LICENSE).
