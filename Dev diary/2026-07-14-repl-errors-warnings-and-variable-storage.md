# Dev Diary — 2026-07-14: REPL error rules, warnings, and the "store doesn't store" bug

## Context

A user reported that the interactive REPL (`wfl` with no file) misbehaved when
storing a variable from the command line: it printed a warning and then the
variable did not appear to be stored at all. The ask was to (1) make the REPL's
errors and warnings follow WFL's error rules, (2) fix the storage bug, and
(3) exercise a range of programs and lines so the REPL works reliably.

## Root causes

Everything traced back to one design mismatch: **the REPL keeps a single
persistent interpreter for the whole session, but it ran the lexer, parser,
analyzer, and type checker fresh on every line.** Those front-end passes are
whole-program tools; against a single interactive line they produce false
positives about the live session.

1. **`store x as 5` was discarded by its own warning.** The per-line analyzer
   flagged the just-declared variable as an "unused variable" *warning*, and
   `process_complete_input` returned early on **any** non-empty diagnostic list
   — warning or error alike. So the command was reported and then dropped before
   the interpreter ever ran. The variable was never stored. (The `wfl <file>`
   path already got this right: it only aborts on `Severity::Error`.)

2. **A variable/action from an earlier line looked "not defined".** Because the
   analyzer and type checker saw only the current line, `display x` on the line
   after `store x as 5` was a fatal "Variable 'x' is not defined" — even though
   `x` was alive in the session. The same happened to `call greet with "..."`
   after an action definition ("'greet' is not an action").

3. **Multi-line blocks silently broke.** `is_input_incomplete` matched parser
   error strings case-sensitively (`contains("expected")`). `check if …:` reports
   "Expected 'end' after if block, found end of input" and `define action …:`
   reports a bare "Expected 'end' after action body" — neither matched, so those
   blocks were treated as complete and errored instead of waiting for their
   `end`. (Count loops happened to work only because "unexpected" contains the
   substring "expected".)

4. **Analyzer diagnostics had no source snippet.** Parser/type/runtime errors
   rendered with the Elm-style caret; analyzer diagnostics (built with no label)
   printed as a bare message with no location.

## What changed

All changes are in `src/repl.rs` (plus a tiny cleanup in `src/interpreter/mod.rs`).

- **The interpreter is the source of truth.** The REPL no longer lets the
  stateless analyzer/type-checker gate execution. Static analysis is kept only
  for advisory **warnings** that are self-contained within one line (unreachable
  code, shadowing, insecure RNG seeding, inconsistent returns). Analyzer
  *errors* (undefined name, "not an action", already-defined) are context
  dependent, so they are not reported here — the interpreter re-checks them
  against the real session environment and reports genuine ones at run time with
  the standard formatting. The type checker is skipped for the same reason. This
  fixes causes #1 and #2 together: `store` stores, and later lines can use what
  earlier lines defined.

- **"Unused variable" warnings are suppressed in the REPL.** A stored binding is
  available to the next line, so "unused" is always a false positive
  interactively.

- **Robust multi-line detection.** `error_means_more_input_needed` replaces the
  fragile string match with two case-insensitive signals: the parser ran out of
  tokens (`end of input`), or it is still waiting for a block terminator
  (`expected 'end`) without having stopped on a concrete token. This covers
  `check if`, `count`, `for each`, `repeat while/until`, and `define action`
  blocks; a real mid-stream mistake (`… found <Token>`) is still surfaced
  immediately.

- **Every diagnostic gets a caret.** `render_diagnostic` synthesizes a
  one-character span from an analyzer diagnostic's line/column when it has no
  label, so all REPL diagnostics follow the same "point at the source"
  convention (WFL Fundamental #4).

- **Cleaner value echo.** A bare expression echoes its value with `Display`
  (`yes`/`no`, unquoted text, `[1, 2, 3]`) instead of Rust's debug spelling, and
  void results (`nothing`/null) are not echoed — so `call greet with "World"`
  prints its output with no stray `null` underneath.

- The four near-identical codespan rendering blocks were collapsed into the one
  `render_diagnostic` helper.

## Tests

Added regression tests in `src/repl.rs`:

- `store_persists_and_prints_nothing` — the reported bug: `store` produces no
  output and the variable is actually in the session env.
- `variable_defined_on_an_earlier_line_is_usable_later`,
  `action_defined_earlier_can_be_called_later_without_null_echo` — cross-line
  references and calls work, and void calls don't echo `null`.
- `undefined_variable_is_reported_as_a_runtime_error`, `syntax_error_is_reported`
  — genuine mistakes are still surfaced.
- `expression_result_is_echoed`, `boolean_echo_uses_yes_no_not_true_false` — echo
  format.
- `incomplete_blocks_request_more_input`,
  `complete_statements_are_not_treated_as_incomplete`,
  `multiline_if_block_runs_only_once_closed` — multi-line detection across block
  forms.

`cargo test --lib`, `cargo fmt --all -- --check`, and
`cargo clippy --all-targets --all-features -- -D warnings` are green.

## Docs

`Docs/02-getting-started/repl-guide.md` had several claims that never matched the
implementation; corrected them while here (Docs-Must-Be-Honest):

- Startup banner and prompt (`WFL REPL - Type .help for commands or .exit to
  quit`, `wfl>` prompt, `...` continuation) instead of a made-up version banner
  and `>`.
- REPL commands are dot-prefixed: `.help`, `.history`, `.clear`, `.exit` — there
  is no `exit`/`quit`/`clear`.
- Replaced a fictional "Cannot add Number and Text" error example (WFL actually
  coerces `5 plus "hello"` to `"5hello"`) with a real undefined-variable error.
- Removed the stale "No history (yet)" limitation — `.history` and arrow-key
  recall both exist.

## Follow-ups (not done here)

- Several tutorial examples elsewhere in `repl-guide.md` omit `call` before an
  action name (`calculate area with 10 and 20`), which parses as concatenation,
  not a call. That's a general action-call-syntax documentation issue, not a
  REPL behavior issue, so it was left out of this change.
