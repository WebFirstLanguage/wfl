# Dev Diary — Multi-value `display`

**Date:** 2026-07-18
**Area:** Parser (`display` statement)
**Type:** Bug fix / small language ergonomics improvement

## The report

A user wrote what reads as perfectly natural WFL:

```wfl
store user age as 28
display "user age is " user age
change user age to 9

check if user age is greater than 18:
    display "Access granted"
otherwise:
    display "Must be 18 or older"
end check
```

…and found that `display` only ever printed the **first** value:

- `display user age` → prints the age ✅
- `display "user age is" user age` → prints only `user age is` ❌
- `display user age "user age is"` → prints only the age ❌

## Root cause

`parse_display_statement` (in `src/parser/stmt/io.rs`) consumed exactly **one**
expression after `display` and returned. Because WFL statements are terminated
by `Eol`, any additional value tokens on the same line were left on the cursor
and re-parsed by the top-level loop as their own `ExpressionStatement` — which
evaluates the value and throws it away. The result was silent, partial output
with no error: the worst kind of surprise for a beginner-first language.

Note that two *adjacent bare variables* can't hit this (the lexer merges
consecutive identifier words into a single multi-word identifier, so
`first name` is one variable). The bug shows up whenever values straddle a type
boundary — string↔variable↔number — which is exactly the "label plus value"
shape people reach for first.

## The fix

