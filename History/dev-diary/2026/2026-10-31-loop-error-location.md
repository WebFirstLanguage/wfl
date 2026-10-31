# Count and for-each loop errors pointed at `end count`

**Date:** 2026-10-31

## Symptom

```wfl
count from 1 to 5 by -1:
    display count
end count
```

```text
error[ERROR]: Count loop step must be a positive number, got -1. ...
  ┌─ main.wfl:3:10
3 │   end count
  │ ╰^ Runtime error occurred here
```

The step is on line 1, but the error points at line 3, column 10: the end of
`end count`.

The report that started this work used `count from 5 to 1 by -1`. That program
does not fail at all, on `dev` or on `main`. Counting up from 5 to 1 never runs
the body, and a loop that never runs is deliberately not checked, so programs
that passed a nonsense step to an empty loop keep working. The docs say the
same thing in `Docs/03-language-basics/loops-and-iteration.md`. The bug is real
for any loop that does run, so the tests use `count from 1 to 5 by -1`.

## Root cause

`parse_count_loop` and `parse_for_each_loop` in
`src/parser/stmt/control_flow.rs` finished by peeking at the next token,
*after* `end count` / `end for`, and stored that token's line and column in the
statement. Every other block statement (`check`, `if`, `repeat`, `main loop`)
stores its opening keyword. `parse_count_loop` already held the `count` token
in `count_token`; it used it only for parse errors.

When the loop was the last thing in the file with no trailing newline, there
was no next token. The parser then fell back to a made-up position of line 0,
column 0, and the renderer showed that as the start of the file, which is the
wrong line.

The unused, uncompiled `src/parser/mod_complete.rs` already records
`count_token` and `for_token`. The live parser had drifted from it.

## Who used the position

Every consumer wanted "where is this statement". None relied on the old value:

- **Interpreter:** the four count-loop errors (two step errors, the
  stalled-counter error past 2^53, non-numeric bounds), for-each's
  `Cannot iterate over ...`, and `--step` output.
- **Type checker:** `Start/End value in count loop must be a number` and
  `Collection in for-each loop must be a list or map`.
- **Analyzer:** `Nested count loops both use the loop variable ...`.
- **Linter:** `Nesting depth exceeds maximum`. With nested loops the old
  positions even came out in reverse order, because the outer loop ends after
  the inner one.
- **Static analyzer:** the unreachable-code warning.
- **Language server:** the squiggles, through the analyzer and type checker.
- **Fixer:** does not read the position.

## Fix

Both parsers now store the position of the keyword token they consume first:
`count_token` for count loops and a new `for_token` for for-each loops. The
fallback position and its `TokenWithPosition` import are gone.

## Tests

`tests/suite/loop_error_location_test.rs` checks the position at each layer
that shows it:

- **Parser:** both loop kinds, a nested loop, and a loop at the end of a file
  without a trailing newline.
- **Real `wfl` binary:** negative step, zero step inside a `check` block (to
  check the column too), a step error at the end of a file, the stalled
  counter, and the two programs where the type checker warns and the run then
  fails. The test reads the `main.wfl:LINE:COLUMN` that follows each message.
- **Analyzer and linter, through their APIs:** the CLI prints these
  diagnostics without a location, so there is nothing to read there. (Checked:
  the nested count-loop error, an undefined variable, and the linter's nesting
  warning.) That is existing CLI behavior and this change leaves it alone.

`wfl-lsp/tests/lsp_diagnostics_test.rs` adds a test through
`WflLanguageCore::analyze_document`, the function that produces the editor's
diagnostics.

## Evidence

- Red: `a10961e` adds only tests. On `dev` at `19c5758`, all 13 suite tests and
  the LSP test failed, each with the token after `end` (for example `(3, 10)`,
  `(4, 14)`) or `(0, 0)` at the end of a file.
- Green: `c792dd4`. All 14 pass.
