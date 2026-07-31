# HTTPS/TLS support for the built-in web server

**Date:** 2026-07-04

## What was added

The `listen` statement grew two optional clauses:

```wfl
// HTTPS, paths in the code
listen on port 8443 secured with certificate "cert.pem" and key "key.pem" as secure_server

// HTTPS, paths from .wflcfg (web_server_tls_cert_file / web_server_tls_key_file)
listen on port 8443 secured as secure_server

// Native HTTP -> HTTPS 301 redirect server
listen on port 8080 redirecting to port 8443 as redirect_server
```

Plain `listen` is untouched, and running an HTTP server *alongside* an HTTPS one
(instead of redirecting) works by simply issuing two `listen` statements.

## Design decisions worth remembering

### No new keywords

`secured`, `certificate`, `key`, and `redirecting` are **plain identifiers**
matched positionally by the parser, not lexer tokens. `key` in particular is
used as a variable in existing programs (`TestPrograms/hash_security_test.wfl`
does `store key as "secret_key_456"`), so reserving it would have broken
backward compatibility. The keyword count stays at 178.

The cost is dealing with the lexer's multi-word identifier merging: adjacent
identifiers fuse into one token, so `port my_port secured` arrives as
`Identifier("my_port secured")` and `certificate cert_var` as
`Identifier("certificate cert_var")`. The parser splits these apart with the
same strip-prefix/suffix approach `respond ... and content_type` already uses.
One subtlety: the merged `... secured` detection must happen *before* the port
expression is parsed, because `with` is a concatenation operator and
`parse_expression` would swallow `with certificate "..."` into the port
expression.

`redirecting to` (rather than `and redirect to`) avoids the same trap: `and`
is a boolean operator and would be absorbed by the port expression.

### Config precedence

TLS intent always lives in the program; `.wflcfg` only supplies default file
paths for the bare `secured` form. A plain `listen` never becomes HTTPS via
config — otherwise dropping cert paths into `.wflcfg` would silently convert
the HTTP half of a dual-server setup.

### warp TLS plumbing

- `warp` is now built with `features = ["tls"]`. That pulls in tokio-rustls
  0.25 → **rustls 0.22.4**, which coexists with the **rustls 0.23.35** already
  in the tree via sqlx's `runtime-tokio-rustls`. Two rustls minors compile
  fine side by side; unifying them means upgrading warp (or replacing it)
  some day.
- warp's `TlsServer` has no `try_bind_ephemeral`, and its `bind_ephemeral`
  panics *inside the spawned task* on a bad certificate or occupied port. The
  interpreter therefore uses `try_bind_with_graceful_shutdown` with a
  never-completing signal (`std::future::pending()`), which returns both TLS
  config errors and bind errors synchronously as `Result`. On top of that,
  certificate/key files are pre-validated with `rustls-pemfile` (now a direct
  dependency; it was already in the lockfile) so a missing or malformed file
  produces an actionable message naming the path.

### Redirect servers are native

The redirect listener answers 301 inside warp itself — requests never enter
the WFL request loop, so `wait for request` on a redirect server never fires
(documented). This sidesteps the fact that `respond` can't set custom headers
yet (the `Location` header requirement). The Location URL preserves host
(port stripped, IPv6 brackets kept), path, and query, and omits the target
port when it's 443. The server is still registered in `web_servers` so
`close server` works.

## Testing

- `tests/web_server_tls_parser_test.rs` — 14 parser cases incl. all merged
  identifier forms and error cases.
- `tests/web_server_tls_test.rs` — end-to-end with rcgen-generated
  self-signed certs (localhost + 127.0.0.1 SANs; reqwest needs
  `danger_accept_invalid_certs`): HTTPS round-trip, HTTP-to-TLS-port failure,
  redirect Location assertion, config-driven bare `secured`, dual HTTP+HTTPS,
  and both actionable error paths.
- `scripts/run_web_tests.sh` / `.ps1` — new TLS section generating a cert
  with openssl and driving `TestPrograms/web_server_tls.wfl` (CI-SKIP'd in
  the plain integration run) with curl. Readiness is probed via the redirect
  port so the probe doesn't consume the program's single `wait for request`.

## Known limitations

- Application-level responses remain sequential per server (existing
  behavior); TLS handshakes are concurrent inside warp.
- The JS transpiler emits a warning for `secured`/`redirecting` listens and
  generates a plain `http` server.
- Exotic merged-identifier shapes (e.g. a port expression like
  `base_port plus offset secured`) aren't recognized; use a simple variable
  or literal port with the `secured` clause.
