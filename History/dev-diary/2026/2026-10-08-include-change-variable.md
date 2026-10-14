# ANALYZE-SEMANTIC no longer fatals `change` of an include-exposed variable

`include from` already shared mutable bindings at runtime. Reading an included
name was a non-fatal warning (#592), and `store` of the same name ran and
mutated the shared slot. `change` of that name was a fatal
`Variable '<name>' is not defined` that stopped the program (exit 3). That is
issue #708 — the assignment-target case the #548 → #580 → #592 include-aware
relaxation never covered.

`Assignment` now routes an unresolved target through
`warn_undefined_variable_if_includes`, the assignment counterpart of
`warn_undefined_callee_if_includes`. With `include from` present the fatal
becomes a non-fatal `Undefined variable '<name>'` warning (a variable message,
not `Undefined action`). Without includes the existing fatal stays. The CLI
does not drop the warning after scanning included files: that scan only
collects actions.

Red tests in `tests/include_change_variable_test.rs` failed first (commit
a4466b13): the issue repro (`no` then `yes`, exit 0), the included action
observing the caller's `change`, the no-include fatal guard, and an analyzer
check that the warning is a variable warning. After the helper (commit
50c2e91f) those tests pass. `Docs/04-advanced-features/modules.md` documents
the warning and the working `change` of an included variable.
`TestPrograms/modules/include_change_variable.wfl` is the gated end-to-end
example (`change` at top level after `include from`; `describe`/`expect`
asserts the shared binding).
