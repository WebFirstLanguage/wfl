# Keyword references match the lexer again, and a test keeps them there

The two keyword reference pages, `Docs/reference/reserved-keywords.md` and
`Docs/reference/keyword-reference.md`, said WFL had "181 keywords (54
structural, 29 contextual, 96 other, 7 literals)". `CLAUDE.md`, the language
specification, the variables guide and the naming guide repeated that figure.
The lexer had moved on. Test-framework words (`describe`, `test`, `expect`,
`setup`, `teardown`, `be`, `have`, `exist`, `contain`, `empty`), module words
(`include`, `export`), binary I/O words (`binary`, `bytes`), the text operators
`starts` and `ends`, the filler word `that`, `parameters`, and the multi-word
token `divided by` were all real tokens that the alphabetical table did not
list. Meanwhile the table listed `downward`, `upward` and `pop`, which were
never keywords: `pop` is a standard-library list function, and the other two
have no special meaning at all.

## What the release binary says

Every lexer word was checked with the release binary (`store <word> as 1`, and
`display <word>` after it). The results match `Token::is_structural_keyword`,
`Token::is_contextual_keyword` and `Parser::parse_variable_name_list` exactly:

| Type | Count | `store <word> as 1` |
|------|-------|---------------------|
| Structural | 63 | `Cannot use reserved keyword …` (for `as`: `Expected a variable name before 'as'`) |
| Contextual | 32 | Accepted for 31 words, and `display` reads the value back |
| Other | 96 | `Expected identifier for variable name, found …` |
| Literal | 7 | `Expected identifier for variable name, found …` |

That is 198 words. The old "29 contextual" counted `any`, `push`, `skip`,
`than` and `zero`, which appear in both lists in `token.rs` but behave as
structural because the parser checks structural first. The pages now count
them once, as structural, so the four types add up to the total.

One contextual word cannot be declared with `store`: `new`. `store new` always
begins `store new constant NAME as value`, so `store new as "fresh"` fails with
`Expected 'constant' after 'new'`. The old pages said it worked.

## Smaller corrections found on the way

- `is` is an "other" keyword, not structural. It is just as reserved; only the
  error message differs.
- Two contextual examples, `store extensions as list` and `store files as
  list`, failed with `Variable 'list' is not defined`. They now use list
  literals.
- `create list called items` is not valid syntax. The examples now use
  `create list items: … end list`.
- Twenty section headings stated counts that did not match their own lists
  (for example "Pattern Matching (26)" listed 24 words), and `comes` was listed
  in two "other" subsections. The quick reference now uses the same fourteen
  feature categories as the complete reference, including new Modules and
  Testing groups.

## The drift guard

`tests/suite/keyword_reference_drift_test.rs` reads every `#[token("word")]`
in `src/lexer/token.rs` plus the boolean-literal regex words. It classifies
each word with the real `Token` methods and parses `store <word> as 1` with the
real parser. It then checks:

- the alphabetical table lists exactly the lexer words, with the right Type
  and "As Var?" value;
- the four type sections list exactly their words;
- every lexer word appears in the quick reference with the right ✓/✗ mark;
- every heading that states a count matches the keywords listed under it, and
  lists no word that is not a keyword;
- the totals in both reference pages, the language specification, the two
  guide pages and `CLAUDE.md` match the lexer.

The test was committed on its own first and failed against the old pages for
exactly the reasons above. Adding a fake `#[token("frobnicate")]` to the lexer
makes four of its six tests fail and name the missing word, so a new keyword
can no longer land without its documentation.
