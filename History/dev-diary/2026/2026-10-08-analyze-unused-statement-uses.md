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

Review of the first green found seven new arms marking HTTP/process
*output* names as used. Those names are writes, not reads, and are not
collected as declarations, so the mark only hid a prior unread `store`
of the same name. The streaming HTTP arms already skip `variable_name`.
The process/HTTP arms now match them. Negative tests for a stored name
rebound as an output and never read failed first (0401c26c).

Review of that green found a real scope collision: collecting container
methods into the same name-keyed map let a method-local replace an outer
binding (and the reverse). Reproduced both ways (`value` unused at top
level while a method reads its own `value`; outer `item` read while a
method `store item` is unread). Action, method, event-handler, and
websocket-handler bodies now overlay their declarations and restore the
parent map. `export constant X` marks `X` used. The gated
`TestPrograms/**/*.test.wfl` sweep fails if any file does not parse.
`Docs/contributing/compiler-internals.md` documents `ANALYZE-UNUSED`
(there is no dedicated diagnostic page). Red tests for the scope and
export cases failed first (42c9fae8).

Isolating method scopes then flagged `store completed as yes` in the
docs Task Manager example: that `store` assigns the container property,
which is already bound in the method environment. Method analysis now
predeclares property names so a property write is not a new unused
local. Red: `9f89c168`.
