# 2026-10-31 — Remove the `downward` keyword that never existed

Both keyword reference pages listed `downward` as a reserved count-loop
direction keyword. The complete page also listed `upward`, and both had
`count downward` / `count upward` examples. Neither word was ever a keyword:
`src/lexer/token.rs` defines no such token. A count loop counts down with
`count from 5 down to 1:`, where `down` is an ordinary identifier that
`parse_count_loop` recognizes only between the start value and `to`. A reader
who trusted the table would write a parse error, and would also wrongly avoid
`downward` and `upward` as variable names.

## What changed

- `Docs/reference/keyword-reference.md`: removed the `downward` row (Control
  Flow 24 → 23) and added a "Counting down" note under the table.
- `Docs/reference/reserved-keywords.md`: removed `downward` and `upward` from
  the Miscellaneous list (6 → 4), the Control Flow feature list (23 → 22), and
  the alphabetical table. Added `down` to "Marker Words That Are Not Keywords
  at All", with a new "`down` and Counting Down" subsection and a validated
  example.
- Totals: both words were counted inside "Other Reserved Keywords", so
  Other went from 96 to 94 and the total from 181 to 179. The same numbers
  changed in `CLAUDE.md`, `Docs/reference/language-specification.md`,
  `Docs/03-language-basics/variables-and-types.md`, and
  `Docs/06-best-practices/naming-conventions.md`.
- The `reversed` rows showed `count reversed`, which does not parse. They now
  show the real form, `for each item in reversed items`.
- `Docs/03-language-basics/loops-and-iteration.md` gained a **Counting Down**
  section. Before, `down to` was only mentioned inside the step paragraph.
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
  prints 10, 5, 0.
- `count from top down to 1:` with a variable `top` fails with
  `Variable 'top down' is not defined` (exit 3): the lexer joins neighboring
  plain words into one name. `count from (top) down to 1:` works. The docs
  describe this trap and the parentheses fix.

`tests/suite/count_down_keyword_docs_test.rs` pins the parser behavior and
guards the docs: neither keyword page may list `downward` or `upward` as a
keyword, both must show `count from 5 down to 1` and link to the loops guide,
and both new code blocks must stay byte-identical to their validated examples
(`TestPrograms/docs_examples/basic_syntax/count_down_01.wfl` and
`TestPrograms/docs_examples/keyword_reference/down_is_not_reserved.wfl`). This
is an R0 documentation change. The docs tests were committed first and failed
on the old pages (the `downward` row at `keyword-reference.md:50`, and no
`down to` text). Changing one line of the loops-guide block made the drift
test fail; the edit was reverted.

## Found along the way (not fixed here)

- The new totals only remove two words that were never keywords. They are
  still not a true count of the lexer. `src/lexer/token.rs` defines 194 word
  tokens plus four boolean words. Nineteen of them (`test`, `describe`,
  `expect`, `include`, `export`, `starts`, `ends`, `empty`, and others) are not
  in the alphabetical table. The table also lists `pop`, which is not a lexer
  keyword, so it has 180 rows against the stated 179. The "Other" sub-counts
  do not match their own lists (Pattern Matching says 26 and lists 24; Web &
  Network says 17 and lists 19; Data & Types says 6 and lists 5), and `comes`
  appears in two of those lists. A full recount is still a separate audit.
- The `top down` trap breaks the No-Unlearning Invariant: the natural way to
  count down from a variable does not work. Making `parse_count_loop` (or the
  lexer) split a trailing `down` off a multi-word name before `to` would fix
  it. That is a behavior change and needs its own Red→Green work.
