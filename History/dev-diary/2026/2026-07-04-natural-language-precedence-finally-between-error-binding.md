# Natural-Language Rough Edges: Precedence, `/`, `finally`, `between`, Error Binding (Issue #571)

**Date:** 2026-07-04

## What Changed

Issue #571 (a documentation code-examples audit) surfaced a set of constructs
that *read like perfectly natural WFL and appear throughout the docs* but did
not work on the release interpreter. Each is the form a beginner reaches for
first, so leaving them broken violated WFL's "no-unlearning" natural-language
promise. This change fixes the language side of the highest-value items rather
than teaching workarounds in the docs.

### Parser precedence (`src/parser/expr/binary.rs`, `src/parser/expr/primary.rs`)

- **Arithmetic now binds tighter than comparison (#1).** `check if y plus 1 is
  equal to 4:` previously failed to type-check because the multi-token
  comparison operators (`is …`, `contains`, `or`) consumed their tokens
  *before* the precedence check ran. While parsing the right-hand side of an
  arithmetic operator (precedence 2/3) the parser would eagerly eat `is equal
  to`, corrupting the parse. Added precedence guards so a comparison
  (precedence 1) and `or` (precedence 0) stop *without consuming* when
  encountered inside a tighter sub-expression. `(y plus 1) is equal to 4` now
  parses as intended.
- **`of`-call arguments absorb arithmetic (#2).** `fibonacci of n minus 1`
  parsed as `(fibonacci of n) minus 1`, so a recursive argument never
  decreased. Call arguments are now parsed with a dedicated
  `parse_of_call_argument` helper that absorbs only arithmetic
  (`plus`/`minus`/`times`/`divided by`/`/`/`%`/`modulo`) and stops at `and`,
  `with`, `from`/`by`/`length`, comparisons, and pattern keywords — so
  `fibonacci of n minus 1` means `fibonacci of (n minus 1)` while multi-argument
  and `with`-concatenation forms keep working.

### Lexer (`src/lexer/token.rs`)

- **`/` division operator (#3).** A lone `/` now lexes as `Token::Slash`
  (division). `//` still starts a comment — logos' longest-match rule keeps the
  comment skip pattern winning over the single-character token.
- **`modulo` word form (#8a).** Added `Token::KeywordModulo` as the word form of
  `%`, matching `plus`/`minus`/`times`.
- **`finally` keyword (#4).** Added `Token::KeywordFinally` (a structural
  keyword).

### `is between` / `is above` / `is below` (#5, #7)

- `X is between A and B` is now an inclusive range check that desugars to
  `X >= A and X <= B`.
- `X is above N` / `X is below N` map to greater-than / less-than. The `above`,
  `below`, and `between` keyword tokens already existed but were only wired into
  pattern quantifiers; they now work in ordinary conditions too.

### `finally` clause (#4)

- Added `finally_block` to `Statement::TryStatement` and threaded it through the
  parser, interpreter, type checker, analyzer, static analyzer, and JS
  transpiler. A `finally:` block runs on both the success and error paths, after
  any matching `when`/`otherwise` clause. If the `finally` block itself raises an
  error, that error wins; otherwise the primary result (success value or the
  still-unhandled error) propagates. A `try` may now consist of a body plus only
  a `finally` (no `when`/`catch`).

### Caught-error binding `when error as <name>` (#9)

- `when error as e:` now binds the caught error under the given name.
  Previously only the implicit `error_message` alias worked and `as` was a parse
  error. The implicit `error_message` alias is still always available.

## No-Unlearning Impact

Every item above is the *first* form a beginner writes. Fixing them at the
language level (instead of documenting a workaround) keeps the beginner path a
subset of the expert path — `check if x is between 1 and 10`, `10 / 2`,
`fibonacci of n minus 1`, and try/when/**finally** now do what they read like.

## Tests

- Lexer: `/` vs `//`, `modulo`, `finally` tokens (`src/lexer/tests.rs`).
- Parser: arithmetic-vs-comparison precedence, `of`-call arithmetic absorption
  (and that it still stops at `with`), `is between` desugaring, `is
  above`/`is below`, and `try … when error as e … finally …`
  (`src/parser/tests.rs`).
- Interpreter: runtime results for `/`, `modulo`, `3 plus 1 is equal to 4`,
  `is between`, and `finally` on both success and caught-error paths
  (`src/interpreter/tests.rs`).
- End-to-end: `TestPrograms/natural_language_constructs.test.wfl` (12 tests via
  the built-in test framework).

All 461 library unit tests, the full `cargo test` suite, and the 101 runnable
integration TestPrograms pass. `cargo fmt` and `cargo clippy -D warnings` are
clean.

## Keyword count

Adding `finally` (structural) and `modulo` (arithmetic/other) takes the reserved
keyword total from **178 → 180** (53 structural, 29 contextual, 96 other, 7
literals). The two-tier keyword reference, the language specification, and
`CLAUDE.md` were updated to match.

## Not addressed here

Issue #571 lists many more items (weak return-type inference, `repeat N times`,
text→number conversion, pattern-VM and filesystem-glob bugs, etc.). Those are
tracked separately and are not part of this precedence/`finally`/`between`/
error-binding change.
