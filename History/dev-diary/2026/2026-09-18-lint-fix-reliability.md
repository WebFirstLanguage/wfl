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

## September 18 review follow-up

PR #735 review reproduced false indentation after `export action` and rejected
single-dash filenames, including version-alias spellings in legacy positions.
The follow-up retains failing regressions before correcting both behaviors.
A reported unterminated single-line conditional was rejected by the existing
parser, so it did not require a formatter or grammar change.

Concurrent WFL in-place fixes now use an exclusive sibling lock across source
validation and replacement. This coordinates formatter processes without
claiming to lock unrelated editors. Normal completion releases the lock;
abnormal termination leaves a visible lock that requires confirming the owner
has stopped before removal. Regression coverage checks contention, independent
destinations, and recovery, with the details recorded in the evidence document.

### Additional legacy-constant review

A further review found that the colon in a named argument of the supported
`create new constant` spelling could be mistaken for a container body. Layout
now recognizes the actual container-instantiation header before opening a
block. Retained failing unit and real CLI regressions cover the old constant
syntax, nested use, real container initialization, and all lint/fix outputs.

### September 18 interface and list-expression review

A subsequent review reproduced false block nesting when a bare interface was
followed on the same line by a statement with a named argument. The adjacent
grammar audit found the same problem in the supported empty-list expression
and contextual `create map` / `create pattern` display operands.
Regression coverage distinguishes these forms from actual interface and list
bodies and verifies all CLI lint/fix modes.
The shared layout scanner now checks the colon at the end of each affected
declaration header, including an interface's optional parent list. Test-only
revision `877a1dd6` preserves the ten failing regressions before the correction.
Independent review checked the implementation against the parser's accepted
grammar and found no further defect.

### September 18 expression-role and option-order review

Fresh reviews found contextual expression words changing block nesting and
reordered output-mode options treating version-alias filenames as version
requests. Retained Red revisions cover the actual lint, source-preservation,
CLI output, and error/no-write failures. The documentation follow-up explains
the affected API and helper contracts in response to the current docstring
coverage warning.

Layout now consults expression roles from the parsed program while preserving
the token scanner's physical-line behavior. Contextual words remain operands,
real route and WebSocket headers remain active, and bodyless event registrations
remain outside the block stack. The visitor uses explicit worklists and checks
source spelling to distinguish synthetic parser nodes. Reordered CLI mode
flags now recognize alias filenames and retain input-validation failures.
Explicit action calls named `main` also require this distinction: only a parsed
main-loop start opens that body, with both serial and concurrent loop fixtures
retaining their indentation. Final local validation passed 2,382 workspace
tests; the earlier standalone SQLite timeout remains retained and unexplained.

### September 18 transaction and pattern-header review

A fresh review and the accompanying opener audit found two more shared-word
cases: `in transaction` in a pattern expression and `check` in a pattern
lookaround. Both incorrectly changed the formatter's nesting stack. Retained
failing tests cover expression and genuine-block forms at library and CLI
boundaries. Layout now verifies the parsed opening position for transaction
and conditional bodies, preserving pattern assertions and ordinary text
variables without changing either parser or runtime behavior.
