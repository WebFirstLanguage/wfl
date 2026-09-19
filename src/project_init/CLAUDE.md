# WFL project guide for agents

This is a WFL application project. Read the existing source, tests, and project
documentation before changing behavior. Add this project's entry points, test
commands, and conventions to this file as they become established.

## Start here

- Check the installed runtime with `wfl --version` and available commands with
  `wfl --help`. Check the language server separately with `wfl-lsp --version`.
- Install WFL and, for editor/agent integration, `wfl-lsp` using the
  [installation guide](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/02-getting-started/installation.md).
  Use runtime and language-server artifacts from the same release; the LSP has
  its own version number, so the two version strings need not be equal.
- These instructions use installed executables. Developing a WFL application
  does not require a checkout of the WFL compiler or a Rust toolchain.
- The linked `main` documentation can describe a newer build than the installed
  runtime. Check the matching release/tag documentation when behavior differs,
  and validate syntax with the runtime that this project actually uses.

## How WFL works

WFL (WebFirst Language) uses readable, English-like statements in `.wfl` files.
Normal execution lexes source into tokens, parses an AST, performs semantic
analysis and type checking, then interprets the program. Read diagnostics even
when execution succeeds: some type diagnostics are warnings. `wfl --analyze`
performs static analysis without executing the program; it is not a substitute
for runtime tests.

Blocks use a colon and an explicit ending such as `end check`, `end for`, or
`end action`. Comments start with `//`. Use `store` to declare a variable and
`change` to update it. Values include numbers, quoted text, `yes`/`no`,
`nothing`, and lists. Prefer descriptive names with underscores to avoid
reserved words; consult the keyword reference before inventing syntax.

This standalone example can be saved as `main.wfl` and run with `wfl main.wfl`:

```wfl
store greeting as "Hello, WFL!"
display greeting

store visit_count as 1
change visit_count to visit_count plus 1
check if visit_count is greater than 1:
    display "Welcome back"
otherwise:
    display "Welcome"
end check

store names as ["Ada", "Linus"]
push with names and "Grace"
for each person in names:
    display "Hello, " with person
end for

define action called double_value with parameters amount:
    return amount times 2
end action

store doubled as double_value of 4
display doubled
```

`with` joins text in expressions; a value-returning action call uses `of`, as
above. A statement-style action call uses `call action_name with argument`.
Use `push with my_list and value` to append to a list. For an additional
condition, nest another `check if` inside an `otherwise:` block and close both
checks. Count loops use `count from 1 to 3:` / `end count`, with `count` as the
loop variable. Find exact module, async, container, pattern, database, and web
syntax through the documentation map below; do not translate another language
word-for-word.

## Run, check, and test

The paths below are examples; use this project's real entry point and test
files. Put WFL options before the source path; arguments after it belong to the
script. Lint is an exception: its options can appear before or after the source
path.

```sh
wfl main.wfl
wfl --analyze main.wfl
wfl --lint main.wfl
wfl --lint --fix --diff main.wfl
wfl --test tests/example.test.wfl
wfl --configCheck
```

Plain `wfl` opens the REPL. Lint fixes with `--diff` preview changes; use
`--in-place` instead to apply fixes. Run plain lint afterward to find remaining
warnings. Lint exits with 0 for clean source, 1 for warnings, and 2 for invalid
input/options. `wfl --configFix` repairs discovered configuration files, which
can include global configuration; review its scope before using it. `wfl config`
starts the interactive global configuration wizard.

The project configuration file is named exactly `.wflcfg`. It uses `key = value`
lines, not WFL syntax. WFL merges global defaults with the closest `.wflcfg`
found while walking upward from the script's directory. More distant local
files are not merged. Keep application data and secrets separate from this
runtime/lint configuration. See the configuration reference for supported keys.

Create `tests/example.test.wfl` with this runnable test:

```wfl
describe "Arithmetic":
    test "adds two numbers":
        expect 2 plus 3 to equal 5
    end test
end describe
```

Run test files with `wfl --test`; failed assertions return a nonzero exit code.
Use assertions for outcomes and reproduce bugs with a failing test before
fixing them. Check changed programs with the real runtime, and run the
project's test suite before reporting success.

## Editor LSP and agent MCP

`wfl-lsp` provides two different integrations:

