# 2026-10-31 — Count-loop steps are always checked

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
- **Clear error, every time** — chosen. It keeps one rule ("`down to` sets
  the direction; the step is a positive distance"), keeps one way to count
  down, and means no step can ever be silently ignored, in either direction.

A zero step on an empty range is included: `by 0` is never meaningful, so it
is never silent either. The two #699 tests that pinned "an empty range ignores
its step" were retired, and their reversed assertions moved to
`tests/suite/count_loop_step_test.rs`.

Compatibility cost: a program containing one of these lines used to finish
(having silently skipped the loop) and now stops with an error. Such a line
never did what it says, so the change surfaces an existing bug rather than
breaking working behaviour. It is the same reasoning #698 and #700 used for
silent wrong answers. The CHANGELOG and the loops guide both call it out.

An empty range with a *valid* step is unchanged and still runs zero times
quietly — `count from 1 to length of items` over an empty list is an idiom,
not a mistake.

## How

`src/interpreter/mod.rs`: the step check no longer consults
`should_continue`. It runs straight after the step is evaluated, before any
trip. The message comes from a new helper, `count_loop_step_error`:

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
- `TestPrograms/docs_examples/error_examples/count_loop_negative_step.wfl` is
  the validated example behind the error shown in the loops guide.

Against the unfixed binary, all four new behaviours in the WFL program failed
with `Expected "" to contain …`, meaning no error was raised at all. The docs
example failed with "Exit code 0, expected 1". The guards passed.

## Docs shipped with the change

- `Docs/03-language-basics/loops-and-iteration.md` — a new "Counting Down"
  section (the guide never actually taught `down to`), and the step rules
  rewritten: always checked, and the error shown with a note for programs
  written for earlier versions.
- `Docs/reference/language-specification.md` — the grammar now includes
  `down` and `as <identifier>`, plus the step rule.
- `Docs/reference/syntax-reference.md` — a countdown example.
- Both keyword reference pages: `by` is a positive step distance.

## Noted, not fixed

The runtime error for a count loop points at the `end count` line, not the
`count from` line. The parser records the loop's position *after* consuming
`end count`. That affects every runtime error a count loop itself raises, so
it is left for its own change rather than folded in here.
