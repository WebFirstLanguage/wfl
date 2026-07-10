# Dev Diary — 2026-07-10: Web server gaps for real apps (#597)

## Context

Building the Scriptorium CMS in pure WFL surfaced three concrete web-server
gaps that forced workarounds or blocked features:

1. No query-string access on requests (`?page=2` was unreadable).
2. Hard-coded 1 MiB body cap and no multipart parser (media uploads blocked).
3. `header` / path / method / body only worked in the top-level request loop,
   not inside actions that receive `req`.

## What changed

### Query string

- Warp now captures the raw query via `warp::query::raw()`.
- `WflHttpRequest` carries a `query` field (no leading `?`; empty when absent).
- Exposed as loop-scoped `query` and as `query of req`, ready for
  `parse_query_string of query`.
- `execute file ... with <request>` also injects `query`.

### Configurable body size

- New `.wflcfg` key: `web_server_max_body_size` (default still `1048576`).
- The listen handler uses the config value instead of a hard-coded constant.

### Multipart parsing

- New builtin: `parse_multipart of <body> and <content_type>`.
- Accepts text or binary body; Content-Type supplies the boundary.
- Returns a list of part objects: `name`, `filename`, `content_type`,
  `content`, `content_bytes`.

### Request access inside actions

- `header "X" of req` now resolves headers from the **request object** first
  (then falls back to the loop-scoped `headers` binding).
- `path of req` / `method of req` / `body of req` already worked via property
  access once `req` is passed in; docs now steer handlers to that form.

## Tests

- `tests/web_server_query_and_request_access_test.rs` — query, action access,
  default body-limit rejection.
- `tests/parse_multipart_test.rs` — WFL-level wiring + error path.
- Unit tests in `src/stdlib/web.rs` for boundary extraction and part shape.
- Config tests for `web_server_max_body_size` default and override.

## Docs

- `Docs/04-advanced-features/web-servers.md` — query, multipart, body limit,
  action-scoped request access.
- `Docs/reference/configuration-reference.md` — `web_server_max_body_size`.
