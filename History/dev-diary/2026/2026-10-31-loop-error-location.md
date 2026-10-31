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

The report that started this work used `count from 5 to 1 by -1`. On `dev` (at
`19c5758`) and on `main`, that program does not fail at all. Counting up from 5
to 1 never runs the body, and a loop that never runs is deliberately not
checked, so programs that passed a nonsense step to an empty loop keep working.
The docs say the same thing in `Docs/03-language-basics/loops-and-iteration.md`.
The bug is real for any loop that does run, so the tests use
`count from 1 to 5 by -1`.

The example comes from open PR #800, which adds a diagnostic for exactly that
loop (see also issue #801). Its description lists the `end count` location as a
pre-existing problem for a separate task; this is that task. #800 raises its
diagnostic with the same loop position, so once both changes are in, it points
at the `count` line too.

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

Every consumer wanted "where is this statement". None depended on the old value
being past the end of the loop:

- **Interpreter:** the four count-loop errors (two step errors, the
  stalled-counter error past 2^53, non-numeric bounds), for-each's
  `Cannot iterate over ...`, and `--step` output.
- **Type checker:** `Start/End value in count loop must be a number` and
  `Collection in for-each loop must be a list or map`.
- **Analyzer:** `Nested count loops both use the loop variable ...`, and the
  count loop's variable symbol. That symbol's line is quoted in
  `Variable 'i' has already been defined at line N`.
- **Linter:** `Nesting depth exceeds maximum`. With nested loops the old
  positions even came out in reverse order, because the outer loop ends after
  the inner one.
- **Static analyzer:** the unreachable-code warning.
- **Language server:** the squiggles, through the analyzer and type checker.
- **Fixer:** does not read the position.

## A program can see the change

My first draft said messages were unchanged and no program could observe the
position. An independent review, and a second pass over my correction, showed
both claims were too strong. A loop error raised in the running program and
caught with `try ... when error` binds only the message, with no position. But
some messages quote a position in their text:

- `load module`, `include from` and `execute file` raise an error that embeds
  the loaded file's semantic error text, including `at line N, column M`.
  `load module` does the same for type errors (`include from` only prints them
  as warnings, and `execute file` does not type-check). A program that catches
  the error and displays `error_message` printed `line 4, column 14` for a
  nested-loop clash on line 2; it now prints `line 2, column 5`.
- When a loop fails at run time inside a file run by `execute file`, the error
  is re-raised as `Error in executed file '...' (line N): ...`. That printed
  `(line 4)` for a loop on line 2; it now prints `(line 2)`.
- `Variable 'i' has already been defined at line N` named the line after the
  loop's end.

All of these correct a wrong number; none changes what the program does.
Still, stdout can differ, so the CHANGELOG entry carries a compatibility note,
and the change is classed R3 rather than R2: CLAUDE.md ("Risk class first")
and the backward-compatibility trigger in `testing.md`'s WFL profile (§11.6)
make anything touching backward compatibility R3.

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
- **Message text and `--step`, through the binary** (added after review): the
  redefinition message, a caught `load module` error and a caught
  `execute file` error, each displayed by the program, and the
  `Line N: CountLoop` / `Line N: ForEachLoop` labels in `--step` output.
- **Malformed input:** the parser is an untrusted-input path, and both loop
  parsers now hold their opening token, so a guard test checks that twelve
  cut-off loops (`count`, `count from 1 to 3:`, `for each item in`, ...) are
  parse errors, not panics. It passes with and without the fix; it guards, it
  does not reproduce anything. No fuzz campaign was run; the fuzz workspace
  compiles.

`wfl-lsp/tests/lsp_diagnostics_test.rs` adds a test through
`WflLanguageCore::analyze_document`, the function that produces the editor's
diagnostics.

## Evidence

- Red: `a10961e` adds only tests. On `dev` at `19c5758`, all 13 suite tests and
  the LSP test failed, each with the token after `end` (for example `(3, 10)`,
  `(4, 14)`) or `(0, 0)` at the end of a file.
- Green: `c792dd4`. All 14 pass.
- The four position tests added after review came after the fix, so they do
  not prove TDD order (`testing.md` §6.2); `a10961e` does that for this defect.
  To show they can fail, I reverted `c792dd4` in the working tree only. All 17
  position tests then failed, the new ones with `defined at line 4`,
  `line 4, column 14`, `(line 4)` and `Line 4: CountLoop` /
  `Line 8: ForEachLoop`; the malformed-input guard passed, as it should. With
  the fix restored, 18 of 18 pass.

## Noticed, not changed

- At end of input, `--step`'s `continue (y/n)?` prompt repeats forever instead
  of stopping. The new `--step` test ends its answers with `n` so it cannot
  hang.
- The for-each parse error `Unexpected end of input after 'each'` still uses a
  made-up position of line 0, column 0. It could use `for_token` now, but it is
  a separate parse-error change with its own tests.
