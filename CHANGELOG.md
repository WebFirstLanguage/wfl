# Changelog

All notable changes to the WFL project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project uses a calendar-based versioning scheme: **YY.MM.BUILD**.

## [Unreleased]

### Security
- **WFL package publishing now keeps credentials registry-scoped.** A
  project-controlled `registry` setting can no longer redirect a saved token to
  another origin; registry URLs are canonicalized and must use HTTPS without
  userinfo, paths, queries, or fragments.
- **Package archives are created in private external temporary files** and
  cleaned up automatically. Archive creation refuses existing output paths, so
  a project-supplied symlink can no longer redirect `wfl share` into truncating
  another file.
- **Published packages now honor root and nested `.gitignore` rules.** Ignored
  logs, debug reports, `.env` files, and other local-only content are excluded
  from both the archive and its checksum instead of being uploaded silently.
- **Registry credentials are written atomically with private permissions.** On
  Unix, the auth directory is mode `0700` and the token file is mode `0600`
  before any secret bytes are written.
- **Package integrity checks now use an explicitly versioned
  `wflhash:v2:` transcript.** File records include domain, path, and content
  lengths; paths use portable `/` separators; verification hashes every
  extracted regular file; and publishing derives the digest from the completed
  archive instead of re-reading a mutable source tree.
- **Package publishing now fails closed on unsafe inputs and resource abuse.**
  Manifests and entry points must be in-project regular files, unsupported
  filesystem objects and ambiguous `.gitignore` patterns are rejected, package
  traversal is bounded, archives upload as bounded streams, and registry
  response bodies are capped at 1 MiB.
- **Registry login supports an explicit registry address.** `wfl login
  [registry]` scopes a token to that HTTPS origin, mismatched logins are
  rejected, and `wfl logout` can recover malformed or incomplete credentials.
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
- `header "<Name>" of <request>` now reads headers from the request object, so it works inside actions that receive `req` as a parameter (previously looked only at loop-scoped `headers` and failed with "no request in scope") (#597)
- `respond to ... with <content> and status <code> and content_type <type>` previously parsed the status as the boolean expression `<code> and content_type`, which failed at runtime and left the HTTP request unanswered; status/content_type values now parse as primary expressions
- `header "<Name>" of <request>` is now case-insensitive; warp normalizes header names to lowercase, so canonically-spelled names like `User-Agent` always returned nothing on real requests. Absent headers now compare equal to `nothing`
- The static analyzer now marks variables inside list literals (e.g. `parameters [user_name]`) as used
- `scripts/run_web_tests.sh` exited before running any test due to `set -e` combined with `((var++))` arithmetic increments

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
