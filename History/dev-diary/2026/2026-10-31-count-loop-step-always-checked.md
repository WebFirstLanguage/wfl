# 2026-10-31 — Count-loop steps are always checked (error now, or warn until 2027-12-01)

## What

`count from 5 to 1 by -1` ran zero times, printed nothing and exited 0. It
reads like a countdown, so the program looked right and silently did nothing.

A matrix run of every combination of direction (`to` / `down to`), range order
and step sign against the release binary showed the problem was general, not
specific to 5 and 1, and that it happened in both directions:

| Loop | Before |
|---|---|
| `count from 5 to 1 by -1`, `count from 12 to 2 by -2` | 0 trips, exit 0, no message |
| `count from 1 down to 5 by -1` (mirror) | 0 trips, exit 0, no message |
| `count from 5 to 1 by 0`, `count from 1 down to 5 by 0` | 0 trips, exit 0, no message |
| `count from 1 to 5 by -1`, `count from 5 down to 1 by -1` | error (already loud) |

## Why it happened

The step is a distance: `to` counts up, `down to` counts down, and a downward
loop subtracts the step. The interpreter validated the step only when the
loop would be entered (`should_continue(start, end) && step <= 0`). A step
pointing the "wrong" way on a range that is empty in the written direction
was therefore never looked at.

