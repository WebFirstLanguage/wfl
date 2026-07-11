# Dev Diary — 2026-07-10: Pass loaded `.wflcfg` to the interpreter (#466)

## Context

Deploying a WFL web server to a public host surfaced a long-standing bug:
`web_server_bind_address` in `.wflcfg` was silently ignored. No matter what an
operator configured, `listen on port N` always bound `127.0.0.1`, so WFL web
servers were effectively loopback-only in any real deployment that relied on
the config file to open the bind address.

The config file *was* parsed correctly — the value simply never reached the
interpreter. In the script-run path, `src/main.rs` built the interpreter with:

```rust
let mut interpreter = Interpreter::with_timeout(config.timeout_seconds);
```

`with_timeout` constructs a **fresh** `WflConfig` with only the timeout copied
across; every other field — including `web_server_bind_address`,
`web_server_max_body_size`, and the logging settings — fell back to
`WflConfig::default()`. So `.wflcfg` was loaded, then thrown away.

This was originally reported and patched in PR #466, which was closed without
merging. This change applies the fix and adds an end-to-end regression test.

## What changed

### `src/main.rs`

```rust
- let mut interpreter = Interpreter::with_timeout(config.timeout_seconds);
+ let mut interpreter =
+     Interpreter::with_config(std::sync::Arc::new(config.clone()));
```

`Interpreter::with_config(Arc<WflConfig>)` already existed and already reads the
full config (bind address, body size, timeout, io_client settings); it just
wasn't being called from the script-run path. The `.clone()` is required because
`config` is read again later in `main.rs` (e.g. `if config.logging_enabled`) —
without it the move fails to compile (`E0382`).

`with_timeout` is unchanged and still used by the REPL and tests, so there is no
API surface change.

### Preserving the 300-second timeout cap

`with_timeout` clamped the execution timeout to 300 seconds
(`if seconds > 300 { 300 } else { seconds }`). To keep this change *purely
additive* — i.e. new config fields propagate, but nothing else about execution
changes — the script-run path re-applies that same cap before building the
interpreter:

```rust
let mut run_config = config.clone();
run_config.timeout_seconds = run_config.timeout_seconds.min(300);
let mut interpreter = Interpreter::with_config(std::sync::Arc::new(run_config));
```

Note this cap never affected web servers in the first place: `check_time` skips
the timeout entirely while `in_main_loop` is set (see
`src/interpreter/mod.rs`), and web servers run inside a `main loop`. The cap only
bounds ordinary, non-main-loop scripts, exactly as before.

## Tests

New `tests/web_server_bind_address_cli_test.rs` drives the **compiled binary**
against a real `.wflcfg` and asserts the configured bind address reaches the
listening socket, inspecting `/proc/net/tcp` (Linux-gated):

- `.wflcfg` `= 127.0.0.1` → socket bound to `0100007F` (loopback).
- `.wflcfg` `= 0.0.0.0`   → socket bound to `00000000` (all interfaces).

Verified this test **fails on the pre-fix code** (the `0.0.0.0` case bound
`0100007F`, reproducing the bug) and passes after the fix. A portable
`wflcfg_server_is_reachable` smoke test also confirms the server comes up and is
connectable when launched via the binary with a `.wflcfg`.

The existing interpreter-level suite (`tests/web_server_bind_address_test.rs`,
which exercises `with_config` directly) continues to pass.

## Notes

- No docs change to the config option itself: `web_server_bind_address` is
  already documented in `Docs/reference/configuration-reference.md` and
  `Docs/04-advanced-features/web-servers.md`. This fix makes the documented
  behavior actually happen.
- Backward compatible: programs that never set `web_server_bind_address` keep
  the `127.0.0.1` default; only configs that explicitly change it are affected —
  which is the intended, previously-broken behavior.
