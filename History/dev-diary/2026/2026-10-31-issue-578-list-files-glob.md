# `list files ... with pattern` matches globs again

`list files in "." with pattern "*.txt"` returned an empty list and exited 0,
even with `.txt` files in the folder. An exact name such as `"report.txt"`
failed the same way, and so did `list files recursively ... with pattern`.
The docs had always promised a glob. This was the first confirmed functional
bug in umbrella issue #578, re-verified on 26.9.19.

The cause was one layer above the filesystem. The parser read `with pattern`,
`with extension` and `with extensions` into the same AST node, so the
interpreter could not tell them apart. It compared every value with the
file's `.ext` string, and `"*.txt"` never equals `".txt"`. Only a bare
extension like `".txt"` happened to work.

`ListFilesFiltered` and `ListFilesRecursive` now carry a `FileFilterKind`
(`Extension` or `Pattern`). The new `src/interpreter/file_filter.rs` compiles
the values once per listing. `with extension(s)` keeps its exact extension
match. `with pattern` matches each file name with `glob::Pattern`, the crate
the `glob` function already uses, so `*`, `?`, `[abc]`, `[a-z]` and `[!abc]`
work. It is case-sensitive and includes dot-files, the same as `glob`.

Two compatibility choices:

- A `with pattern` value that equals a file's extension still matches it, so
  `with pattern ".txt"` returns what it did before. No file the old code
  returned is dropped.
- An invalid glob such as `"[abc"` used to give an empty list. It now raises
  a catchable runtime error that names the pattern, which shows the mistake
  instead of hiding it. That program was already broken, but it now stops
  where it used to carry on with an empty list.

Red tests failed first in commit 22ffb93: the new
`tests/suite/list_files_pattern_test.rs` (2 passed, 8 failed, every failure an
empty listing or a missing error) and the re-enabled
`issue_578_list_files_with_pattern_matches`. The two passing tests were the
backward-compatibility guards, which pass before and after. With the fix all
of them pass. Unit tests cover the filter rules and the parser's filter kind.
`TestPrograms/list_files_pattern.test.wfl` is the gated end-to-end program; it
builds its fixture under `target/test-artifacts/` and removes it in
`teardown`. The pattern rules are documented in
`Docs/05-standard-library/filesystem-module.md#pattern-rules`, with a validated
example in `TestPrograms/docs_examples/stdlib_functions/list_files_pattern.wfl`.