That was a deliberate decision. The #699 fix first validated the step
unconditionally, and review of that fix called it a compatibility break:
`count from 5 to 1 by 0` had always completed. The check was narrowed to
loops that run, and the loops guide documented the result: "`count from 5 to
1` does nothing, whatever step you give it."

## The decision

This change reverses that decision; the requester chose the direction after
seeing the matrix. Two fixes were weighed:

- **Negative step counts down** (BASIC `STEP -1`, Python `range(5, 0, -1)`).
  This is additive, but it gives WFL two ways to count down. It would also
  change the documented rule that the step is always positive, and leave a
  sharp edge: `count from n to 1 by -1` with `n = 0` would have to be either
  an error or silent.
- **Always report a bad step** — chosen. It keeps one rule ("`down to` sets
  the direction; the step is a positive distance"), keeps one way to count
  down, and means no step can ever be silently ignored, in either direction.

A zero step on an empty range is included: `by 0` is never meaningful, so it
is never silent either.

### Governance correction: a deprecation window, not an immediate error

The first version of this change made every bad step an immediate error.
Codex review of #800 pointed out that this breaks programs that currently
finish. GOVERNANCE.md §3.1 requires a break to be announced a year ahead, with
the old behaviour kept working until the deadline. A Maintainer-approved
exception is the only alternative. The point was correct and should have
been caught before the PR went up; the "it never did what it says" argument
does not waive the rule.

The requester chose the deprecation path:

- **Bad step, loop would run** — a runtime error, as before this change, now
  with a message that names the program's own loop.
- **Bad step, empty range** — the loop still runs zero times and the program
  carries on, but WFL prints a warning on stderr, once per loop. The warning
  has the same text as the error, plus "This loop's range is empty, so it
  runs zero times; from 2027-12-01 this will be an error." The removal is
  tracked in #801.

The deadline is 2027-12-01 rather than exactly a year from today, so the
window stays at least a year even if review and release take some weeks.

An empty range with a *valid* step is unchanged and still runs zero times
quietly — `count from 1 to length of items` over an empty list is an idiom,
not a mistake.

## How

`src/interpreter/mod.rs`: the step is checked straight after it is
evaluated, before any trip.

- If the loop would run, it returns `count_loop_step_error` as a runtime
  error.
- If the range is empty, it prints `count_loop_step_warning` on stderr and
  falls through to the normal loop, which runs zero times.
- A `count_step_warned` set, keyed on source file, line and column, makes a
  loop reached on every trip of an outer loop warn only once.
- The date lives in one constant, `COUNT_LOOP_STEP_ERROR_FROM`.

`count_loop_step_error` builds the message:

- A negative step names the loop the program most likely meant. It walks from
  start to end, in whichever direction that is, using the step's magnitude:
  `count from 5 to 1 by -1` →
  "Write `count from 5 down to 1`"; `count from 12 to 2 by -2` →
  "`count from 12 down to 2 by 2`"; `count from 1 down to 5 by -1` →
  "`count from 1 to 5`". A step of 1 is the default, so it is left out.
- A zero step says so plainly: "A step of 0 never moves the counter".
- A non-finite step only states the rule, since there is no sensible
  suggestion.

The old message suggested an unrelated example
(`count from 10 down to 1 by 2`). The new one uses the program's own numbers,
which are printed the same way `display` prints them.

The warning names the line where the parser recorded the loop, which is the
`end count` line (see "Noted, not fixed"), so it says "count loop ending on
line N" rather than claiming to point at the `count from` line.

## Testing

Red first, as a test-only commit:

- `tests/suite/count_loop_step_test.rs` runs the real binary and asserts exit
  status 1. It checks that the body and the following statement never run,
  and that the diagnostic names the rule and the suggested loop. Cases: the
  reported loop, other numbers, the mirror direction, already-loud loops
  (new message), a computed fractional step, a zero step on both empty
  ranges, and catching the error with `try` / `when error`. Two guards pin
  what must not change: empty ranges with a valid step stay quiet (including
  the empty-list idiom), and positive steps still count both ways.
- `TestPrograms/count_loop_step_test.wfl` covers the same behaviour end to end
  in `describe` / `expect` form, catching each error with `when error`.
- `TestPrograms/docs_examples/basic_syntax/loops_step_warning.wfl` is the
  example behind the warning shown in the loops guide. The docs validator
  checks only its exit status, so a Rust test
  (`loops_guide_quotes_the_real_warning`) runs the guide's own snippet and
  pins the quoted warning to the interpreter's actual stderr.

After the governance correction, a third test-only commit rewrote these tests
for the deprecation path before the interpreter changed. Empty-range cases
assert exit 0, zero trips, the program carrying on, and the warning on
stderr (never stdout) with the deadline. Loops that would run still assert
the error. Against the error-everywhere interpreter, 10 tests failed for
exactly that reason.

Against the unfixed binary, all four new behaviours in the WFL program failed
with `Expected "" to contain …`, meaning no error was raised at all. The docs
example failed with "Exit code 0, expected 1". The guards passed.

## What review caught

- **A test that could not pass.** A runtime diagnostic quotes the source lines
  around the error, so a literal `display "after"` in a test program showed up
  in the error output even though that line never ran. "The program must stop
  at the loop" therefore failed against a correct fix. The probe words are now
  assembled at run time (`"aft" with "er"`). The assertion is unchanged, and
  re-running against the unfixed interpreter showed the same Red failures.
- **Untested branches.** No test reached the non-finite branch. `inf`, `-inf`
  and `NaN` steps are now built at run time (squaring 1e9 eight times
  overflows), along with a `-0` step, all on empty ranges. Those tests went in
  a second test-only commit, ahead of the fix.
- **Two beginner docs taught the bug.** The REPL guide's "Try backwards" showed
  `count from 5 to 1` printing 5 4 3 2 1, and the first-program exercise hinted
  `count from 10 to 1`. Both loops run zero times. They now use or point to
  `down to`.
- **Migration advice.** Rewriting a dead `count from 5 to 1 by -1` runs its
  body for the first time, so the guide and CHANGELOG now say to review the
  body first, or delete the loop if it was never meant to run.

## Docs shipped with the change

- `Docs/03-language-basics/loops-and-iteration.md` — a new "Counting Down"
  section (the guide never actually taught `down to`), and the step rules
  rewritten: always checked, and the error shown with a note for programs
  written for earlier versions.
- `Docs/reference/language-specification.md` — the grammar now includes
  `down` and `as <identifier>`, plus the step rule.
- `Docs/reference/syntax-reference.md` — a countdown example.
- Both keyword reference pages: `by` is a positive step distance.
- `Docs/02-getting-started/repl-guide.md` and `your-first-program.md` — the
  backwards-counting examples use `down to`.

## Noted, not fixed

The runtime error for a count loop points at the `end count` line, not the
`count from` line. The parser records the loop's position *after* consuming
`end count`. That affects every runtime error a count loop itself raises, so
it is left for its own change rather than folded in here.

Both keyword reference pages also list a `downward` keyword with a
`count downward` example. The lexer has no such token: the real form is
`down to`, where `down` is a contextual word. That is a stale-docs problem
unrelated to the step, so it is left for its own change too.
