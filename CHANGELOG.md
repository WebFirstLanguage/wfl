# Changelog

All notable changes to the WFL project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project uses a calendar-based versioning scheme: **YY.MM.BUILD**.

## [Unreleased]

### Security
- **MCP file resources are restricted to bounded WFL sources inside the configured
  workspace.** `resources/read` now canonicalizes file URIs, rejects traversal and
  symlink escapes (including `.wflcfg`), caps returned source at 4 MiB, and no
  longer echoes request or response bodies into diagnostic logs.
- Unsupported database URL errors no longer echo the full connection URL,
  preventing embedded credentials from being disclosed in diagnostics.
- **Cyclic values no longer abort the interpreter during display, diagnostics,
  or isolated-module cloning.** List/object formatting now detects cycles and
  caps nesting depth, while deep clones preserve cycles and shared references
  inside the cloned graph.
- **Subprocess policy is enforced on every process launch** (shell path and
  direct-exec / `with arguments` path). Previously, `shell_execution_mode` and
  related checks ran only when the engine believed a shell was required, so
  forms such as `execute command "sh" with arguments ["-c", "..."]` bypassed
  the default `forbidden` policy.
- **`allow_shell_execution` is now enforced** as a master switch: when `false`
  (the default), all `execute command` / `spawn command` launches are blocked.
- **Secure defaults deny all external processes.** To opt in for local tooling:

  ```ini
  allow_shell_execution = true
  shell_execution_mode = sanitized
  # or: shell_execution_mode = allowlist_only
  #     allowed_shell_commands = echo, ls, git
  ```

- README and configuration docs updated to describe the real policy (subprocess
  execution disabled by default; not a free “sandboxed” escape hatch).

