# WFLHub Language Gaps — Product Requirements Document

## 1. Overview

### Purpose

WFLHub (the official WFL package registry, `wflhub.org`) is designed to be a **self-hosted WFL application** — a web server written entirely in WFL using the existing `listen on port`, `wait for request`, and `respond to` primitives. However, 10 language-level capabilities are missing that prevent building a production-grade registry. This document specifies each gap, proposes WFL-idiomatic syntax, outlines the Rust-level implementation approach, and provides acceptance criteria.

### Relationship to `wflpkg_prd.md`

The companion document `wflpkg_prd.md` defines WFLHub's product requirements — API endpoints, data model, authentication, web portal, and security. **That PRD assumes these 10 language features exist.** This document bridges the gap between the product vision and the current WFL runtime capabilities.

### Severity Definitions

| Severity | Meaning |
|----------|---------|
| **P0 — Blocker** | WFLHub cannot function at all without this feature. No workaround exists. |
| **P1 — Major** | Feature is required for production use but a degraded MVP could technically launch without it. |
| **P2 — Polish** | Feature improves reliability, developer experience, or security posture but is not strictly required for a functional registry. |

---

## 2. Summary Table

| # | Feature | Severity | Depends On | New Crate Dependencies | Effort | Phase |
|---|---------|----------|------------|----------------------|--------|-------|
| 1 | Binary File I/O | P0 | — | None | S | 1 |
| 2 | Database Access (SQL) | P0 | — | None (activate `sqlx`) | L | 1 |
| 3 | Password Hashing (Argon2) | P0 | — | `argon2` | S | 1 |
| 4 | JWT Sign/Verify | P0 | — | `jsonwebtoken` | M | 1 |
| 5 | Multipart Form Parsing | P0 | Binary I/O (#1) | `multer` or warp built-in | M | 1 |
| 6 | Server-Side Sessions | P1 | Database (#2) | None | M | 2 |
| 7 | Template Rendering | P1 | — | `tera` | M | 2 |
| 8 | Rate Limiting | P1 | — | None | S | 2 |
| 9 | Email Sending (SMTP) | P2 | — | `lettre` | M | 3 |
| 10 | Structured Route Registration | P2 | — | None | M | 3 |

**Effort key:** S = Small (1–3 days), M = Medium (3–5 days), L = Large (5–10 days)

---

## 3. Feature Specifications

---

### 3.1 Binary File I/O

**Severity:** P0 — Blocker
**Phase:** 1

#### Motivation & Current State

WFLHub must receive `.wflpkg` archive uploads (binary tar.gz files up to 50 MB) and serve them back for download. The current `open file at ... for reading/writing` only handles UTF-8 text via `IoClient::open_file_with_mode()` (`src/interpreter/mod.rs:477`). Binary data is silently corrupted or rejected.

The `FileOpenMode` enum (`src/parser/ast.rs:897`) has three variants: `Read`, `Write`, `Append` — all text-oriented.

#### Proposed WFL Syntax

```wfl
// Open for binary read
open file at "package.wflpkg" for reading binary as archive_handle

// Open for binary write
open file at "output.wflpkg" for writing binary as output_handle

// Read raw bytes (returns a binary buffer value)
store contents as read binary from archive_handle

// Read specific byte count
store chunk as read 4096 bytes from archive_handle

// Write raw bytes
write binary contents into output_handle

// Get file size in bytes
store size as size of archive_handle
```

#### Implementation Approach

1. **AST** (`src/parser/ast.rs`):
   - Add `ReadBinary` and `WriteBinary` variants to `FileOpenMode`.
   - Add `ReadBinaryStatement` and `WriteBinaryStatement` to `Statement` enum.

2. **Lexer** (`src/lexer/token.rs`):
   - Add `KeywordBinary` token (keyword `binary`).
   - Add `KeywordBytes` token (keyword `bytes`).

3. **Parser** (`src/parser/stmt/io.rs`):
   - Extend `parse_open_file_statement()` to detect `binary` after `reading`/`writing`.
   - Add `parse_read_binary_statement()` and `parse_write_binary_statement()`.

4. **Runtime Value** (`src/interpreter/value.rs`):
   - Add `Value::Binary(Vec<u8>)` variant for byte buffers.
   - Implement `Display` as `<binary: N bytes>`.

5. **IoClient** (`src/interpreter/mod.rs:477`):
   - `open_file_with_mode()`: When mode is `ReadBinary`/`WriteBinary`, open with `std::fs::OpenOptions` in raw mode (no UTF-8 conversion).
   - Add `read_binary()` and `write_binary()` methods that work with `Vec<u8>`.

6. **Interpreter** (`src/interpreter/mod.rs`):
   - Handle `ReadBinaryStatement` and `WriteBinaryStatement` in the statement dispatch.

#### New Dependencies

None — uses `std::fs` and `std::io` from the standard library.

#### Acceptance Criteria

- [ ] `open file at "test.bin" for writing binary as f` creates a binary file handle
- [ ] `write binary data into f` writes raw bytes without encoding transformation
- [ ] `read binary from f` returns a `Value::Binary` containing the exact bytes
- [ ] Round-trip: write then read a `.wflpkg` tar.gz file produces identical content (SHA-256 match)
- [ ] `read N bytes from f` returns exactly N bytes (or fewer at EOF)
- [ ] `size of handle` returns file size in bytes
- [ ] Text I/O continues to work unchanged (backward compatibility)

---

### 3.2 Database Access (SQL)

**Severity:** P0 — Blocker
**Phase:** 1

#### Motivation & Current State

WFLHub requires PostgreSQL for users, packages, versions, and API tokens (see `wflpkg_prd.md` §4.5 Data Model). The `sqlx` crate version 0.8.1 is **already listed in `Cargo.toml`** with features `runtime-tokio-rustls`, `sqlite`, `mysql`, and `postgres` — but is completely unused. The keyword `database` is already reserved (`KeywordDatabase`, `src/lexer/token.rs:127`).

#### Proposed WFL Syntax

```wfl
// Connect to a database
connect to database at "postgres://user:pass@host/db" as db

// Execute a query that returns rows
execute query "SELECT id, name FROM packages WHERE name = $1" on db with parameters [package_name] as results

// Execute a mutation (INSERT, UPDATE, DELETE)
execute query "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3)" on db with parameters [username, email, hash]

// Iterate over result rows
count through results as row:
    store pkg_id as field "id" of row
    store pkg_name as field "name" of row
    display pkg_name
end count

// Close connection
close database db

// Transaction support
begin transaction on db as tx
execute query "UPDATE packages SET downloads_total = downloads_total + 1 WHERE id = $1" on tx with parameters [package_id]
commit transaction tx
// Or: rollback transaction tx
```

#### Implementation Approach

1. **AST** (`src/parser/ast.rs`):
   - Add `ConnectDatabaseStatement { url: Expression, variable_name: String }`.
   - Add `ExecuteQueryStatement { query: Expression, connection: Expression, parameters: Option<Expression>, result_name: Option<String> }`.
   - Add `CloseDatabaseStatement { connection: Expression }`.
   - Add `BeginTransactionStatement`, `CommitTransactionStatement`, `RollbackTransactionStatement`.

2. **Lexer** (`src/lexer/token.rs`):
   - `KeywordDatabase` already exists. Add: `KeywordQuery`, `KeywordParameters`, `KeywordTransaction`, `KeywordCommit`, `KeywordRollback`.

3. **Parser** — new file `src/parser/stmt/database.rs`:
   - `parse_connect_database_statement()`: `connect to database at <expr> as <name>`
   - `parse_execute_query_statement()`: `execute query <expr> on <expr> [with parameters <expr>] [as <name>]`
   - `parse_close_database_statement()`: `close database <name>`
   - Transaction parsers for begin/commit/rollback.

4. **Runtime Value** (`src/interpreter/value.rs`):
   - Add `Value::DatabaseConnection(...)` wrapping a `sqlx::AnyPool`.
   - Add `Value::DatabaseRow(...)` wrapping column data as a map.
   - Add `Value::Transaction(...)` for transaction handles.

5. **Interpreter** (`src/interpreter/mod.rs`):
   - New module `src/interpreter/database.rs` containing async functions that call `sqlx`.
   - Parameterized queries only — never string-interpolated SQL (SQL injection prevention).
   - Connection pooling via `sqlx::AnyPool` (reuse connections).

6. **Activate `sqlx`** — no `Cargo.toml` change needed; just `use sqlx` in new code.

#### New Dependencies

None — `sqlx 0.8.1` is already in `Cargo.toml`. Just needs to be used.

#### Acceptance Criteria

- [ ] `connect to database at <url> as db` establishes a connection pool
- [ ] `execute query ... on db with parameters [...] as results` runs a parameterized SELECT and returns rows
- [ ] `execute query ... on db with parameters [...]` runs INSERT/UPDATE/DELETE and returns affected row count
- [ ] `field "column_name" of row` extracts a column value from a result row
- [ ] Result rows support iteration with `count through`
- [ ] `close database db` cleanly shuts down the connection pool
- [ ] Transactions: begin/commit/rollback work correctly
- [ ] SQL injection is impossible — all queries use parameterized bindings
- [ ] Connection errors produce clear WFL runtime errors

---

### 3.3 Password Hashing (Argon2)

**Severity:** P0 — Blocker
**Phase:** 1

#### Motivation & Current State

WFLHub requires Argon2 password hashing for user registration and login (`wflpkg_prd.md` §4.2, §4.5). The existing `src/stdlib/crypto.rs` implements WFLHASH (a custom hash function) but has no password hashing capability. WFLHASH is a fast hash — unsuitable for passwords.

#### Proposed WFL Syntax

```wfl
// Hash a password (returns Argon2 PHC string)
store hashed as hash password user_password

// Verify a password against a stored hash (returns true/false)
store is_valid as verify password attempt against stored_hash
```

#### Implementation Approach

1. **Stdlib** (`src/stdlib/crypto.rs`):
   - Add `hash_password(env)` function: takes a plaintext string, returns an Argon2id PHC-format string (e.g., `$argon2id$v=19$m=19456,t=2,p=1$...`).
   - Add `verify_password(env)` function: takes plaintext + PHC hash, returns boolean.
   - Use `argon2` crate with recommended defaults (Argon2id, 19456 KiB memory, 2 iterations, 1 lane).

2. **Registration** in `register_crypto()` (`src/stdlib/crypto.rs`):
   - Register `"hash password"` and `"verify password"` as built-in functions.

3. **Alternative: Parser-level** — could also be parsed as dedicated statements, but stdlib functions are simpler and consistent with existing crypto functions. Prefer the stdlib approach.

#### New Dependencies

```toml
argon2 = "0.5"
```

#### Acceptance Criteria

- [ ] `hash password "mysecret"` returns a string starting with `$argon2id$`
- [ ] `verify password "mysecret" against hashed` returns `true`
- [ ] `verify password "wrong" against hashed` returns `false`
- [ ] Different calls to `hash password` with the same input produce different hashes (random salt)
- [ ] Hashing takes at least 100ms (proof of computational cost)
- [ ] Empty password is rejected with a clear error

---

### 3.4 JWT Sign / Verify

**Severity:** P0 — Blocker
**Phase:** 1

#### Motivation & Current State

WFLHub uses JWT tokens for API authentication (`wflpkg_prd.md` §4.2.2). The `wflpkg` CLI stores tokens in `~/.wfl/auth.json`. There is no JWT capability in the current runtime — no signing, no verification, no claims extraction.

#### Proposed WFL Syntax

```wfl
// Create claims as a regular WFL object
store claims as create object with "sub" as "user123" and "exp" as expiry_timestamp and "scope" as "publish"

// Sign a JWT
store token as sign jwt with claims my_claims and secret jwt_secret

// Verify and decode a JWT (returns claims object or null on failure)
store decoded as verify jwt token_string with secret jwt_secret

// Access decoded claims
check if decoded is not null:
    store user_id as field "sub" of decoded
    store scope as field "scope" of decoded
end check
```

#### Implementation Approach

1. **Stdlib** — new file `src/stdlib/jwt.rs`:
   - `sign_jwt(env)`: Takes a WFL object (claims) and a secret string. Converts claims to JSON, signs with HMAC-SHA256 (HS256), returns the JWT string.
   - `verify_jwt(env)`: Takes a JWT string and secret. Verifies signature and expiration. Returns decoded claims as a WFL object, or `Value::Nothing` on failure.

2. **Register in stdlib** (`src/stdlib/mod.rs`):
   - Add `pub mod jwt;` and call `jwt::register_jwt(env)` in `register_stdlib()`.

3. **Claims mapping**: WFL objects (`Value::Object`) map naturally to JSON claims. The `exp` field (if present) is validated as a Unix timestamp.

#### New Dependencies

```toml
jsonwebtoken = "9"
```

#### Acceptance Criteria

- [ ] `sign jwt with claims obj and secret "key"` returns a valid JWT string (3 dot-separated Base64 segments)
- [ ] `verify jwt token with secret "key"` returns the original claims object
- [ ] Verification with wrong secret returns `nothing`
- [ ] Expired tokens (past `exp` claim) return `nothing`
- [ ] Claims support standard fields: `sub`, `exp`, `iat`, `scope`, and custom fields
- [ ] Non-string secrets are rejected with a clear error

---

### 3.5 Multipart Form Parsing

**Severity:** P0 — Blocker
**Phase:** 1

#### Motivation & Current State

The publish endpoint (`POST /api/v1/packages`, `wflpkg_prd.md` §4.1.3) requires multipart form data: text fields (`name`, `version`, `checksum`) plus a binary file (`archive`). The current `wait for request comes in on server as req` captures the request but provides no way to parse multipart bodies. The `method of`, `path of`, `header of`, and `body of` accessors exist, but `body of` returns the raw body as a text string — unusable for multipart.

This feature depends on **Binary I/O (#1)** because the extracted file field must produce a `Value::Binary`.

#### Proposed WFL Syntax

```wfl
wait for request comes in on server as req

// Parse multipart form from request
store form as parse multipart from req

// Access text fields
store name as field "name" of form
store version as field "version" of form

// Access file fields (returns binary value)
store archive as file "archive" of form
store filename as filename "archive" of form
store content_type as content_type "archive" of form

// Check if field exists
check if has field "archive" in form:
    display "Archive uploaded"
end check
```

#### Implementation Approach

1. **AST** (`src/parser/ast.rs`):
   - Add `ParseMultipartExpression { request: Expression }` to `Expression` enum.

2. **Lexer** (`src/lexer/token.rs`):
   - Add `KeywordMultipart` token.
   - Add `KeywordParse` token (if not already present).

3. **Parser**:
   - Parse `parse multipart from <expr>` as an expression that returns a multipart form object.

4. **Runtime Value** (`src/interpreter/value.rs`):
   - Add `Value::MultipartForm(HashMap<String, MultipartField>)` where `MultipartField` holds either text or binary data plus metadata (filename, content type).

5. **Interpreter** — integration with warp's multipart support or the `multer` crate:
   - When the server receives a request with `Content-Type: multipart/form-data`, the raw body bytes are stored.
   - `parse multipart from req` runs `multer` on those bytes.
   - Text fields become `Value::Text`, file fields become `Value::Binary`.

#### New Dependencies

```toml
multer = "3"
```

Or use warp's built-in `warp::multipart` (already available since `warp` is a dependency).

#### Acceptance Criteria

- [ ] `parse multipart from req` extracts all form fields
- [ ] Text fields accessible via `field "name" of form`
- [ ] File fields accessible via `file "archive" of form` as `Value::Binary`
- [ ] `filename "archive" of form` returns the original filename
- [ ] Missing fields return `nothing` (not an error)
- [ ] Malformed multipart data produces a clear runtime error
- [ ] Archives up to 50 MB parse successfully without memory issues

---

### 3.6 Server-Side Sessions

**Severity:** P1 — Major
**Phase:** 2

#### Motivation & Current State

WFLHub's web portal requires session-based authentication for browser users (`wflpkg_prd.md` §4.2.4). A TDD test already exists at `TestPrograms/web_server_session_test.wfl` defining the expected API — this specification aligns with that test.

The test expects: session-enabled server setup, session creation/retrieval/destruction, CSRF token generation, session value get/set, and session expiration.

This feature depends on **Database Access (#2)** for persistent session storage.

#### Proposed WFL Syntax

Aligned with the existing TDD test (`TestPrograms/web_server_session_test.wfl`):

```wfl
// Create a session-enabled server
listen on port 8080 as server with sessions enabled

// Configure sessions
configure sessions on server with timeout 1800000 and storage "memory"
enable csrf protection on server
enable secure cookies on server

// On a request — create a session
store session as create session for request
set session value "user_id" to "user123" in session
store csrf_token as generate csrf token for session

// Respond with session cookie
respond to request with response_body and content_type "application/json" and set session session

// Retrieve session from subsequent request
store session as get session from request

// Read session data
store user_id as get session value "user_id" from session

// Destroy session
destroy session my_session
respond to request with "{}" and clear session

// Expiration cleanup
store expired as find expired sessions on server
```

#### Implementation Approach

1. **AST** (`src/parser/ast.rs`):
   - Extend `ListenStatement` with optional `sessions_enabled: bool`.
   - Add `ConfigureSessionsStatement`, `CreateSessionExpression`, `GetSessionExpression`, `SetSessionValueStatement`, `DestroySessionStatement`.
   - Extend `RespondStatement` with optional `set_session` / `clear_session` fields.

2. **Parser** (`src/parser/stmt/web.rs`):
   - Extend `parse_listen_statement()` to handle `with sessions enabled`.
   - Add parsers for session-related statements.

3. **Runtime** — new module `src/interpreter/sessions.rs`:
   - `SessionStore` trait with `MemorySessionStore` and `DatabaseSessionStore` implementations.
   - Session IDs: cryptographically random 32-byte hex strings.
   - CSRF tokens: separate random tokens stored in session.
   - Session data stored as `HashMap<String, Value>`.
   - Automatic cookie management (`Set-Cookie` header on respond with session).

4. **Interpreter** (`src/interpreter/mod.rs`):
   - Server state extended with optional `SessionManager`.
   - Request processing checks for session cookie, loads session.
   - Response processing sets/clears session cookies.

#### New Dependencies

None — session management is implemented in pure Rust. Uses existing `rand` for token generation.

#### Acceptance Criteria

- [ ] `listen on port ... as server with sessions enabled` starts a session-capable server
- [ ] `create session for request` creates a new session with unique ID
- [ ] `set session value "key" to value in session` stores data
- [ ] `get session value "key" from session` retrieves stored data
- [ ] `get session from request` retrieves existing session via cookie
- [ ] `destroy session s` invalidates the session
- [ ] `respond ... and set session s` sends `Set-Cookie` header
- [ ] `respond ... and clear session` sends cookie expiration header
- [ ] Sessions expire after configured timeout
- [ ] CSRF token generation and validation work
- [ ] Existing TDD test (`TestPrograms/web_server_session_test.wfl`) passes

---

### 3.7 Template Rendering

**Severity:** P1 — Major
**Phase:** 2

#### Motivation & Current State

WFLHub's web portal (`wflpkg_prd.md` §4.3) requires server-rendered HTML pages — homepage, package pages, search results, user profiles, and dashboard. Currently, WFL can only respond with string literals or concatenated strings, making HTML generation extremely cumbersome and unmaintainable.

#### Proposed WFL Syntax

```wfl
// Load a template engine
store templates as load templates from "templates/"

// Render a template with data
store data as create object with "title" as "WFLHub" and "packages" as package_list
store html as render template "home.html" with data using templates

// Render inline template string
store html as render template string "<h1>{{ title }}</h1>" with data

// Respond with rendered HTML
respond to request with html and content_type "text/html"
```

#### Implementation Approach

1. **Stdlib** — new file `src/stdlib/templates.rs`:
   - `load_templates(env)`: Takes a directory path, returns a `Value::TemplateEngine` wrapping a `tera::Tera` instance.
   - `render_template(env)`: Takes template name, data object, and engine. Converts WFL object to `tera::Context`, renders, returns HTML string.
   - `render_template_string(env)`: One-off render from a string template.

2. **Register in stdlib** (`src/stdlib/mod.rs`):
   - Add `pub mod templates;` and register in `register_stdlib()`.

3. **Value conversion**: WFL `Value::Object` maps to Tera's `Context`. `Value::List` maps to Tera arrays. `Value::Text`, `Value::Number`, `Value::Boolean` map directly.

4. **Template language**: Use Tera's `{{ variable }}`, `{% for %}`, `{% if %}` syntax. This is separate from WFL syntax and lives in `.html` files.

#### New Dependencies

```toml
tera = "1"
```

#### Acceptance Criteria

- [ ] `load templates from "templates/"` loads all `.html` files from the directory
- [ ] `render template "name.html" with data using engine` produces HTML with substituted values
- [ ] Template loops (`{% for %}`) work with WFL lists
- [ ] Template conditionals (`{% if %}`) work with WFL booleans
- [ ] Missing variables produce clear errors (not silent empty strings)
- [ ] Template syntax errors are caught at load time with file/line info
- [ ] HTML auto-escaping is enabled by default (XSS prevention)

---

### 3.8 Rate Limiting

**Severity:** P1 — Major
**Phase:** 2

#### Motivation & Current State

WFLHub requires rate limiting on all API endpoints (`wflpkg_prd.md` §4.4.3): publish (10/hr/user), search (60/min/IP), download (300/min/IP), login (5/min/IP with backoff). The middleware TDD test at `TestPrograms/web_server_middleware_test.wfl` simulates rate limiting but notes it needs real implementation.

Currently there is no mechanism to track request rates per client.

#### Proposed WFL Syntax

```wfl
// Create a rate limiter
store limiter as create rate limiter with limit 60 per "minute"

// Create a rate limiter with custom key (e.g., per-user)
store publish_limiter as create rate limiter with limit 10 per "hour"

// Check rate limit (returns true if allowed, false if exceeded)
store client_ip as client_ip of request
store allowed as check rate limit on limiter for client_ip

check if allowed is false:
    respond to request with "Rate limit exceeded" and status 429
end check

// Get remaining requests
store remaining as remaining requests on limiter for client_ip

// Reset rate limit (admin use)
reset rate limit on limiter for client_ip
```

#### Implementation Approach

1. **Stdlib** — new file `src/stdlib/ratelimit.rs`:
   - In-memory sliding-window rate limiter using `HashMap<String, VecDeque<Instant>>`.
   - `create_rate_limiter(env)`: Creates limiter with given limit and window.
   - `check_rate_limit(env)`: Checks if a key has remaining capacity. Prunes expired entries.
   - `remaining_requests(env)`: Returns count of remaining requests in current window.
   - `reset_rate_limit(env)`: Clears entries for a key.

2. **Runtime Value** (`src/interpreter/value.rs`):
   - Add `Value::RateLimiter(Rc<RefCell<RateLimiterState>>)`.

3. **Register in stdlib** (`src/stdlib/mod.rs`).

#### New Dependencies

None — implemented with `std::collections::HashMap`, `std::collections::VecDeque`, and `std::time::Instant`.

#### Acceptance Criteria

- [ ] `create rate limiter with limit 60 per "minute"` creates a limiter allowing 60 requests/minute
- [ ] `check rate limit on limiter for "key"` returns `true` when under limit, `false` when exceeded
- [ ] Sliding window: requests from >1 window ago don't count against current limit
- [ ] `remaining requests on limiter for "key"` returns correct count
- [ ] Different keys are tracked independently
- [ ] Limiter is safe for concurrent use within WFL's async runtime
- [ ] Memory is bounded — expired entries are pruned automatically

---

### 3.9 Email Sending (SMTP)

**Severity:** P2 — Polish
**Phase:** 3

#### Motivation & Current State

WFLHub requires email for user registration verification (`wflpkg_prd.md` §4.2.1) and potentially for security notifications. There is no SMTP or email capability in the current runtime.

#### Proposed WFL Syntax

```wfl
// Configure email transport
store mailer as connect to email server at "smtp.example.com" with port 587 and username "noreply@wflhub.org" and password smtp_password

// Compose and send an email
store email as create email with from "noreply@wflhub.org" and to user_email and subject "Verify your WFLHub account" and body email_body
send email using mailer

// Send with HTML body
store email as create email with from "noreply@wflhub.org" and to user_email and subject "Welcome" and html_body html_content
send email using mailer

// Close connection
close email server mailer
```

#### Implementation Approach

1. **Stdlib** — new file `src/stdlib/email.rs`:
   - `connect_email_server(env)`: Creates an SMTP transport via `lettre`. Returns a `Value::EmailTransport`.
   - `create_email(env)`: Builds a `lettre::Message` from WFL arguments. Returns a `Value::EmailMessage`.
   - `send_email(env)`: Sends the message via the transport. Returns success/failure.

2. **Register in stdlib** (`src/stdlib/mod.rs`).

3. **Security considerations**:
   - SMTP credentials should come from environment variables or config, not hardcoded.
   - TLS required by default (STARTTLS on port 587).
   - Rate limit outbound emails to prevent abuse.

#### New Dependencies

```toml
lettre = "0.11"
```

#### Acceptance Criteria

- [ ] `connect to email server at "smtp://..." ...` establishes SMTP connection
- [ ] `create email with from ... and to ... and subject ... and body ...` builds a valid email
- [ ] `send email using mailer` delivers the email via SMTP
- [ ] HTML body support works (`html_body` field)
- [ ] TLS is enforced by default
- [ ] Connection errors produce clear WFL runtime errors
- [ ] Invalid email addresses are rejected at compose time

---

### 3.10 Structured Route Registration

**Severity:** P2 — Polish
**Phase:** 3

#### Motivation & Current State

WFLHub has 10+ distinct API endpoints (`wflpkg_prd.md` §4.1) plus web portal routes. The current pattern requires a single `wait for request` loop with a chain of `check if path is equal to "/..."` conditionals — resulting in a massive, unmaintainable routing block.

#### Proposed WFL Syntax

```wfl
listen on port 8080 as server

// Register routes with method and path
register route "GET" at "/api/v1/search" on server as handle_search
register route "GET" at "/api/v1/packages/:name" on server as handle_package_info
register route "POST" at "/api/v1/packages" on server as handle_publish
register route "DELETE" at "/api/v1/packages/:name/:version" on server as handle_yank
register route "GET" at "/api/v1/packages/:name/:version/download" on server as handle_download

// Define route handlers as actions
define action called handle_search with parameters request:
    store query as parameter "q" of request
    // ... search logic ...
    respond to request with results and content_type "application/json"
end action

define action called handle_package_info with parameters request:
    store name as route parameter "name" of request
    // ... lookup logic ...
    respond to request with info and content_type "application/json"
end action

// Start accepting connections (dispatches to registered handlers)
start server
```

#### Implementation Approach

1. **AST** (`src/parser/ast.rs`):
   - Add `RegisterRouteStatement { method: Expression, path: Expression, server: Expression, handler_name: String }`.
   - Add `StartServerStatement { server: Expression }`.

2. **Lexer** (`src/lexer/token.rs`):
   - Add `KeywordRoute`, `KeywordRegister` tokens.

3. **Parser** — extend `src/parser/stmt/web.rs`:
   - `parse_register_route_statement()`: `register route <method> at <path> on <server> as <handler>`
   - `parse_start_server_statement()`: `start server` or `start <server_name>`

4. **Runtime** — extend server infrastructure in interpreter:
   - `RouteTable` struct: maps `(method, path_pattern)` to WFL action names.
   - Path parameter extraction (`:name` segments).
   - Query parameter extraction (`parameter "q" of request`).
   - `start server` enters event loop, dispatching requests to matched handlers.

5. **Interpreter**:
   - `register route` adds entries to the server's route table.
   - `start server` replaces the manual `wait for request` loop with automatic dispatch.
   - Unmatched routes get a 404 response.

#### New Dependencies

None — route matching is simple string/pattern matching.

#### Acceptance Criteria

- [ ] `register route "GET" at "/path" on server as handler` registers a route
- [ ] Routes with path parameters (`:name`) extract values accessible via `route parameter "name" of request`
- [ ] Query parameters accessible via `parameter "q" of request`
- [ ] `start server` dispatches requests to correct handlers
- [ ] Method mismatch returns 405 Method Not Allowed
- [ ] Path mismatch returns 404 Not Found
- [ ] Multiple routes can be registered for the same path with different methods
- [ ] Route registration order doesn't affect matching (most specific wins)
- [ ] Existing `wait for request` pattern continues to work (backward compatibility)

---

## 4. Dependency Graph

```
Binary I/O (#1) ──────────────────────► Multipart Form Parsing (#5)
                                              │
                                              ▼
                                     [Publish endpoint works]

Database Access (#2) ─────────────────► Server-Side Sessions (#6)
                                              │
                                              ▼
                                     [Web portal auth works]

Password Hashing (#3) ───────┐
                              ├──────► [User auth system works]
JWT Sign/Verify (#4) ────────┘

Template Rendering (#7) ─────────────► [Web portal pages work]

Rate Limiting (#8) ──────────────────► [API protection works]

Email Sending (#9) ──────────────────► [User verification works]

Structured Routing (#10) ────────────► [Clean API dispatch]
```

**Critical path:** Features #1–#5 (Phase 1) are all P0 blockers. The registry API cannot accept or serve packages without binary I/O and multipart parsing. It cannot authenticate users without database access, password hashing, and JWT.

**Independent tracks in Phase 1:**
- Track A: Binary I/O → Multipart (serial)
- Track B: Database Access (independent)
- Track C: Password Hashing + JWT (independent, can parallelize)

---

## 5. Phased Roadmap

### Phase 1 — Blockers (Weeks 1–4)

| Week | Feature | Track | Effort |
|------|---------|-------|--------|
| 1 | Binary I/O (#1) | A | S (1–3 days) |
| 1–2 | Database Access (#2) | B | L (5–10 days) |
| 2 | Password Hashing (#3) | C | S (1–3 days) |
| 2–3 | JWT Sign/Verify (#4) | C | M (3–5 days) |
| 3–4 | Multipart Form Parsing (#5) | A | M (3–5 days) |

**Gate:** At the end of Phase 1, a minimal WFLHub can: accept user registration, authenticate via JWT, receive package uploads via multipart, store metadata in PostgreSQL, and serve binary package downloads.

### Phase 2 — Major Features (Weeks 5–8)

| Week | Feature | Effort |
|------|---------|--------|
| 5–6 | Server-Side Sessions (#6) | M (3–5 days) |
| 6–7 | Template Rendering (#7) | M (3–5 days) |
| 7–8 | Rate Limiting (#8) | S (1–3 days) |

**Gate:** At the end of Phase 2, WFLHub has a usable web portal with HTML pages, browser-based login sessions, and API rate limiting.

### Phase 3 — Polish (Weeks 9–10)

| Week | Feature | Effort |
|------|---------|--------|
| 9 | Email Sending (#9) | M (3–5 days) |
| 9–10 | Structured Routing (#10) | M (3–5 days) |

**Gate:** At the end of Phase 3, WFLHub has email verification and clean route-based code organization.

---

## 6. Cross-Cutting Concerns

### 6.1 Backward Compatibility

**Sacred rule: Never break existing WFL programs.**

- All new keywords (`binary`, `bytes`, `query`, `parameters`, `transaction`, `commit`, `rollback`, `multipart`, `parse`, `route`, `register`) must be checked against existing programs in `TestPrograms/`.
- New `Value` variants (`Binary`, `DatabaseConnection`, `DatabaseRow`, `RateLimiter`, etc.) must implement `Display` and comparison operators gracefully.
- Existing `FileOpenMode::Read`/`Write`/`Append` continue to work unchanged.
- Existing `listen on port ... as server` without `with sessions enabled` continues to work.
- Existing `wait for request` pattern is not deprecated by structured routing.

### 6.2 Security

- **SQL injection**: Enforced via parameterized queries only. The parser should reject raw string interpolation in `execute query`.
- **Password storage**: Argon2id with recommended parameters. Never log or display password hashes.
- **JWT secrets**: Should be loaded from environment variables or config files, never hardcoded. Use `zeroize` for secret key memory.
- **Session tokens**: Cryptographically random, 256-bit minimum entropy. HttpOnly and Secure cookie flags.
- **Template rendering**: HTML auto-escaping enabled by default to prevent XSS.
- **Rate limiting**: Applied before authentication to prevent brute-force attacks.
- **Binary uploads**: Size limits enforced before reading full body into memory.
- **Email**: TLS required, rate limit outbound to prevent abuse as spam relay.

### 6.3 Configuration

New features should respect `.wflcfg` project configuration where applicable:

```
database_url is "postgres://..."
jwt_secret is "..."
smtp_host is "..."
smtp_port is 587
session_timeout is 1800000
rate_limit_default is 60
template_dir is "templates/"
```

### 6.4 Documentation

Each feature requires:
- Entry in `Docs/05-standard-library/` for stdlib additions (crypto, jwt, templates, email, ratelimit)
- Entry in `Docs/04-advanced-features/` for language-level additions (binary I/O, database, sessions, routing)
- Working examples in `TestPrograms/docs_examples/`
- Updates to `Docs/reference/keyword-reference.md` and `Docs/reference/reserved-keywords.md` for new keywords

### 6.5 Testing Strategy

- **TDD mandatory**: Each feature starts with failing tests.
- **Existing TDD tests**: `TestPrograms/web_server_session_test.wfl` and `TestPrograms/web_server_middleware_test.wfl` define expected behavior for sessions and middleware/rate-limiting.
- **Unit tests**: Rust-level tests in each new module (`src/stdlib/jwt.rs`, `src/interpreter/database.rs`, etc.).
- **Integration tests**: WFL programs in `TestPrograms/` that exercise the full pipeline (parse → interpret → runtime).
- **MCP validation**: All documentation code examples validated with `mcp__wfl-lsp__parse_wfl` and `mcp__wfl-lsp__analyze_wfl` before inclusion.

### 6.6 New Cargo Dependencies Summary

| Crate | Version | Feature | Size Impact |
|-------|---------|---------|-------------|
| `argon2` | 0.5 | Password Hashing (#3) | ~150 KB |
| `jsonwebtoken` | 9 | JWT (#4) | ~200 KB |
| `multer` | 3 | Multipart (#5) | ~100 KB |
| `tera` | 1 | Templates (#7) | ~500 KB |
| `lettre` | 0.11 | Email (#9) | ~300 KB |
| `sqlx` | 0.8.1 | Database (#2) | Already included |

**Total new dependency footprint:** ~1.25 MB compiled (estimate).

---

## 7. Open Questions

1. **Database dialect**: Should WFLHub target PostgreSQL exclusively, or should the database abstraction support SQLite for local development? (`sqlx` supports both via `AnyPool`.)
2. **Template engine choice**: Tera vs Handlebars? Tera is more feature-rich (macros, inheritance) but larger. Handlebars is simpler and more familiar to JS developers.
3. **Session storage backend**: Memory-only for v1, or implement database-backed sessions immediately? Memory sessions don't survive server restarts.
4. **Binary value interop**: How should `Value::Binary` interact with existing string operations? Should `length of binary_val` return byte count? Should concatenation of two binaries work?
5. **Route parameter syntax**: `:name` (Express-style) vs `{name}` (Axum-style) vs `<name>` (Flask-style)?
6. **Multipart size limits**: Per-field limits or only total request size? Should file fields stream to disk for large uploads?