Per the reporter's own rule — *"anything in quotes is text, anything else is a
variable or an action, and I should be able to have more than one"* — `display`
now accepts multiple space-separated values and folds them into a single
`Concatenation`, right-associative exactly like `with` (see "Deep-review fix
pass" below for why the association direction matters, not just the shape):

```
display a b c   ≡   display a with b with c
```

Implementation:

- Added `Parser::is_value_start` (`src/parser/helpers.rs`) — true for the tokens
  that begin a fresh value: string/int/float/bool/nothing literals, identifiers,
  `(`, and the keyword-led value expressions `call`, `count`, and `current`
  (see the review follow-up below).
- After parsing the first value, `parse_display_statement` loops while the next
  token is a value-start, parsing each additional value with the full
  expression parser and folding it into a `Concatenation`.

### Why this is backward-compatible

- **Direct index access is untouched.** `display numbers 0` is absorbed by the
  first `parse_expression` call (the `0` is the index), so the trailing token
  never reaches the fold. Verified: `display "first score: " scores 0` →
  `first score: 100`.
- **Line breaks still terminate.** `Eol` is not a value-start, so
  `display numbers` followed by `0` on the next line keeps the `0` as its own
  statement.
- **Single values are unchanged** — no `Concatenation` wrapper is created.
- The only programs whose behavior changes are ones that previously
  *silently dropped* trailing values — i.e. ones that were already broken.

### Spacing

Values are joined directly, exactly like `with` — no separator is inserted.
Spaces come from the quotes: `display "I am " age " years old"` →
`I am 25 years old`. This keeps a single concatenation semantics in the
language (No-Unlearning: the space-separated form and the `with` form are the
same form), rather than inventing a second, space-adding rule that only
`display` would follow.

### Review follow-up: keyword-led values

Automated review (Codex/Copilot) flagged that the first cut only folded
literals, identifiers and `(`, so a value that *starts* with a keyword was
still dropped — or worse. Two concrete cases:

- `display "number: " call get_number` printed only `number: ` and evaluated
  the call separately.
- `display "count is " count` inside a count loop **failed to parse**: `count`
  reached `parse_statement` as leftover and was treated as the start of a new
  count loop (`Expected 'from' after 'count'`). That's a hard error on natural,
  documented code (the count-loop variable), strictly worse than the original
  silent drop.

Fix: `is_value_start` now also accepts `call`, `count`, and `current` — the
keyword-led primary expressions that are *not* also binary operators. Keywords
the binary parser already consumes after a value (`with`, `find`, `replace`,
`split`, `matches`) never reach the fold, and a leading `-` is parsed as
subtraction against the previous value (so `display "x" -5` was never a fold
case). `current` requires its full form (`current time in milliseconds` /
`formatted as ...`) exactly as it does as a first value — an incomplete
`current time` errors identically whether folded or not. The docs were also
tightened to describe values honestly (a variable, number, action call, or
arithmetic expression) rather than promising arbitrary expression forms.

Also addressed in the same pass: `parse_display_statement` now `expect`s the
`display` token (it is only dispatched on `display`) instead of defaulting the
statement position to a misleading `(0, 0)`.

## Tests

- Parser unit tests (`src/parser/tests.rs`): string→var, var→string, three-value
  right-associative fold, index-access regression, single-value regression, a
  "doesn't leak into the next statement" test (the following `change` still
  parses), and the review-driven keyword-value folds (`count` variable and a
  `call` action).
- E2E: `TestPrograms/display_multiple_values.wfl` (the original report, the
  `with` equivalent, mixed values, an expression, index access, a folded `call`,
  the `count` loop variable, and the `check`/`otherwise` block).
- Docs: `TestPrograms/docs_examples/basic_syntax/display_multiple_01.wfl`
  (manifest-tracked, 5-layer validated) backing the new
  *Display Several Values at Once* section in
  `Docs/02-getting-started/hello-world.md`.

## Deep-review fix pass

A maintainer deep-review (and an independent `@claude` validation pass) of the
first cut found five issues, all fixed in this pass:

**1. Association direction was observably wrong.** The first cut folded
left-associatively (`(a with b) with c`), while explicit `with` parses
right-associatively (`a with (b with c)`, see the `KeywordWith` continuation
in `expr/binary.rs`). `Expression::Concatenation` evaluates left, then right,
then stringifies both — so association direction controls *when* a value gets
stringified relative to a later value's side effects. With two identical
lists each holding `[before, after]`:

```wfl
display left_items "" pop of left_items          // [before, after]after  (left-fold, WRONG)
display right_items with "" with pop of right_items  // [before]after     (with, right-fold)
```

The left fold stringified `left_items` (as `"" `'s left operand) *before* the
`pop` on the right ran, showing the pre-mutation list; `with` stringifies it
*after*, per its evaluation order. `parse_display_statement` now parses every
value into a `Vec<Expression>` first, then folds from the right, producing the
identical tree — not just a similar one — to the equivalent `with` chain. See
`test_display_four_values_right_associative_nesting` (parser) and the
mutation-order case in `tests/display_multiple_values_stdout_test.rs`
(interpreter, exact stdout).

**2. `is_value_start` was still missing safe keyword starters.** Added `not`,
`pattern`, `output`, `file`, `directory`, `process`, `header`, `list`, and
`read` — each is a keyword-led `parse_primary_expression` arm that produces a
standalone value and is never consumed as a continuation elsewhere, so it's
safe to fold. Deliberately *not* added: `loop`, `exit`, `repeat`, `try`,
`when` (these double as statement/block openers, so silently folding them
into a `display` would swallow what's far more likely to be a missing line
break than an intended value) and `Minus`/`LeftBracket` (both are always
already consumed earlier in the parse — see the `is_value_start` doc comment
in `helpers.rs` for the full reasoning). `back` and `error` are unambiguous by
the same test as `not`, but weren't flagged by review, so they were left as a
possible follow-up rather than added speculatively.

**3. `find`/`replace`/`split` remain excluded — this is a pre-existing bug,
not a `display` bug, and out of scope here.** When one of these keywords
follows an already-parsed value at the *first* value's `parse_expression`
call (before `display`'s fold loop is ever consulted), the general
binary-expression parser's continuation arm for that keyword
(`expr/binary.rs`, the `Token::KeywordFind`/`KeywordReplace`/`KeywordSplit`
arms under `parse_binary_expression`) discards the preceding value in some
paths — e.g. `display "parts: " split "a,b" by ","` prints only `[a, b]`, not
`parts: [a, b]`, because `split`'s continuation arm never incorporates `left`.
Adding these tokens to `is_value_start` would not fix this, since the bug
happens one call frame earlier. Fixing the general grammar is a larger,
independently-scoped change (it affects every expression, not just
`display`) that deserves its own TDD pass and regression suite rather than
being folded into this PR under time pressure — flagged in the PR thread as a
follow-up question for the maintainer rather than silently left alone.

**4. Ambiguity claims corrected.** The docs and this diary previously implied
space-separated `display` values are free-form. In fact several forms that
look like "two values" are actually one, by the same grammar rules that apply
everywhere else in WFL, not anything `display`-specific: a run of bare words
(`display a b c`) is one multi-word identifier (the lexer merges consecutive
identifier tokens before the parser ever sees them); `display numbers 0` is a
direct index, not two values; `display total -5` is subtraction, not two
values (unary `-` is only reachable when nothing precedes it, and here the
first value's `parse_expression` call already consumes `-5` as `total minus
5` before the fold loop is ever reached). `Docs/02-getting-started/hello-world.md`
now says this plainly instead of only demonstrating the happy path.

**5. Added exact-stdout regression coverage.** Prior tests only checked AST
shape (parser unit tests) or exit code (the `TestPrograms/*.wfl` CI runner
redirects stdout to `/dev/null`). `tests/display_multiple_values_stdout_test.rs`
now asserts exact stdout for the documented happy paths, a user-defined action
return value, an action with arguments and one with a side effect, a
container instance/property/method, and — the one that matters most — the
mutating-list case proving space-separated `display` and `with` now produce
byte-identical output.

## New-head follow-up: centralization and more same-line boundaries

A second maintainer review of `fef9fb7` asked for `is_value_start` to stop
being an independently-maintained token list and instead be defined in terms
of the *same* classification `parse_primary_expression` uses, plus asked for
the `count`/`read` same-line fix from the deep-review pass to be regression
tested as whole programs rather than trusted by inspection.

- **Centralization.** `src/parser/helpers.rs` now has
  `Parser::can_start_primary_expression`, a single predicate mirroring every
  arm of `parse_primary_expression` (explicit keyword arms plus the
  contextual-keyword catch-all via `Token::is_contextual_keyword`).
  `is_value_start` is now `can_start_primary_expression` minus an explicit,
  documented exclusion list, instead of its own hand-maintained list — so a
  new `parse_primary_expression` arm is included by default rather than
  requiring a second edit to opt in. A coupling test in `parser/tests.rs`
  (`can_start_primary_expression_matches_parse_primary_expression`) feeds a
  representative token through both functions and asserts they agree, so the
  two can't silently drift apart again. This also added `back` and `error` to
  `is_value_start` (previously left out only because they weren't flagged by
  review, not because they were unsafe — see finding 2 above).
- **New same-line statement boundaries.** Actually centralizing the
  classification (rather than hand-curating a short list) surfaced six more
  tokens with the exact same shape as the `count`/`read` finding: `create`,
  `change`, `push`, `parent`, `skip`, and `give` are all contextual keywords
  (bare variables in expression position) that are *also* dedicated arms of
  `parse_statement`'s top-level dispatch, with no expression-position
  equivalent for their statement form. Unlike `count`/`read`, none of the six
  has one unambiguous continuation token to guard on (`create` alone forks
  into containers, lists, patterns, directories, files, maps, dates, times,
  and plain variable declarations), so — same reasoning as the pre-existing
  `loop`/`exit`/`repeat`/`try`/`when` exclusions — they're excluded from
  `is_value_start` outright rather than guarded. `skip` was the sharpest case
  to catch: as a bare statement it's `continue` (a control-flow effect, one
  token), with *no syntactic difference at all* from folding it as a value
  (also one token) — an unguarded inclusion would have silently turned a
  loop's `continue` into inert display output instead of a parse error.
  `parse_display_statement`'s fold loop also gained a real lookahead guard,
  `Parser::is_display_fold_statement_boundary`, so `display "x" count from 1
  to 3:` and `display "x" read output from process p` keep parsing as two
  statements (the count-loop/read-output form) instead of the display
  swallowing just the keyword and stranding the rest. Each of the eight
  excluded tokens (the original five plus these six) now has an explicit
  `*_after_display_stays_a_separate_statement` whole-program regression test
  in `parser/tests.rs`.
- **Test determinism.** The `file exists at "does-not-exist-for-sure.txt"`
  stdout case depended on the test runner's working directory not happening
  to contain a file by that name. It now builds a unique, absolute path under
  the OS temp directory via `test_helpers::get_unique_test_file_path`, the
  same helper the rest of the integration suite already uses to avoid
  cross-test and cross-machine collisions.
- **Tooling note.** `cargo build`/`test`/`clippy`/`fmt` were blocked by this
  session's tool-approval policy with no human available to grant it, so this
  pass was verified by careful manual trace against the current source rather
  than a local build — flagged as a blocker in the PR thread, consistent with
  the deep-review pass's first attempt.

## Third-head follow-up: fixing CI formatting, a Windows path bug, and real centralization

A third maintainer review of `e95b0a2` found CI red on `cargo fmt --all --
--check` (two call sites over rustfmt's 100-column width), a Windows-specific
lexer bug in the new stdout test, and — the substantive finding — that
`can_start_primary_expression` was *still* an independently hand-maintained
`matches!` list, no different in kind from the token list it replaced; only
the coupling *test* had changed, not the coupling itself.

- **Formatting.** Reformatted the `PushStatement` `matches!` assertion in
  `src/parser/tests.rs` and the `missing_path` builder in
  `tests/display_multiple_values_stdout_test.rs` to match rustfmt's actual
  output (confirmed against the CI diff for run `29660025888`, since `cargo
  fmt` itself was not runnable locally — see the tooling note above, still
  unresolved this pass).
- **Windows-unsafe path interpolation.** `env::temp_dir()` on Windows returns
  a `\`-separated path (e.g. `C:\Users\...\Temp\...`), and WFL string literals
  only recognize `\n`, `\t`, `\r`, `\\`, `\0`, and `\"` as escapes (see
  `parse_string` in `src/lexer/token.rs`) — anything else after a backslash is
  a lex error. The `keyword_led_values_fold_with_exact_output` stdout test
  embedded `missing_path` directly into a WFL string literal, so on Windows a
  component like `\Local` or `\Temp` would lex as an invalid escape and fail
  the whole test. Fixed by escaping each `\` as `\\` before interpolating, so
  the embedded literal round-trips to the exact same path on every OS.
- **Actual centralization.** The previous pass's `can_start_primary_expression`
  was compared against `parse_primary_expression` only by a *sample-based*
  unit test — a real improvement in coverage, but structurally the same kind
  of list `is_value_start` always was: a new `parse_primary_expression` arm
  for a token outside the sample would still silently pass every test. Fixing
  this without hand-enumerating the ~200-variant `Token` enum meant moving the
  enforcement out of a test and into the parser itself:
  `parse_primary_expression` (`src/parser/expr/primary.rs`) is now a thin
  wrapper around its real dispatch, renamed
  `parse_primary_expression_dispatch`. The wrapper captures the leading token
  before dispatching, then compares `can_start_primary_expression`'s
  prediction against what the dispatch actually did — via a pair of
  `debug_assert!`s, compiled out in release builds like the rest of this
  crate's runtime invariants. This runs on *every* primary-expression parse,
  not a curated sample: every parser test, every `TestPrograms/*.wfl` run,
  every program compiled in a debug build. A new dispatch arm added without
  updating the predicate (or the reverse) now panics the first time anything
  exercises that token, rather than only when someone remembers to extend a
  hand-picked list.

  The one subtlety worth recording: naively comparing the *error message text*
  between the two functions is unsound, because an arm that recurses (e.g.
  `file size of <expr>`) can propagate a *nested* failure's generic
  "Unexpected token in expression" text up through `?` even though the
  *leading* token (`file`) genuinely has a dedicated arm. The fix compares
  message text *and* source position: the generic fallback error is always
  raised at the exact position of whichever token triggered it, and every arm
  consumes its own leading token via `bump_sync()` before recursing into
  anything else — so a nested failure's position is always strictly later
  than the leading token's, never coincidentally equal. Requiring both to
  match distinguishes "this token itself has no arm" from "this token's arm
  recursed into something else that failed." The existing sample-based
  coupling test in `parser/tests.rs` is kept as explicit, documented coverage
  of each keyword-led arm, but it is no longer the only thing standing between
  the two staying in sync.
- **Tooling note (unresolved).** `cargo`, `rustc`, and `git` mutating commands
  still require approval with no human available to grant it in this session.
  This pass, like the two before it, was verified by careful manual trace
  against the current source — including hand-tracing every
  `parse_primary_expression` arm's error paths against the new `debug_assert!`
  logic above to rule out false positives — rather than a local build. CI is
  the first real compiler/test run these changes see.
