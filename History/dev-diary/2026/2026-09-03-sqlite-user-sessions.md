# Dev Diary — 2026-09-03 — SQLite-backed user sessions (issue #555)

## Summary

WFL programs can now keep HTTP sessions with the natural-language surface
from issue #555. `listen … with sessions enabled` builds a runtime
`SessionManager`; `configure` / `enable` override `.wflcfg` defaults; handlers
create, read, update, and destroy sessions and attach `Set-Cookie` on
`respond`. Three stores ship: in-memory, a JSON file, and SQLite (`database`).

```wfl
listen on port 8080 as web_server with sessions enabled
configure sessions on web_server with timeout 1800000 and storage "database"
enable csrf protection on web_server

main loop:
    wait for request comes in on web_server as req
    store sess as get session from req
    check if sess is nothing:
        store sess as create session for req
        set session value "user_id" to "guest" in sess
        respond to req with "ok" and set session sess
    otherwise:
        store user_id as get session value "user_id" from sess
        respond to req with user_id
    end check
end loop
```

## Design decisions

- **Language statements, not a stdlib native.** Natives are
  `fn(Vec<Value>) -> Result<Value, RuntimeError>` with no interpreter, request,
  or sqlx access — they cannot persist sessions or set cookies.
- **No new lexer keywords.** `session` / `sessions` are positional markers, so
  existing `store session as "active"` programs keep working. Keyword count
  stays 181.
- **User data is not dumped onto the session object.** `id of sess` uses the
  existing `property of object` form; values live behind
  `get` / `set session value` so they cannot collide with `id`.
- **Timeouts are milliseconds.** `configure sessions … with timeout 1800000`
  is 30 minutes; `session_timeout_ms` matches that unit.
- **CSRF is explicit.** `generate csrf token for session` stores a hex token
  on that session. `enable csrf protection` records the flag. v1 does not
  auto-reject requests — that would change handler semantics without a test
  that requires it. The #555 e2e program checks `X-CSRF-Token` in user code.
- **No session secret in `.wflcfg`.** IDs and tokens come from the OS CSPRNG
  (same source as `secure_random_bytes`).

## Storage

SQLite uses the in-tree sqlx dependency (`sqlite://` + `create_if_missing`)
with `wfl_sessions` and `wfl_session_kv`. File storage writes a temp file and
renames. Memory and file share one `tokio::sync::Mutex`; the database backend
uses the sqlx pool. Last write wins per session id. `create session` fails
cleanly when `session_max_sessions` is reached.

## Testing

Red commit `5a1a49e` added parser and store tests that failed for the intended
reasons (unknown AST variants / missing `wfl::interpreter::sessions`). Green
commits implement the surface. `scripts/run_web_tests.sh` drives
`TestPrograms/web_server_session_test.wfl` with `curl -c/-b` (login cookie,
profile, CSRF, logout clear, stats, storage KV).

Issue #555 stays open: WebSockets already shipped; keyword-reference web
examples are a separate leftover.
