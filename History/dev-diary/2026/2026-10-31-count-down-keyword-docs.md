# 2026-10-31 — Document `down to`, the count-down form the keyword pages missed

Both keyword reference pages used to list `downward` (and the complete page
also `upward`) as reserved count-loop direction keywords, with
`count downward` / `count upward` examples. Neither word was ever a keyword:
`src/lexer/token.rs` defines no such token. The keyword-reference drift fix
(`2026-10-31-keyword-reference-drift.md`) removed both words and corrected
every count while this change was in progress. It did not say how WFL does
count down, so a reader who looked up `downward` still learned nothing about
the real form.

A count loop counts down with `count from 5 down to 1:`. `down` is an ordinary
identifier that `parse_count_loop` (`src/parser/stmt/control_flow.rs`)
recognizes only between the start value and `to`.

## What changed

- `Docs/reference/reserved-keywords.md`: `down` joins "Marker Words That Are
  Not Keywords at All", with a new **`down` and Counting Down** subsection and
  a validated example. The `pop` / `upward` / `downward` note now points to it.
- `Docs/reference/keyword-reference.md`: a **Counting down** note under the
  Control Flow table, and a pointer from the `pop` / `upward` / `downward`
  scenario.
- `Docs/03-language-basics/loops-and-iteration.md`: the count-loop step fix
  (`2026-10-31-count-loop-step-always-checked.md`) added a short **Counting
  Down** section while this change was in review. This change extends that
  same section instead of adding a second one: `down` is not a keyword, there
  is no `downward` or `upward`, and a variable start value needs parentheses.
- The `reversed` rows showed `count reversed`, which does not parse. They now
  show the real form, `for each item in reversed items`.
- `CLAUDE.md` gained a Critical Syntax bullet: `count from 5 down to 1:`, not
  `count downward`.

## What was verified

All claims were checked against the release binary:

- `count downward from 5 to 1:` fails with
  `Expected 'from' after 'count' ... found Identifier("downward")`, and
  `count from 5 downward to 1:` fails with
  `Expected 'to' or 'down to', found Identifier("downward")` (exit 2).
  `upward` behaves the same. The lexer reports both words as `Identifier`.
- `store downward as ...`, `store upward as ...`, and `store down as ...` all
  work.
- `count from 5 down to 1:` prints 5 to 1, and `count from 10 down to 0 by 5:`
  prints 10, 5, 0. The validated example
  `TestPrograms/docs_examples/basic_syntax/count_down_01.wfl` shows only the
  variable-start case, because the merged section already shows the rest.
- `count from top down to 1:` with a variable `top` fails with
  `Variable 'top down' is not defined` (exit 3): the lexer joins neighboring
  plain words into one name. `count from (top) down to 1:` works. The docs
  describe this trap and the parentheses fix.

`tests/suite/count_down_keyword_docs_test.rs` pins the parser behavior, checks
that both keyword pages show `count from 5 down to 1` and link to the loops
guide, and keeps both new code blocks byte-identical to their validated
examples (`TestPrograms/docs_examples/basic_syntax/count_down_01.wfl` and
`TestPrograms/docs_examples/keyword_reference/down_is_not_reserved.wfl`). This
is an R0 documentation change. The docs tests were committed first and failed
on the old pages. Changing one line of the loops-guide block made the drift
test fail; the edit was reverted.

The first version of this change also removed `downward` / `upward` and
recounted the keywords by subtracting them (181 → 179). The drift fix landed
first with a full recount against the lexer (198), so the merge kept its
counts and tables. A test that rejected `downward` / `upward` rows was dropped
in the merge, because `keyword_reference_drift_test.rs` already rejects any
documented word the lexer does not define.

## Found along the way (not fixed here)

- The `top down` trap breaks the No-Unlearning Invariant: the natural way to
  count down from a variable does not work, and the message does not say why.
  Making `parse_count_loop` (or the lexer) split a trailing `down` off a
  multi-word name before `to` would fix it. That is a behavior change and needs
  its own Red→Green work.
