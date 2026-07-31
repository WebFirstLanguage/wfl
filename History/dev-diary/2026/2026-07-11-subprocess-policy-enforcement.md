# Dev Diary — Subprocess policy enforcement (2026-07-11)

## Summary

Closed a critical sandbox bypass: subprocess security policy only ran when the
interpreter thought a *shell* was required. Direct `Command::new` paths (non-empty
`with arguments`, or command strings without shell metacharacters) never consulted
`shell_execution_mode` or `allow_shell_execution`.

## What changed

- Single gate: `CommandSanitizer::authorize_process_execution`, called from both
  `execute_command` and `spawn_process` before any `Command::new`.
- `allow_shell_execution = false` (default) hard-denies all process execution.
- `shell_execution_mode = forbidden` (default) also denies all process execution
  when the master switch is on; `allowlist_only` / `sanitized` / `unrestricted`
  apply to every launch path.
- Docs, README, CHANGELOG, and TestPrograms local `.wflcfg` updated for the
  intentional secure-by-default behavior change.

## Migration

Programs that intentionally run external tools must set `.wflcfg`:

```ini
allow_shell_execution = true
shell_execution_mode = sanitized
```

Prefer `allowlist_only` on hosts that handle untrusted code.

## Out of scope (follow-ups)

- Main-loop wall-clock timeout exemption as a capability
- Release-build recursion cap (today `debug_assert!`)
- Restricting sqlx network drivers for playground/restricted builds
