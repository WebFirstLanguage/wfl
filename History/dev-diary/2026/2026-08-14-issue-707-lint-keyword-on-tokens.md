# 2026-08-14 — LINT-KEYWORD was a substring search, not a lint (#707)

## Symptom

```wfl
store s as "MNOP"
display s
```

```text
warning[LINT-KEYWORD]: Keyword 'NO' should be lowercase
 = Change to 'no'
```

The control confirms the mechanism: `"MXYP"` reports nothing. The rule matched
the substring `NO` inside `MNOP`, inside a string literal.

Comments went the same way — `// Note:` produced `Keyword 'No'`, `// TODO:`
produced `Keyword 'TO'` — as did ordinary words: `"Ineligible"` produced
`Keyword 'In'`.

Found while porting a PHP minifier, where the program needs the character-class
constant
`"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_"`. That string
contains `MNOP` and cannot be changed — it *is* the definition of `\w`. Combined
with the companion `LINT-INDENT` defect (#706), it meant `--lint` could not be
used as a gate at all.

## Root cause

`KeywordCasingRule::apply` took the parsed `Program` and ignored it
(`_program`), then ran `source.find()` over the raw file for each of 26
keywords, in `UPPERCASE` and `Mixedcase` forms. No tokenization, no word
boundaries, no exclusion of strings or comments.

The `Mixedcase` half is what made it collide with English: `No`, `To`, `In`,
`For`, `Each`, `End`, `Check`, `If`, `Count`, `From` are ordinary words and
common prefixes, so any file with prose comments was likely to trip several.

It was also a **false negative**. `source.find` returns only the first hit, so
the rule emitted at most one diagnostic per keyword per casing — 52 for a file
of any size. A file with fifty genuine `STORE` keywords reported one. The rule
could neither avoid reporting non-keywords nor finish reporting real ones.

## The fact that shaped the fix

WFL keywords are **case-sensitive**: `src/lexer/token.rs:17` is
`#[token("store")]` with no `ignore(case)`. So `STORE` and `Store` never lex as
keywords — they lex as *identifiers*. `STORE s AS "x"` fails with
`Variable 'STORE s AS' is not defined`.

That rules out the obvious repair. "Only report where the lexer produced a
keyword token" would have reported **nothing**, silently deleting the lint while
looking like a fix.

The rule now flags **identifier tokens whose lowercased text is a keyword**,
which is what it was always reaching for. All four defects fall out at once:
string literals lex as string tokens, comments never reach the token stream,
`Ineligible` lowercases to a non-keyword, and iterating tokens reports every
occurrence rather than the first.

Keyword-ness is decided by lexing the lowercased word and checking it yields a
single non-identifier token — so the rule tracks the lexer instead of a
hardcoded list that drifts. The 26-keyword array is gone.

## The mechanic that was load-bearing

The lexer merges adjacent identifier words into one multi-word `Identifier`
token: `STORE counter` lexes as `Identifier("STORE counter")`. Checking the
whole token text would have found no keyword — and silently deleted the lint
again, in a second way.

So the rule slices `byte_start..byte_end` out of the source and checks each
whitespace-separated word, offsetting the column accordingly. A multi-word
identifier cannot span lines (a newline flushes it), so the line is the token's
and the column is exact.

## Red evidence

5 of 10 linter tests failed against the old rule. The incidental one is the
nicest: the three-occurrence case returned two diagnostics — `STORE` once, plus
a bogus `TO` matched *inside the word* `STORE`.

```text
---- test_keyword_casing_ignores_string_literals stdout ----
string literal contents must not be linted, got [... message: "Keyword 'NO' should be lowercase" ...]
```

Red: `74114dc`. Green: `f6ff1f1`. 10/10 after.

## Known limitation, unchanged by this work

`--lint` runs after parsing, so a mis-cased keyword that *breaks* parsing never
reaches the linter — `STORE alpha as 1` exits 2 with a parse error. LINT-KEYWORD
therefore only fires on mis-cased words that still parse, e.g. `store Count as 5`.
That is pre-existing CLI behavior, identical before and after, and is recorded
here so the rule's real reach is not overstated.
