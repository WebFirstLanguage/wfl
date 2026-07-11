# Dev Diary — 2026-07-11: Expose the interpreter version to WFL programs (#602)

## Context

A WFL program had no way to ask **which interpreter version is running it**. The
only workaround was to shell out to `wfl --version` via `execute command` and
string-parse the banner. That has three problems:

1. It probes whatever `wfl` happens to be on `PATH`, not the binary actually
   executing the program — so a full-path invocation like `/opt/wfl/bin/wfl
   main.wfl`, or a machine with multiple installed versions, can report the
   *wrong* version.
2. It costs a subprocess for a value the interpreter already holds as a compiled
   constant (`wfl::version::VERSION` in `src/version.rs`).
3. It requires the `execute command` capability just to read a version string.

The motivating consumer is **wfl-web** (the WFL website, itself written in WFL),
which displays the language version in its hero eyebrow and had already drifted
(`v26.1` shown while the interpreter was `26.7.x`).

## What changed

WFL now predefines a global immutable constant, `wfl_version`, that resolves to
the bare semver text of the running interpreter (for example `"26.7.28"`).

```wfl
store v as wfl_version
display "Running on WFL " with v   // Running on WFL 26.7.28
```

### Why a constant, not a builtin function

The issue floated two shapes: a contextual-keyword expression (`wfl version`) or
a zero-arg `of`-form builtin (`version of interpreter`). Both were rejected in
favor of a plain global constant, mirroring the existing `newline`/`tab`
constants:

- A **new keyword** (`wfl`) would push the reserved-keyword count up and risk
  breaking any program that already uses `wfl` as an identifier — a
  backward-compatibility hazard for zero real benefit.
- A **bare-name zero-arg builtin** (like `random`) hits the include-file wrinkle
  noted in #592: zero-arg `NativeFunction` values are deliberately *skipped*
  when importing an included file's scope (see `src/interpreter/mod.rs`, the
  `matches!(value, Value::NativeFunction(_, _))` guard added for #557), so a
  bareword builtin would not resolve cleanly inside includes.
- A **constant** (`Value::Text`) sidesteps all of it: it is not a
  `NativeFunction`, so it flows into included-file scopes like any other value;
  it needs no arity handling or auto-call machinery; and the bare semver text is
  the most composable form (programs that want a full banner build it with
  `with`).

This keeps the beginner form and the expert form identical — a value you read,
nothing to unlearn.

### Files touched

- `src/stdlib/core.rs` — `register_core` now defines `wfl_version` as
  `Value::Text(crate::version::VERSION.into())`, right beside `newline`/`tab`.
- `src/analyzer/mod.rs` — registers `wfl_version` as an immutable `Text` symbol
  in the global scope so it is not flagged "not defined" (same treatment as
  `newline`/`tab`). The type checker delegates undefined-variable checks to the
  analyzer, so no separate type-checker registration is needed.
- `Docs/05-standard-library/core-module.md` — new "Constants" section
  documenting `wfl_version` (and, filling a prior gap, `newline`/`tab`).
- `TestPrograms/wfl_version_test.wfl` — end-to-end example that reads the
  constant, checks its type, and composes it into a banner.
- `tests/wfl_version_builtin_test.rs` — regression tests asserting the value
  equals `wfl::version::VERSION`, is `Text`, and composes in text expressions.

## Testing

- Wrote the failing integration tests first (TDD): all three failed with an
  analyzer "not defined" error before the change.
- After the change, `cargo test --test wfl_version_builtin_test` is green, and
  the value matches the compiled-in `VERSION` constant exactly.
- Verified the constant resolves inside `include from` files as well as the main
  program (the #592 wrinkle that a bareword builtin would have hit).
- `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`,
  and the full `cargo test` lib/integration suite pass (the `binary_io_test`
  failures seen without a release build are pre-existing and environmental —
  that suite hardcodes `target/release/wfl`).

## Follow-ups

- Once released, wfl-web can drop its `execute command "wfl --version"` probe
  (`lib/site.wfl`, `wfl_runtime_version`) and read `wfl_version` directly.
- A structured form (`major`/`minor`/`build` map) was floated in the issue as a
  "welcome but not necessary for v1" nicety; deferred, since the bare text plus
  `split` covers it.