- **Editor LSP:** configure the editor's language client to launch `wfl-lsp`
  with `--stdio` (stdio is also the default) for `.wfl` files. This provides
  diagnostics, completion, and hover information. The editor launches the
  server and communicates over stdin/stdout.
- **Agent MCP:** configure the agent's MCP client to launch `wfl-lsp` with
  `--mcp`, using this project as its working directory. Use an absolute
  executable path if the client cannot find it on PATH. MCP and LSP use
  different protocols; choose the mode for the client connecting to it.

For the WFL VS Code extension, these settings select an installed server:

```json
{
  "wfl.serverPath": "wfl-lsp",
  "wfl.serverArgs": ["--stdio"]
}
```

Open a `.wfl` file after installing the extension. See its **WFL** output
channel for startup problems, and use the editor setup guide
for installation and other editors.

When MCP is connected, use `parse_wfl`, `analyze_wfl`, `typecheck_wfl`, and
`lint_wfl` to check proposed source. Each accepts a `source` string containing
WFL code; `get_completions` and `get_symbol_info` provide additional help.
These tools check code rather than running application tests. If MCP is not
available, use the CLI checks and tests above. The MCP integration guide below
includes client configuration examples.

## Docker testing

The official [Docker testing guide](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/guides/docker-testing.md)
explains using `bsbyrdwfl/wfl:nightly` to run an application's tests without a
local WFL install. The image targets Linux x86-64 (`linux/amd64`); Docker Desktop
must use Linux containers. It runs WFL in `/work`. Mount this project there and
pass an existing test file:

Linux shell:

```bash
docker run --rm --pull=always --user "$(id -u):$(id -g)" --mount "type=bind,src=$PWD,dst=/work" bsbyrdwfl/wfl:nightly --test tests/example.test.wfl
```

PowerShell:

```powershell
docker run --rm --pull=always --mount "type=bind,src=$($PWD.Path),dst=/work" bsbyrdwfl/wfl:nightly --test tests/example.test.wfl
if ($LASTEXITCODE -ne 0) { throw "WFL tests failed" }
```

Omit `--test` to run an ordinary script. Docker propagates WFL's failing exit
status. The bind mount is writable; tests should write only to intended output
locations. Inspect the runtime using
`docker run --rm bsbyrdwfl/wfl:nightly --version` and record the version used.
`nightly` moves; the guide explains version tags, retention, and mirroring an
image when a permanent historical version is needed. The runtime image does
not include the editor language server.

## Find the authoritative details

Start with the relevant official page, follow its topic links, and validate
examples against this project's installed version:

| Need | Documentation |
|---|---|
| All topics and learning path | [Documentation index](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/README.md) |
| Variables, conditions, loops, lists, actions, errors | [Language basics](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/03-language-basics/index.md) |
| Exact syntax and operators | [Syntax reference](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/reference/syntax-reference.md), [operator reference](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/reference/operator-reference.md) |
| Reserved words and naming conflicts | [Keyword reference](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/reference/keyword-reference.md), [reserved keywords](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/reference/reserved-keywords.md) |
| Built-in functions and modules | [Standard library](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/05-standard-library/index.md), [built-in function reference](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/reference/builtin-functions-reference.md) |
| Modules, async, containers, patterns, web, databases | [Advanced features](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/04-advanced-features/index.md) |
| `.wflcfg` keys and precedence | [Configuration reference](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/reference/configuration-reference.md) |
| Test syntax and assertions | [Testing guide](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/guides/testing-guide.md) |
| Editor and language-server setup | [Editor setup](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/02-getting-started/editor-setup.md) |
| Agent tooling and client configuration | [MCP integration](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/contributing/mcp-integration.md) |
| Project layout and style | [Project organization](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/06-best-practices/project-organization.md), [code style](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/06-best-practices/code-style-guide.md) |
| Working examples | [WFL by example](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/guides/wfl-by-example.md) |
| Errors and troubleshooting | [Error codes](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/reference/error-codes.md), [troubleshooting](https://github.com/WebFirstLanguage/wfl/blob/main/Docs/guides/troubleshooting.md) |

Additional discovery entry point: [WFL on Context7](https://context7.com/webfirstlanguage/wfl).
If your agent has Context7 access, use it to find relevant WFL documentation,
then verify the result against official documentation and the installed tools.
This scaffold does not install or configure Context7, MCP clients, or editors.