### Added
- HTTP request query string access: raw query is available as `query` / `query of req` (no leading `?`) so `parse_query_string of query` works on real requests (#597)
- Configurable request body size limit via `.wflcfg` `web_server_max_body_size` (default still 1 MiB) (#597)
- `parse_multipart of <body> and <content_type>` returns a list of part objects (`name`, `filename`, `content_type`, `content`, `content_bytes`) for multipart form uploads (#597)
- HTTPS support for the built-in web server: `listen on port 8443 secured with certificate "cert.pem" and key "key.pem" as server` (PEM files; paths may be expressions)
- Bare `secured` form takes certificate/key paths from new `.wflcfg` settings `web_server_tls_cert_file` / `web_server_tls_key_file`; in-language paths always win, and a plain `listen` never becomes HTTPS via config
- HTTP→HTTPS auto-redirect servers: `listen on port 8080 redirecting to port 8443 as redirect_server` answers every request natively with `301 Moved Permanently`, preserving host, path, and query (target port omitted when 443)
- Certificate/key files are validated at `listen` time; missing or malformed files are reported with the offending path and a hint for generating dev certificates
- `secured`, `certificate`, `key`, and `redirecting` are positional marker words, not reserved keywords — existing programs using them as variable names are unaffected
- HTTPS section in `Docs/04-advanced-features/web-servers.md` (dev-cert generation, redirect and dual HTTP+HTTPS patterns, production notes); new settings documented in `Docs/reference/configuration-reference.md`
- New `execute file` statement for running WFL files in-process: `execute [wfl] file at <path> [with <request>] [and read output as <variable>]`
- Web servers can now serve dynamic WFL pages PHP-style: execute a `.wfl` file per request, pass it the HTTP request context (`method`, `path`, `client_ip`, `body`, `headers`), capture its display output, and respond with it
- Output capture mechanism (`display`/`print` redirection) for nested interpreter runs, with correct nesting semantics
- Request objects from `wait for request` now carry `method`, `path`, `client_ip`, `body` and `headers` properties (in addition to the existing standalone variables)
- Errors in executed files (missing file, parse errors, runtime errors) are catchable in the parent with `try`/`when`, including `when file not found`
- Nesting depth guard (4 levels) protects against a file that executes itself
- Built-in database support for SQLite, PostgreSQL, and MariaDB/MySQL backed by sqlx connection pooling:
  - `open database at "<url>" as db` (alias: `connect to database at ... as ...`) routed by URL scheme (`sqlite://`, `sqlite::memory:`, `postgres://`, `postgresql://`, `mysql://`, `mariadb://`)
  - `store rows as query db with "<sql>" [and parameters [...]]` returns a list of row objects keyed by column name
  - `store result as execute db with "<sql>" [and parameters [...]]` returns `{affected_rows, last_insert_id}` (`last_insert_id` is `nothing` on PostgreSQL — use `RETURNING`)
  - `close database db`
  - Parameters always bind through the database driver (never string interpolation), so SQL injection via values is not possible; placeholders are driver-native (`?` for SQLite/MariaDB, `$1` for PostgreSQL)
  - Type-aware decoding: integers/floats/decimals → number, `NULL` → `nothing`, `BOOLEAN` → boolean, `BLOB`/`BYTEA` → binary, `DATE`/`TIME`/`TIMESTAMP` → date/time/datetime
  - Database errors are catchable with `try`/`when error`
  - Note: `store <name> as query <handle> with ...` and `store <name> as execute <handle> with ...` are now reserved statement shapes; a multi-word variable whose name starts with the word `query`, followed by a `with` concatenation, would previously have parsed as an expression
- Web route parameter helpers in the standard library:
  - `path_params of <path> and "<template>"` extracts `:name` segment captures (plus trailing `*name` wildcards) as an object, or returns `nothing` on no match; captures are percent-decoded and query strings are ignored
  - `path_matches of <path> and "<template>"` returns a boolean for routing conditionals
- CI job running the database test suite against live PostgreSQL 16 and MariaDB 11 service containers (`WFL_TEST_POSTGRES_URL` / `WFL_TEST_MYSQL_URL` gate the backend-specific tests)
- New documentation: `Docs/04-advanced-features/databases.md`; route-parameters section in `Docs/04-advanced-features/web-servers.md`

### Fixed
- Outbound HTTP requests no longer fail when they land on a pooled keep-alive
  connection the peer has already closed. reqwest's 90-second idle window was
  left at its default while peers close idle connections far sooner (Node at 5
  seconds, many proxies between 5 and 15), and neither send site could recover,
  so a `POST` written to a dead socket surfaced as
  `Failed to send HTTP POST request: error sending request for url (...)` for a
  request the server never received — most visibly as an intermittent failure
  after a program had been idle between calls. Pooled connections are now
  reused only while idle for under three seconds, and a request that failed
  before any of the response arrived is re-sent once on a fresh connection, on
  both the `read response`/`read content` and `stream response` paths. A
  request that reached a response head is never re-sent
- `header "<Name>" of <request>` now reads headers from the request object, so it works inside actions that receive `req` as a parameter (previously looked only at loop-scoped `headers` and failed with "no request in scope") (#597)
- `respond to ... with <content> and status <code> and content_type <type>` previously parsed the status as the boolean expression `<code> and content_type`, which failed at runtime and left the HTTP request unanswered; status/content_type values now parse as primary expressions
- `header "<Name>" of <request>` is now case-insensitive; warp normalizes header names to lowercase, so canonically-spelled names like `User-Agent` always returned nothing on real requests. Absent headers now compare equal to `nothing`
- The static analyzer now marks variables inside list literals (e.g. `parameters [user_name]`) as used
- `scripts/run_web_tests.sh` exited before running any test due to `set -e` combined with `((var++))` arithmetic increments
- `wfl -h` and `wfl -V` now print help and version. Both were already treated as
  trivial, non-interpreting invocations internally, but the argument parser only
  recognized `--help`, `--version`, and `-v`, so the short aliases fell through
  and were mistaken for an input file path, failing with
  `No such file or directory`

### Removed
- **The `wflpkg` package manager has been removed in its entirety.** The
  `crates/wflpkg` crate and its standalone `wflpkg` binary are gone, along with
  everything they reached into WFL:
  - The positional `wfl` subcommands `create`, `add`, `remove`, `update`,
    `build`, `run`, `share`, `search`, `info`, `login`, `logout`, and `check`,
    and the `PACKAGE MANAGEMENT` section of `wfl --help`.
  - The `package:` import protocol. `load module from "package:my-lib"` no
    longer resolves through a `packages/` directory; the string is now an
    ordinary relative path and fails as one.
  - The `project.wfl` / `project.lock` / `workspace.wfl` manifest formats, the
    `.wflpkg` archive format, the `wflhash:v2:` package-integrity transcript,
    the download cache, and the `wflhub.org` registry client and credential
    store. The corresponding `[Unreleased] Security` entries have been dropped,
    since they described code that never shipped a release.
  - **Impact on the WFL language: none**, with one exception. The file-based
    module system — `load module from "path.wfl"`, `include from "path.wfl"`,
    and `export` — is untouched and fully supported. Only the `package:` prefix
    is withdrawn, and no released WFL program could depend on it in practice:
    resolving it required an installed `packages/` tree that only the removed
    `wfl add` could produce.
  - **Impact on tooling:** `wfl run <file.wfl>` and `wfl test <file.wfl>` were
    handled inside the same subcommand dispatch and go with it. Use the
    documented spellings `wfl <file.wfl>` and `wfl --test <file.wfl>`; neither
    alias was ever listed in `wfl --help` or in `Docs/`.
  - **Rationale:** the package manager is being redesigned from scratch. Its
    supply-chain and trust-root decisions are the hardest in the project to walk
    back once published, so it was withdrawn before the first release candidate
    rather than shipped and then revised. The design documents are archived,
    unimplemented, under `Docs/Archive/wflpkg/`.
  - **Governance (`GOVERNANCE.md` §2.2, §8):** package and registry design is a
    Maintainer-only decision area. The Maintainer directed this removal and
    accepted the immediate withdrawal. Recorded here so the decision is
    auditable rather than implicit.
- **The WFL to JavaScript transpiler has been sunset.** The `wfl --transpile`
  command and its `--target`, `--no-runtime`, and `--es-modules` options are gone,
  along with the `wfl::transpiler` library module (`JavaScriptTranspiler`,
  `TranspilerConfig`, `TranspilerTarget`, `transpile`, `transpile_default`). The
  transpiler only ever covered a shrinking subset of the language — it rejected
  web servers, WebSockets, response streaming, and TLS listens outright — so it
  could not keep pace with the interpreter and gave a misleading impression of
  JavaScript output fidelity.
  - **Impact:** no change to the WFL language itself. Every existing WFL program
    still runs unchanged under the interpreter (`wfl <file.wfl>`); only the
    JavaScript output path is affected.
  - **Migration:** run programs directly with the WFL interpreter. The retired
    flags now exit with code 2 and an explicit message rather than being
    misparsed as an input file path, so existing build scripts fail loudly
    instead of silently doing the wrong thing.
  - **Governance (`GOVERNANCE.md` §3.1):** the ≥ 1-year deprecation window applies
    to breaking *existing WFL programs*, and no WFL program is affected — the
    language, its semantics, and every program in `TestPrograms/` are unchanged.
    What is withdrawn is a build-tooling surface (the `--transpile` CLI mode) and
    a library module. Per §2.2, breaking-change decisions rest with the
    Maintainer, who directed this sunset and accepted the immediate removal
    rather than a deferred one. Recorded here so the decision is auditable rather
    than implicit.

## [25.9.1] - 2025-09-20

### Added
- Comprehensive documentation consolidation and optimization
- New consolidated development guide for AI assistants
- Enhanced README with GitHub-optimized navigation
- Table of contents and collapsible sections for better browsing
- Improved cross-linking between documentation files

### Changed
- Updated version scheme documentation with current examples
- Reorganized documentation structure for better GitHub navigation
- Consolidated AI assistant instructions into single comprehensive guide
- Enhanced project status display with collapsible details

### Removed
- Redundant AGENTS.md and CLAUDE.md files (consolidated into .augment/rules/DEVELOPMENT.md)
- Outdated version references throughout documentation

### Fixed
- Version consistency across all documentation files
- Broken or outdated links in documentation
- Documentation navigation structure

## [25.8.11] - 2025-08-12

### Added
- Enhanced bracket array indexing support
- Comprehensive pattern matching with natural language syntax
- Improved error reporting with source context
- Advanced async/await functionality
- Container system for object-oriented programming

### Fixed
- Fixed bracket array indexing parsing issues
- Improved memory management in parser
- Enhanced error recovery in lexer
- Fixed static analyzer variable usage detection

## [25.5.30] - 2025-05-30

### Added
- Configuration validation & auto-fix flags (`--configCheck` and `--configFix`)
- Enhanced SDK integration and bug reporting system
- Improved development tooling and debugging capabilities

### Fixed
- Fixed memory leak in closures with weak references to parent environments
- Improved file I/O with append-mode operations instead of read-modify-write
- Optimized parser memory allocations to reduce heap churn
- Fixed static analyzer incorrectly flagging variables as unused in action definitions

## [25.4.20] - 2025-04-20

### Added
- Nightly build and installer pipeline for Windows, Linux, and macOS
- Automated installers: MSI for Windows, tar.gz/deb for Linux, pkg for macOS
- Skip-if-unchanged logic to avoid unnecessary builds
- Default configuration files included in installers
- Documentation for building and releasing WFL

### Changed
- Updated build system to support cross-platform compilation
- Updated documentation to clarify sequential wait-for behavior

### Fixed
- Fixed memory leak in closures by using weak references for captured environments
- Improved debug report to return a Result and show appropriate error messages
- Hardened `.clear` REPL command against stdout failures

## Version Scheme

WFL uses a calendar-based version scheme: **YY.MM.BUILD**

- **YY**: Two-digit year (e.g., 25 for 2025)
- **MM**: Month number (1-12)
- **BUILD**: Build number within the month (resets each month)

Example: `25.9.1` means Year 2025, September, Build 1

This format ensures compatibility with Windows MSI installers while providing clear release date information.
