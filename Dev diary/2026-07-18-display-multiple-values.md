# Dev Diary — Multi-value `display`

**Date:** 2026-07-18
**Area:** Parser (`display` statement)
**Type:** Bug fix / small language ergonomics improvement

## The report

A user wrote what reads as perfectly natural WFL:

```wfl
store user age as 28
display user age "user age is" user age
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
left-associative `Concatenation`:

```
display a b c   ≡   display a with b with c
```

Implementation:

- Added `Parser::is_value_start` (`src/parser/helpers.rs`) — true for the tokens
  that begin a fresh value (string/int/float/bool/nothing literals, identifiers,
  `(`).
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

### Scope left alone (deliberately)

A leading unary minus (`display "x" -5`) and bracket list literals still aren't
value-start triggers, so those rare shapes are unchanged. They were never part
of the report, and treating operator/bracket tokens as new values risks
colliding with arithmetic and index syntax. `with` still covers them.

## Tests

- Parser unit tests (`src/parser/tests.rs`): string→var, var→string, three-value
  left-associative fold, index-access regression, single-value regression, and a
  "doesn't leak into the next statement" test (the following `change` still
  parses).
- E2E: `TestPrograms/display_multiple_values.wfl` (the original report, the
  `with` equivalent, mixed values, an expression, index access, and the
  `check`/`otherwise` block).
- Docs: `TestPrograms/docs_examples/basic_syntax/display_multiple_01.wfl`
  (manifest-tracked, 5-layer validated) backing the new
  *Display Several Values at Once* section in
  `Docs/02-getting-started/hello-world.md`.
