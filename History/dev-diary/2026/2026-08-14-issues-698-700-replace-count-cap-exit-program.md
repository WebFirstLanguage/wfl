# 2026-08-14 — replace, the count-loop cap, and `exit program` (#698, #699, #700)

## What

Three long-standing bugs, all found in one sitting while porting a 738-line PHP
JavaScript minifier to WFL, all fixed together:

- **#698** — `replace <pattern> with <text> in <text>` validated its three
  arguments and then returned the input unchanged. It parsed, type-checked,
  ran, exited 0, and produced a wrong answer with no diagnostic. Compounding
  it, the literal 3-argument `replace` in `src/stdlib/text.rs` was unreachable
  from WFL source (`replace` lexes as a keyword), so the language shipped with
  **no reachable string replacement at all**.
- **#699** — the `count` loop's trip guard was keyed on the loop's *end value*
  rather than on the trip count: `end_num > 1_000_000` meant "no limit",
  anything smaller meant "10001 trips maximum". So `count from 1 to 20000`
  aborted while `count from 1 to 1000001` ran uncapped, and
  `count from 2000000 down to 1` was refused because the value it counted
  *down to* was small.
- **#700** — `exit program`, the spelling in both keyword reference pages, did
  not parse (`program` fell through as an operand: "Variable 'program' is not
  defined", exit 3). Bare `exit` parsed but was ignored at top level, so
  program termination had no working spelling.

## Why they mattered

All three fail in the way that costs the most to debug. #698 and #700 are
silent: a correct-looking program gets a wrong answer or skips a stop, and the
exit status says everything is fine. #699 is loud but backwards — it rejects
ordinary small loops, allows the enormous ones, and names a limit that appears
nowhere in the documentation.

## How they were fixed

**#698 — `src/stdlib/pattern.rs`.** `native_pattern_replace` now actually
replaces: `find_all_with_budget` gives non-overlapping matches in ascending
order, and the rebuild walks *characters* (the VM reports character offsets, so
slicing on byte offsets would panic on multibyte input) in a single forward
pass. The needle may now also be a plain `Value::Text`, matched verbatim, which
is what makes literal string replacement reachable: `replace "world" with
"there" in s`. The type checker accepts `Pattern` or `Text` in that position.
Beginner form and expert form are the same statement — the no-unlearning
invariant applied to a gap that previously had no beginner form at all.

Capture-group backreferences in the replacement (`$1`) remain unimplemented;
that is a separate enhancement, and the docs now say so rather than implying
otherwise.

**#699 — `src/interpreter/mod.rs`.** The end-value-keyed cap is gone. `count`
is now bounded by the same execution timeout as `repeat` and `for each`, which
`check_time()` already enforces on every trip — the cap was never what stopped
a runaway loop (`count from 1 to 100000000000` was already killed by the 60 s
timeout, not by the guard). What replaced it is a much narrower check that
catches the case the cap was accidentally covering: a step that cannot move the
counter toward the end value (`by 0`, or a negative step) is refused up front
with a diagnostic that names the step, instead of spinning for 60 seconds.

**#700 — parser, AST, interpreter.** `ExitStatement` carries an `ExitScope`
now: `exit` / `exit loop` keep exactly their old meaning (leave every enclosing
loop), and `exit program` is the new form. It is raised as an
`ErrorKind::ExitProgram` sentinel rather than a `ControlFlow` variant, because
it has to unwind action calls and expression evaluation, neither of which
carries a control-flow channel. The sentinel is deliberately not catchable:
`when error:` re-raises it (a `finally:` block still runs), include/module
frames pass it through unwrapped, the concurrent `main loop` propagates it
instead of treating it as a handler failure, and the top of the run turns it
back into a successful finish — remaining top-level statements and the
conventional `main` action are skipped, streams and pending requests are still
finalized, and the process exits 0.

Bare `exit` at top level is still a no-op, the same as `break` there. That is a
deliberate compatibility choice: making the loop-exit signal terminate the
program would silently change what existing programs do after a loop. The two
reference pages now document the split (`exit loop` vs `exit program`) instead
of a form that errored.

## Testing

Red first: `tests/issues_698_700_test.rs` was committed as a test-only commit
in which 16 of its 19 tests fail against unmodified source, each for the
reason its issue describes. The three that already passed pin behaviour the fix
must not change — a non-matching pattern leaves its text alone, a
non-text/non-pattern needle is refused, and bare `exit`/`exit loop` still
leaves the loop and lets the program continue.

The tests run the real binary, so exit status is part of what is asserted —
which is the whole point for #698 and #700, where the old failure was a wrong
answer with status 0.

End-to-end WFL coverage lives in `TestPrograms/replace_and_count_loop_test.wfl`
(10 `test` blocks, 11 `expect` assertions) and
`TestPrograms/exit_program_test.wfl`, which stops halfway through and relies on
the gated runner's "exit 0" to assert that a clean stop is a successful stop.
The statements after the stop end in a read of a file that cannot exist, so
unexpected continuation fails the run rather than merely printing a line.

## Docs shipped with the change

- `Docs/04-advanced-features/pattern-matching.md` — new "Replacing Matches".
- `Docs/05-standard-library/pattern-module.md` — `pattern_replace`.
- `Docs/05-standard-library/text-module.md` — `replace` (literal form).
- `Docs/03-language-basics/control-flow.md` — new "Stopping the Program",
  with the `break` / `exit loop` / `exit program` table.
- `Docs/03-language-basics/loops-and-iteration.md` — the step must be
  positive; a count loop has no trip limit; `exit loop` documented next to
  `break`, and the "(if supported)" hedges removed now that both are pinned by
  tests.
- Both keyword reference pages updated together.

## What review caught

Automated review of the fix found five more defects in it, all of the same
family — a change that is right in the common path and wrong at a boundary
nobody walked:

- **Two more places swallowed the stop sentinel.** The websocket event
  dispatcher printed every handler error and returned `Ok(())`, so
  `exit program` inside `on websocket message:` printed
  `WebSocket message handler error: [Exit program]` and the pump carried on.
  A `test` block records any non-assertion error as a failure, so stopping
  inside a test was reported as a failing test and the run continued with a
  nonzero status — the exact opposite of "a clean stop is a successful stop".
  Both now use the same `is_exit_program()` pass-through as the other
  boundaries. The lesson is that adding an out-of-band signal means auditing
  *every* place that absorbs a `Result`, not just the ones the feature's own
  tests walk.
- **The step check broke a working program.** Validating the step
  unconditionally turned `count from 5 to 1 by 0` — an empty range whose body
  never ran, and which therefore never cared about its step — into a runtime
  error. The check now runs only when the loop is actually entered. A fix for
  a loud bug quietly became a compatibility break, which is precisely what the
  backward-compatibility rule exists to catch.
- **A positive step still is not necessarily a step.** Past 2^53 the counter's
  floating-point resolution exceeds the step, so `count + step == count` and
  the loop never advances. Removing the trip cap made that endless rather than
  merely slow, and inside a `main loop` — where the deadline is suspended —
  nothing would have stopped it. The loop now refuses to continue the moment
  the counter provably stops moving.
- **A diagnostic contradicted itself.** `TypeError` appends
  "Expected X but found Y" whenever both fields are set, so accepting two types
  while naming one produced "Expected Pattern or Text to replace, got Number -
  Expected Pattern but found Number". With two valid types there is no single
  expectation to report, so that field is now left unset.

Not fixed, deliberately: count loops inside a `main loop` remain unbounded.
The deadline exemption is intentional (a server must not time out on its own
uptime) and applies to every loop form, so `repeat forever` and a `for each`
over a caller-supplied list are equally unbounded there. The removed cap was
never a control against this — it only bit loops whose end value was *below*
1,000,000, so any caller wanting an unbounded loop just passed a larger number.
The real fix is a per-handler deadline, which is a design change rather than a
bug fix; the docs now state the exemption plainly instead of implying the
timeout covers it.

## Noted, not fixed

`one or more <class>` currently matches the *shortest* run, not the longest:
`find digits in "a1b22c333"` returns `1`, and replacing with `one or more
digit` rewrites each digit separately. That is a pattern-VM greediness
question, independent of replacement — `replace` faithfully replaces whatever
`find_all` reports — so it is left for its own issue rather than folded into
this batch. The docs avoid examples that would depend on it, and
`greedy`/`lazy` are lexed but not accepted in that position today.
