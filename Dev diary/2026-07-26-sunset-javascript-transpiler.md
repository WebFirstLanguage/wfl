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

## Governance: the §3.1 deprecation window

Review raised `GOVERNANCE.md` §3.1, which requires announcing an unavoidable
break ≥ 1 year in advance and keeping the old behavior working until that
deadline. The objection is worth recording rather than waving past.

§3.1 opens with "Never break existing **WFL programs** without a documented
path," and that is the surface it protects. No WFL program is affected here: the
language, its semantics, the analyzer, the type checker, and the interpreter are
untouched, and all 110 runnable programs in `TestPrograms/` pass unchanged
against the release build. What is withdrawn is build tooling (the `--transpile`
CLI mode) and a library module (`wfl::transpiler`) — neither of which any WFL
source file can depend on.

Per §2.2, "Language design / breaking change" is a Maintainer decision. The
Maintainer directed this sunset and, when the objection was put to them
explicitly, chose immediate removal with the rationale recorded rather than a
deferred window. That decision is logged in `CHANGELOG.md` under Removed so it
is auditable.

The residual cost is real and stated plainly: a downstream crate importing
`wfl::transpiler` loses it without a shim, and a build script calling
`--transpile` breaks — loudly and with the fix in the error text, but it does
break. That trade was made deliberately, not overlooked.

## Review follow-ups

Two issues surfaced in review of this change:

- **`assert_no_js_output` only searched the top level.** Its doc comment claimed
  "anywhere under `dir`" while the body did a single non-recursive `read_dir`, so
  a nested artifact like `nested/out.js` would have slipped through and the
  absence assertion would have passed vacuously. The helper now walks
  subdirectories, `run_cli` creates a writable `nested/` for the binary to emit
  into, and the full historical invocation targets `--output nested/out.js` so
  the recursion is actually exercised rather than being untested safety code.
  Verified by planting a `nested/planted.js` and confirming the assertion fires.

- **`wfl -h` and `wfl -V` were mistaken for input file paths.** This is the same
  root cause as the transpiler flags: `main()` already classifies `-h`/`-V` as
  trivial, non-interpreting invocations (they skip the large-stack reservation),
  but the parser in `run()` only recognized `--help`, `--version`, and `-v`. Both
  aliases fell through to the generic argument arm and failed with
  `No such file or directory` — the exact confusing failure the sunset arm exists
  to prevent. Fixed in `src/main.rs` and pinned by
  `tests/cli_help_version_flags_test.rs`, which was observed Red (3 of 3 failing
  on `-h`/`-V`) before the fix. Pre-existing, unrelated to the transpiler, but
  fixed here because it is the same misparse the sunset work was about.

A second review round added three more:

- **The `.js` check was case-sensitive.** `out.JS` is just as much a leaked
  artifact, and on a case-insensitive filesystem it is the same file. Now
  compared with `eq_ignore_ascii_case`; verified by planting `nested/LEAK.JS` and
  watching the assertion fire.

- **`print_help()` didn't document `-h`/`-V`.** An alias that works but is
  undocumented is undiscoverable, so the help text now lists `--help, -h` and
  `--version, -v, -V`, pinned by `help_text_documents_the_short_aliases`.

- **The sunset error said nothing about `--output`.** `--output` outlived the
  transpiler (it still serves `--dump-env`), so a former user had no way to tell
  whether it still produced JavaScript. The error now says explicitly that it
  does not.

One review finding was **refuted rather than fixed**: a P1 claimed the Red commit
was not an ancestor of the removal, citing `git merge-base --is-ancestor f29e1ca
911f20e`. `911f20e` does not exist in this repository, and
`git merge-base --is-ancestor f29e1ca HEAD` succeeds — the branch is linear
(`f29e1ca` → `2327776` → …). Replied on the PR with the evidence; no change made.
