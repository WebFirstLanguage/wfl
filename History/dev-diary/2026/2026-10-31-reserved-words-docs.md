# 2026-10-31 — Explain reserved words, and point agents at them first

`start`, `output`, `route`, `captured`, `by`, and `find` are reserved, and they
kept surprising people and agents. Earlier diary entries show the pattern: the
pattern kitchen-sink experiment stopped running because it stored a variable
named `output` (2026-10-08), and two capture fixtures had to be renamed to
avoid `captured` (2026-10-09). The keyword tables listed all six words, but
nothing said what "reserved" means or why WFL reserves words, and `CLAUDE.md`
mentioned reserved words only inside the documentation-writing section. Agents
found the rule by hitting a parse error.

## What changed

- `Docs/reference/reserved-keywords.md` now opens with **What Is a Reserved
  Word? (Start Here)**: the naming positions a reserved word cannot fill, why
  WFL reserves words (words replace punctuation, so each grammar word must have
  one meaning; early errors; stable meaning across versions; readable code),
  the cost and how contextual and marker words keep it small, and how to read
  the error, which names the word as a `Keyword...` token.
- A new **Words That Commonly Bite** section covers each of the six words: the
  constructs it powers, and suggested underscore names. A runnable
  side-by-side example uses `start`, `captured`, `by`, and `find` in their real
  jobs. Pitfall 5 covers reserved words inside spaced names.
- The quick reference, error codes, troubleshooting, variables, naming
  conventions, and routing pages link to those sections and list the six
  words. The alphabetical table now shows real syntax for the six words.
- `CLAUDE.md` gained **Reserved Words — Check Before You Name Anything** near
  the language overview, a Critical Rules bullet, and the six words in the
  existing syntax bullet.

## What was verified

All claims were checked against the release binary before writing:

- Each word fails as a `store` name, a `for each` variable, an action
  parameter, and an action name (exit 2, parse error at the word). Spaced names
  such as `store start time as 1` fail the same way. Every suggested
  replacement name works.
- Three message forms exist. Structural words (`route`, `by`, `when`) produce
  `Cannot use reserved keyword 'Keyword...' as a variable name`; the others
  produce `Expected identifier ... found Keyword...`. A reserved parameter name
  produces `Expected ':' after action definition: expected Colon, found
  Keyword...`, which does not mention names at all.
- `wait for read output from process child as output` still parses and runs.
  Existing tests in `tests/suite/subprocess_cleanup_test.rs` use it, so the
  docs describe it as the one narrow exception. `store output` and
  `change output` are still rejected.

`tests/suite/reserved_common_words_test.rs` pins these points and checks that
the docs side-by-side block is byte-identical to
`TestPrograms/docs_examples/keyword_reference/common_reserved_words.wfl`
(registered in the docs-examples manifest). The behavior already existed, so
this is characterization coverage for an R0 documentation change. To show the
tests are not vacuous, adding the non-reserved `count` to the word list made
the rejection tests fail, and editing one line of the docs block made the
drift test fail. Both edits were reverted.

## Found along the way (not fixed here)

- The keyword pages say "181 keywords", but `src/lexer/token.rs` defines 194
  word tokens. About 19 of them (`test`, `describe`, `expect`, `include`,
  `export`, `empty`, and others) are missing from the alphabetical table. The
  new text avoids repeating the count. A recount is a separate audit.
- `find all <pattern> in <text>` is documented in the pattern module page but
  does not parse: the lexer merges `all <name>` into one identifier. The new
  text does not mention `find all`. Reading a field of a `find` result
  (`.matched_text` or `["matched_text"]`) also gives a type-checker warning,
  so the example checks `isnothing of` instead.
- The error messages show internal token names. A friendlier message (for
  example "`start` is a reserved word; try `start_time`") would follow
  Principle 4. That is a behavior change and needs its own Red→Green work.
