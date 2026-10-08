# Analyzer accepts documented `path of req` inside action bodies

`Docs/04-advanced-features/web-servers.md` tells users to read request fields
inside actions with `path of req` (and `method`/`query`/`body`/`body_bytes`)
so handlers stay self-contained. The runtime already treats a one-argument
`of` form as a property read on the request object. The analyzer parsed that
form as a call to an undefined `path` action and reported a fatal
`Variable 'path' is not defined`, so `wfl` never ran the program. That is
issue #647.

`FunctionCall` now recognizes the request-object field names used as a
one-argument `of` callee and analyzes the object argument instead of
reporting undefined / "not a function". Bare `path` inside an action stays
undefined — only the `of` form is property access, matching the docs.

Red tests in `tests/request_of_action_body_test.rs` failed first (commit
4c2769e1): the documented snippet, `wfl --analyze` on it, and guards that
bare `path` and an unknown `of` callee stay fatal. After the helper those
tests pass (the analyze CLI fixture also uses every stored binding so
`ANALYZE-UNUSED` does not fail `--analyze`).

Review of that green found two leftover gaps. After `wait for request`, the
analyzer's leftover scope binds `path` as Text, so the type checker inferred
the `of` callee as a function and warned `Cannot call Text` on a normal
`wfl` run. `ambiguous_auth_headers` is on the request object but was not in
the accepted field list. `request_object_property_type` is now shared by
the analyzer and type checker (including Boolean `ambiguous_auth_headers`).
Red tests for those two cases failed first (commit c3e97f21).
