# Database Bindings, Route Parameters, and Web Server Fixes

**Date:** 2026-06-12

## What Changed

### Database support (SQLite, PostgreSQL, MariaDB)

The sqlx dependency that has been sitting in Cargo.toml is now exposed to WFL
programs:

```wfl
open database at "sqlite://app.db" as db
store inserted as execute db with "INSERT INTO users (name) VALUES (?)" and parameters ["Alice"]
store users as query db with "SELECT * FROM users"
close database db
```

- Backends route by URL scheme: `sqlite://` / `sqlite::memory:`,
  `postgres://` / `postgresql://`, `mysql://` / `mariadb://` (MariaDB uses
  the MySQL driver).
- Runtime lives in `src/interpreter/database.rs` as an explicit `DbPool` enum
  (deliberately not sqlx `Any`, which has lossy type mapping). Pools hang off
  `IoClient` behind string handles, exactly like file handles.
- Rows decode to lists of objects keyed by column name; SQL `NULL` maps to
  the runtime value of the `nothing` literal (`Value::Null`).
- Parameters always go through driver-level `.bind()` — SQL injection via
  values is structurally impossible. Placeholders are driver-native (`?` vs
  `$1`); we do not rewrite SQL text.
- No new lexer keywords: `connect` and `query` are contextual with strict
  lookahead, so existing programs (including variables named `query`) parse
  unchanged.

### Route parameters for the web server

New stdlib helpers in `src/stdlib/web.rs`:

```wfl
store params as path_params of path and "/users/:id"
check if params is nothing:
    respond to req with "Not Found" and status 404
otherwise:
    respond to req with params["id"]
end check
```

`:name` captures one segment (percent-decoded), trailing `*name` captures the
rest, `path_matches` gives a boolean. Implemented as a split-segment matcher —
the pattern VM would have been overkill.

### Web server fixes (FRAMEWORK_FINAL_REPORT follow-up)

Investigating the archived framework report's blockers against live servers
turned up two real bugs, both now fixed with regression tests:

1. **respond status clause swallowed the rest of the line.**
   `respond to req with "x" and status 404 and content_type "text/plain"`
   parsed the status as the boolean expression `404 and content_type`, which
   failed at runtime (undefined variable) and left the request unanswered —
   every 404 path in the comprehensive demo was silently broken. Status and
   content_type values now parse as primary expressions.
2. **Header access never matched on real requests.** warp lowercases header
   names, but the lookup was exact-match, so `header "User-Agent" of req`
   always returned nothing. Lookup is now case-insensitive, and absent
   headers compare equal to `nothing`.

The report's headline claim ("`wait for request comes in on ...` does not
parse") no longer reproduces; parser regression tests in
`tests/main_loop_parser_test.rs` lock in the try/catch-inside-main-loop
shapes.

## Why

Strategic direction: close primitive-level gaps in the runtime (database
access first) and let higher-level framework features live as WFL packages
later, rather than building a Laravel-style framework into the core.

## Testing

- TDD throughout — every feature/fix started from a failing test.
- `tests/database_parser_test.rs`, `tests/database_test.rs` (SQLite suites run
  everywhere; PostgreSQL/MariaDB suites gate on `WFL_TEST_POSTGRES_URL` /
  `WFL_TEST_MYSQL_URL` and were verified against live PostgreSQL 16 and
  MariaDB 10.11/11 servers).
- `tests/route_params_test.rs`, `tests/respond_statement_parser_test.rs`,
  `tests/header_access_runtime_test.rs` (real warp server round-trips).
- E2E: `TestPrograms/database_sqlite_test.wfl` (runs in the standard program
  suites) and `TestPrograms/web_route_params_test.wfl` (driven by
  `scripts/run_web_tests.sh|ps1` with curl assertions).
- CI: new `database-tests` job with postgres:16 and mariadb:11 service
  containers.

## Docs

`Docs/04-advanced-features/databases.md` (new, all examples parse-validated;
the complete example runs), route-parameters section in `web-servers.md`,
keyword reference notes for the reserved statement shapes, CHANGELOG.
