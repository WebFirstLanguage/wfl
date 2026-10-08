# ANALYZE-UNUSED now counts every statement operand as a use

`expect` already marked its subject and expression-valued operands (see
2026-09-25). The unused-variable visitor still ended in `_ => {}`, so
reads inside `create file`, `create list`, container instantiation,
process/HTTP/include statements, listen redirect ports, and several
expression kinds (`file exists at`, database queries, …) were reported
unused. That is issue #711 — the unfinished sweep #468 asked for.

`mark_used_variables` and `mark_used_in_expression` are now exhaustive.
Variants that carry no reads have explicit empty arms, so a new
`Statement` or `Expression` kind is a compile error instead of a silent
false positive. Nested declaration collection also walks container
methods, event handlers, websocket handlers, `wait for`, and
repeat-while/until so unused bindings inside those blocks stay visible.

Red tests in `tests/analyzer_unused_statement_uses_test.rs` failed first
(commit e6b4572c): the issue reproductions, a sweep of previously
unhandled operands, `wfl --analyze` on the combined repro, and a guard
that gated `TestPrograms/**/*.test.wfl` emit no `ANALYZE-UNUSED`.
Genuinely unread bindings still warn. After the exhaustive visitor those
tests pass.
