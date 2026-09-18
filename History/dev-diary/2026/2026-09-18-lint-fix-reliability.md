# September 18, 2026 — Reliable lint and fix commands

The documented `wfl --lint --fix program.wfl` invocation was rejected because
`--lint` consumed its next argument as a filename before recognizing `--fix`.
The CLI now accepts lint options before or after the path, keeps the older
repeated-path invocation working, and rejects conflicting modes before side
effects. Lint returns 0 for clean input, 1 for findings, and 2 for usage/input
errors. Fix previews return source or a usable unified patch without logging
mixed into stdout; completed fixes return 0 even if manual lint work remains.

The previous fixer reconstructed source with an incomplete AST printer. It
discarded comments, changed escaped literals and expression grouping, lost
newer statement forms, and could emit invalid WFL. File formatting now makes
targeted source edits and validates the result before writing. Comments,
literal spelling, line endings, and syntax are preserved. Local renames avoid
collisions, keywords, public APIs, module bindings, map keys, and pattern
capture names. Long lines and deep nesting still require human refactoring.

In-place output uses a sibling temporary file and atomic replacement after a
complete write/flush/sync. Read-only and stale-source failures leave existing
contents unchanged. Lint and fix share token-based block layout and respect
the configured indentation and style switches. Lint also applies configured
length/depth limits, traverses nested statements, measures Unicode line length
in characters, and avoids treating string contents as indentation or trailing
whitespace errors.

Regression tests cover CLI modes, valid patch application/reversal, malformed
and oversized source, extreme indentation configuration, source preservation,
and actual execution before and after formatting. A corpus test verifies
syntax, literal spelling, and idempotence for every currently parseable WFL
program under `TestPrograms`. The old concatenation unit test required a line
break after `with`, which the parser does not accept; its replacement asserts
parseable output and exact expression preservation.

See [verification evidence](../../../Engineering/evidence/2026-09-18-lint-fix.md)
for the recorded Red revisions, acceptance coverage, and validation results.
