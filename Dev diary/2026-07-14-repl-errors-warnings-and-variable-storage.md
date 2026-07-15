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

The implementation change is entirely in `src/repl.rs`; the REPL guide
(`Docs/02-getting-started/repl-guide.md`) and this diary are updated alongside it.

- **Session-aware static analysis (accumulated program).** The REPL keeps one
  persistent interpreter but the analyzer/type-checker are whole-program tools;
  running them on a single line makes earlier-line names look "undefined". The
  fix runs the *same* `analyze → type-check → execute` pipeline `wfl <file>`
  uses, but over `session_inputs + this submission` as one program, then reports
  only the diagnostics that fall inside the new submission (their line numbers
  translated back to the submission's own coordinates). Only the new submission
  is executed — earlier ones already ran against the persistent interpreter.
  - A name/action defined on an earlier line is in scope, so cross-line
    references resolve (fixes the core bug) — but the checks the interpreter
    *cannot* do still run: the insecure-RNG `ANALYZE-SECURITY` lint (even when the
    submission references an earlier-line variable — previously the analyzer bailed
    on the "undefined" session symbol before the lint ran), undefined references
    inside a not-yet-called action body (caught at definition time, not deferred
    to invocation), and cross-line type mismatches (e.g. `store n as 5` then
    `change n to "text"`).
  - Semantic **errors are fatal** (they abort a file too); **type diagnostics are
    advisory** — shown but non-blocking, exactly as `wfl <file>` treats them.
  - Two file-only diagnostics are intentionally suppressed in the REPL:
    `ANALYZE-UNUSED` (a binding is available to a *future* submission) and
    "already defined" (re-`store`/re-defining a name is normal interactively — the
    interpreter reassigns).
  - *Design history:* the first version of this PR disabled the static gates and
    relied on the interpreter alone. Maintainer review showed that dropped three
    real safeguards (the security lint, definition-time reference checks, and
    cross-line type contracts). This accumulated-program approach restores them
    while keeping cross-line references working.

- **Session-level security gate (owns `ANALYZE-SECURITY`).** The insecure-RNG
  seeding control is *not* left to the accumulated-source analysis, because that
  analysis has two blind spots the maintainer's second review flagged:
  1. It reports only diagnostics whose line falls inside the *new* submission, so
     an `ANALYZE-SECURITY` attached to a `random_seed` on an *earlier* line was
     filtered out — seeding in one command and doing crypto in a separate later
     command slipped through.
  2. When the accumulated source exceeds the size cap, or the combined source
     fails to lex/parse, analysis falls back to the current submission alone,
     which returned "advisory, never blocks" — downgrading even self-contained
     fatal errors, so a long session could execute what a short one would block.
  The fix tracks the two ingredients of the lint — an insecure `random_seed` and
  any [security-sensitive builtin](../src/analyzer/static_analyzer.rs) — as
  session state (`rng_insecurely_seeded`, `security_builtin_used`), folded in
  after each executed submission. Before running any submission, the REPL blocks
  when the session (state + this submission) has *both* ingredients. This runs on
  every path, independent of the capped combined analysis, so:
  - Seeding then crypto in **separate** submissions is blocked (either order); the
    completing submission is fatal, so it is not recorded, and the session never
    accumulates both ingredients — a later *innocent* command is not over-blocked.
  - The **fallback** path (over-cap or lex/parse failure) still blocks the
    combination: the security control is a separate pass, not a source-size mode
    switch. Only context-dependent name-resolution stays advisory in the fallback
    (it genuinely can't be judged without the earlier lines).
  - Seeding **without** any security-sensitive builtin remains allowed
    (reproducible simulations), matching the file lint.
  The scan (`rng_security_ingredients`) reuses the analyzer's own call collector
  and `SECURITY_SENSITIVE_BUILTINS` list, so REPL and `wfl <file>` agree on what
  counts as seeding and as security-sensitive.

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
- `undefined_variable_is_reported`, `syntax_error_is_reported` — genuine mistakes
  are still surfaced.
- `expression_result_is_echoed`, `boolean_echo_uses_yes_no_not_true_false` — echo
  format.
- `incomplete_blocks_request_more_input`,
  `complete_statements_are_not_treated_as_incomplete`,
  `multiline_if_block_buffers_across_lines_and_runs_on_close` — multi-line
  detection across block forms.
- `render_diagnostic_snaps_span_to_char_boundaries`,
  `render_diagnostic_points_at_source_even_at_eof` — the synthesized caret span
  stays on UTF-8 boundaries and still points at the source at EOF.

Session-aware analysis (from the maintainer review):

- `insecure_rng_is_blocked_even_when_referencing_an_earlier_line_variable`,
  `self_contained_security_lint_still_blocks` — the `ANALYZE-SECURITY` lint fires
  and blocks, including when the submission references an earlier-line variable.
- `undefined_reference_inside_an_action_body_is_blocked_at_definition` — a typo
  inside a deferred action body is caught when defined, not at invocation.
- `earlier_session_variable_is_usable_inside_a_definition`,
  `self_recursive_action_is_accepted` — valid earlier-session and self-references
  resolve.
- `cross_line_type_mismatch_is_surfaced` — an incompatible assignment against an
  earlier-line variable is reported (advisory).
- `re_storing_a_variable_is_allowed` — re-`store` is not blocked by the file-only
  "already defined" error.

Session-level security gate (second maintainer review):

- `insecure_seed_then_crypto_in_separate_submissions_is_blocked`,
  `crypto_then_insecure_seed_in_separate_submissions_is_blocked` — the seed +
  crypto combination is blocked when split across separate submissions, in either
  order (the whole-program lint could not see this).
- `innocent_command_after_a_blocked_crypto_op_is_not_over_blocked` — the blocked
  (completing) submission is not recorded, so a later unrelated command still runs.
- `insecure_seed_alone_is_still_allowed` — seeding without any crypto/auth op is
  not blocked.
- `fallback_path_still_blocks_insecure_seeding` — forcing the isolated fallback
  (tiny `max_session_analysis_bytes`) still blocks the combination, same-submission
  and cross-submission; the performance fallback is not a security mode switch.
- `fallback_does_not_block_reference_to_a_live_earlier_session_binding` — in the same
  forced fallback, a reference to a live earlier-session variable is dropped (no
  error, not blocked), so cross-line references keep working in the degraded path.
- `fallback_blocks_undefined_reference_inside_a_deferred_action_body` — a genuine
  typo inside a not-yet-called action body is still fatal at definition time in the
  fallback (the name is absent from the live session).
- `fallback_allows_action_referencing_a_valid_earlier_session_binding` — the
  companion: an action body referencing a live earlier-session binding is accepted.
- `rng_security_ingredients_reports_each_half_independently` (analyzer) — the
  ingredient scan reports seeding and security-builtin use independently and
  captures call sites.

The isolated fallback (`static_check_isolated`) blocks per diagnostic rather than
relaxing wholesale: **self-contained fatal errors still block**. For an
**undefined-name** diagnostic (variable/action/handler/list reference) the fallback
consults the **live interpreter environment** — the source of truth for what the
session actually holds — to decide:

- Name **is** a live earlier-session binding → the "undefined" is a false positive
  from losing the prefix; drop it (no noise, no block), so cross-line references
  keep working in the degraded path.
- Name is **not** in the session → a genuine undefined reference; keep it fatal.

The second case is what a maintainer review (head `52c16b2`) required: defining an
action stores its body *without running it*, so a genuine typo inside a not-yet-called
action/handler body is not validated by execution. An earlier version downgraded
*all* undefined-name diagnostics to advisory, which let a broken deferred definition
through at the cap boundary — re-opening the "caught at definition time" invariant
specifically after the fallback. Keying the decision on live-env membership (rather
than diagnostic position) preserves that invariant, avoids false blocks for valid
earlier-session references, and keeps the fallback consistent with the full-analysis
path (which likewise treats a name absent from the session as a genuine error).

Type checking is intentionally skipped in the fallback: its diagnostics are advisory
(never block), and without the earlier-session symbols it would emit false-positive
"undefined" type diagnostics for valid earlier-session references. The insecure-RNG
control is enforced regardless by the session gate above.

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
