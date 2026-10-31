# 2026-10-31 — `find all <pattern> in <text>` works as documented

## What

`Docs/05-standard-library/pattern-module.md`, the builtin reference, and the
"Replacing Matches" guide all promised `find all <pattern> in <text>`. It did
not exist. The lexer merges adjacent plain words, so `find all word in t`
reached the parser as `find`, `Identifier("all word")`, `in`, `t`; the `find`
arm had no `all` handling and built a single-match `find` over a variable
literally named `all word`. The documented program stopped with
`error[ANALYZE-SEMANTIC]: Variable 'all word' is not defined` (exit 3). The
doc's own example quietly used `pattern_find_all of text and word` instead.

## Implement or correct the docs?

The choice was between implementing the spelling and rewriting the docs to the
function form. The No-Unlearning Invariant decided it: a beginner who learns
`find word in text` reaches for `find all word in text` next, and the only
working alternative, `pattern_find_all of text and word`, swaps the argument
order and drops to a snake_case function. That is the "work around the
language to do the most natural thing" crack the invariant says to fix rather
than document. #700 (`exit program`) was the same kind of defect and was fixed
the same way.

## How

**New node.** `Expression::PatternFindAll { text, pattern, legacy_binding,
legacy_fallback, .. }`. The interpreter runs it with
`find_all_with_budget` under the run's shared budget, so a step/state breach is
a catchable resource error rather than a short list, and returns the same list
of match objects as `pattern_find_all` (the object builder now lives in one
helper, `stdlib::pattern::match_list_value`). The type checker gives it
`List of Map from Text to Any`; elements of that list need no `nothing` check.

**Parsing the merged token.** After `find`, an `Identifier("all <name>")` is
split: `<name>` seeds the pattern operand through
`parse_seeded_expression_continuation`, the helper the merged `flush` and
`write line` operands already use, so postfix and `of` calls behave as for any
operand. A bare `Identifier("all")` (the lexer does not merge `all` with `(` or
a literal) starts an ordinary operand. A bare `all` directly before `in` is
left alone: `find all in t` searches with a pattern named `all`, as before.

**Not stealing old programs (testing.md §11.6).** `all` is not reserved, and
WFL names may contain spaces, so `find all digits in t` was already valid when
the program defined `all digits`. The node therefore carries the pre-`find all`
parse of the same tokens as `legacy_fallback`, plus the full name
(`legacy_binding`). The analyzer, type checker, and interpreter each pick the
fallback while that name is defined — the same mechanism as the
`FlushStreamStatement` back-compat fields. If only the old parse succeeds, or
the two readings end at different tokens, the parser returns the old parse
unchanged.

**Keeping the double parse linear.** `flush` is a statement, but `find all`
nests. If the fallback parse could itself recurse into `find all` handling,
depth *d* would cost about 2^d primary-expression parses on untrusted input.
The fallback is, by definition, what the old parser produced, so it is parsed
with a `legacy_find_only` flag that turns `find all` recognition off. Each level
then costs one full parse plus one linear one.

**Fixer.** The source-preserving `--fix` renames variables token by token. In
`find all Word Finder in t` the use is inside the merged token
`all Word Finder`, so renaming the declaration to `word_finder` would have
broken the program. The operand is now protected, like the merged
`exit ... with code <name>` operand.

## Testing

Red first, in three test-only commits that precede the implementation:

- `tests/suite/find_all_syntax_test.rs` — 16 binary-driven and parser tests.
  Against the dev base, 11 failed for the intended reason (the documented
  spelling was read as an undefined `all word`, or parsed to `PatternFind`
  over `"all word"`; the fixer test showed the split rename). Five passed and
  pin behaviour that must survive: four programs with existing `all digits`,
  `all`, and `all patterns` bindings, and the missing-`in` parse error.
- `TestPrograms/patterns/find_all_syntax.test.wfl` — exit 3 on the base binary.
  A control copy with each `find all X in Y` rewritten to
  `pattern_find_all of Y and X` passed 5/5 on the same binary, so the Red was
  the missing form and nothing else.
- `parser::tests::nested_find_all_parses_in_polynomial_steps` — 30-deep
  nestings must each parse in under 50 000 primary-expression parses. Red on
  base: the form was not recognised. The first version nested only through the
  pattern operand, `find all (find all (...) in t) in t`, and its first Green
  run taught two things. The old reading fails at the first `(` there, so that
  shape could never catch an exponential fallback; nesting through the text
  operand, `find all word in (find all word in (...))`, does, and was added in
  a third Red commit. And the unoptimized parser overflowed the default 2 MiB
  test thread at that depth. The existing single-match `find (...)` nesting
  overflows there too (from depth 27; `find all` from 26), so it is a property
  of debug frames, not of this change; the binary parses on its 1 GiB
  interpreter stack, and the test now does the same on a 64 MiB thread.

  Green: 1 456 parses through the text operand and 121 through the pattern
  operand at depth 30. Mutation check: with `legacy_find_only` disabled, depth
  20 alone took 5 242 876 parses (about 5·2^20) and the test failed.

## Not changed here

Reading a field straight off a `find` result (`hit.matched_text` or
`hit["matched_text"]` with no `isnothing of hit` check first) prints a
type-checking warning, "Cannot access property 'matched_text' on non-container
type Map from Text to Any or Nothing". The program still runs. The warning is
arguably right — `find` can return `nothing` — but its wording points at the
wrong problem. Guarded access, as in the docs' `find` example, is already
clean. Changing the diagnostic is a separate decision and is tracked
separately.
