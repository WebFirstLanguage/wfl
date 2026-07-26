# Dev Diary — 2026-07-26: Sunsetting the JavaScript transpiler

The WFL to JavaScript transpiler is retired. This change removes the
`wfl::transpiler` module, the `--transpile` CLI mode and its options, the
transpiler test suite, and the transpiler example program, and replaces the
retired flags with an explicit sunset error.

## Why

The transpiler only ever covered a shrinking subset of WFL. It rejected web
servers, WebSockets, response streaming, and TLS listens outright, and every
new runtime feature since then had to grow a matching "not supported in JS
transpilation" arm just to keep the module compiling. That is a maintenance
tax paid on every language change, in exchange for a JavaScript output path
whose fidelity was never close to the interpreter's. Keeping it advertised in
`wfl --help` also overstated what the tool could actually do, which the docs
honesty policy does not allow.

Retiring it removes ~3,300 lines of production code plus a 722-line test file
and takes the "does the transpiler still compile?" question off the critical
path for future interpreter work.

## Risk and compatibility contract

- **Risk class:** R3 — backward compatibility (a shipped CLI surface is being
  withdrawn).
- **Compatibility contract:** the **WFL language is unchanged**. Every existing
  WFL program still parses, type-checks, and runs identically under the
  interpreter; nothing in `TestPrograms/` depended on transpilation. What is
  withdrawn is the `--transpile` output path and the `wfl::transpiler` library
  API (`JavaScriptTranspiler`, `TranspilerConfig`, `TranspilerTarget`,
  `transpile`, `transpile_default`).
- **Failure mode chosen:** the retired flags are *kept as recognized arguments*
  that exit with code 2 and a message naming the removal and the replacement
  (`wfl <file.wfl>`). Without this, `--transpile` would fall through to the
  generic argument arm, be taken as an input file path, and report
  `No such file or directory` — a confusing failure that hides the real cause.
  Failing loudly and specifically is the deprecation path here: an existing
  build script breaks visibly, with the fix in the error text.
- **External state:** none beyond temporary files created and removed by tests.

## Acceptance criteria and coverage

| Acceptance criterion | Regression coverage |
|---|---|
| `--transpile` exits 2 with a message that names the removal and points at `wfl <file>` | `tests/transpiler_sunset_test.rs::transpile_flag_is_rejected_with_sunset_message` |
| The transpiler-only options `--target`, `--no-runtime`, `--es-modules` are rejected the same way instead of being misparsed | `tests/transpiler_sunset_test.rs::transpiler_sub_options_are_rejected` |
| No `.js` artifact is produced by any invocation, including the full historical `--transpile --target browser --es-modules --output out.js` form | `tests/transpiler_sunset_test.rs::full_transpile_invocation_produces_no_javascript` |
| `wfl --help` no longer advertises transpilation or its options | `tests/transpiler_sunset_test.rs::help_output_does_not_mention_transpilation` |
| Ordinary interpretation of the same program is unaffected | `tests/transpiler_sunset_test.rs::interpreting_the_same_program_still_works` |

The tests drive the real `wfl` binary via `CARGO_BIN_EXE_wfl` and assert on its
actual exit code, streams, and on-disk side effects — no mocking of the CLI
boundary they claim to verify. The absence assertions (no `.js` written, no
`transpil` in help) are the point of the change, so they are written as
negative assertions rather than as "did not crash".

## Red → Green record

The sunset tests were written and run first, on the tree with the transpiler
still present, and committed test-only as `f29e1ca` (an ancestor of the removal
commit).

Observed Red — 4 of 5 failed for the intended reasons:

- `transpile_flag_is_rejected_with_sunset_message`: exit code `0`, not `2` — the
  transpiler ran and reported `Transpiled to: main.js`.
- `transpiler_sub_options_are_rejected`: exit code `1` with
  `Os { code: 2, kind: NotFound, message: "No such file or directory" }` — the
  bare `--target` fell through and was taken as a file path. This is exactly the
  confusing failure the new error arm exists to prevent.
- `full_transpile_invocation_produces_no_javascript`: `out.js` was written.
- `help_output_does_not_mention_transpilation`: help still printed the
  `TRANSPILATION:` block.
- `interpreting_the_same_program_still_works` passed throughout — the intended
  control, confirming the harness itself was sound while the other four failed.

Green after the removal:

```text
cargo test --test transpiler_sunset_test
test result: ok. 5 passed; 0 failed
```

## What was removed

- `src/transpiler/` — `mod.rs`, `javascript.rs`, `runtime.rs`.
- `pub mod transpiler;` from `src/lib.rs`.
- `--transpile` parsing, the transpile execution block, and the `TRANSPILATION:`
  help section from `src/main.rs`.
- `tests/transpiler_test.rs`.
- `TestPrograms/transpiler_example.wfl`.
- The stale "Compile WFL to JavaScript" planned-feature line in
  `Docs/01-introduction/natural-language-philosophy.md`.

Historical records that *mention* the transpiler are deliberately left intact:
earlier Dev Diary entries, `Docs/superpowers/plans/`, and the timestamped
`Docs/rust_loc_report.md` snapshot are dated accounts of what was true when
they were written, and rewriting them would falsify the project's history
rather than document it. Foundation principle 7 (Interoperability with Web
Standards) is unchanged — it is about interoperating with the web platform, not
about this specific transpiler, and planned JavaScript *library* interop in
`Docs/04-advanced-features/interoperability.md` is a separate, still-planned
feature.

## Residual risk

- Anyone depending on the `wfl::transpiler` library API as a downstream crate
  loses it outright; there is no shim. This is intentional — a stub that
  silently produced nothing would be worse than a compile error.
- The `--target`, `--no-runtime`, and `--es-modules` names are now reserved by
  the sunset arm. If a future feature wants one of those names, that arm must be
  narrowed at the same time (a test in the sunset suite will fail loudly, by
  design, rather than the flag quietly changing meaning).
