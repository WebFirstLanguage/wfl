# Dev Diary — 2026-07-18: Bare zero-arg include-exposed action reference (#592)

## Context

This is the first increment of **Phase 2 — Language correctness** under the
production-readiness tracker (#610), advancing the workstream *"Make main-file
and included-module semantics consistent"*.

A zero-argument action exposed by an `include from` file, referenced by its
**bare name** (no `of`, no `call`), was a **fatal** analyze error:

```wfl
// mod.wfl
define action called greet:
    return "hello from greet"
end action

// main.wfl
include from "mod.wfl"
store x as greet        // error[ANALYZE-SEMANTIC]: Variable 'greet' is not defined
display x               // never runs; exit 3
```

The identical `store x as greet` runs fine in a *single* file, and both
`call greet` and `greet of "x"` (for an action taking an argument) already
worked across an include. So the same code was fatal or fine depending only on
whether the action came from an included file — exactly the main-vs-include
inconsistency Phase 2 is meant to remove. This was the third and last surviving
form of the #548 → #580 include-resolution family (#580 fixed the `of` and
`call` forms).

## Root cause

`greet of "x"` (a `FunctionCall` with a bare-`Variable` callee) and
`call greet with "x"` (an `ActionCall`) both route their unresolved-callee
handling through one include-aware helper, `warn_undefined_callee_if_includes`
(`src/analyzer/mod.rs`). When the program uses `include from`, that helper
downgrades the fatal error to a **non-fatal** `Undefined action '<name>'`
warning (the action may be exposed by the included file at runtime, which the
analyzer never reads) and returns `true`; with no includes it emits nothing and
returns `false`, preserving the fatal path so genuine typos stay caught.

A **bare** reference (`store x as greet`) lowers to `Expression::Variable`
with no call node, so it never reached that helper — the `Expression::Variable`
arm called `report_undefined_name` directly, which is fatal. The rest of the
pipeline already tolerated the bare reference: the type checker returns
`Type::Unknown` for an unknown name and defers to the analyzer, and the
interpreter already auto-calls a zero-argument action referenced by bare name
(which is why it worked in a single file and inside a `main loop`). The defect
was therefore purely the analyzer aborting first.

## What changed

One production change: the `Expression::Variable` arm in
`Analyzer::analyze_expression` now routes an unresolved, non-container-property
name through the **same** `warn_undefined_callee_if_includes` helper the
`of`/`call` forms use, and only falls back to the fatal `report_undefined_name`
when the helper returns `false` (i.e. no includes present). No new logic — it
reuses the exact relaxation #580 unified the other two forms onto, so the three
call forms cannot drift apart again.

Behavior now (verified end-to-end on the release build):

- bare reference **at top level** → non-fatal `Undefined action` warning, runs, exit 0;
- bare reference **inside an action body** → same;
- bare undefined name **without any include** → still fatal (`Variable '…' is not defined`, exit 3).

## Regression protection

- `src/analyzer/mod.rs` unit tests: `test_bare_undefined_name_relaxed_with_includes_issue_592`
  (with an include → warning, not error) and
  `test_bare_undefined_name_without_includes_stays_fatal_issue_592` (guardrail: still fatal).
- `tests/phase1_correctness_regression_test.rs`: the previously `#[ignore]`d
  `issue_592_*` acceptance tests are now **active** guards (top level + action
  body), plus a new `issue_592_bare_undefined_without_include_stays_fatal`
  no-include guardrail. The Phase 1 coverage map in that file is updated from
  "High (open) ⏳" to "High (fixed) ✅".
- End-to-end fixture: `TestPrograms/module_include_bare_zero_arg.wfl` (with its
  helper `module_bare_zero_arg_helper.wfl`, skip-listed like `module_helper.wfl`
  in both `run_integration_tests.sh` and `.ps1`).

## Documentation

`Docs/04-advanced-features/modules.md` gains a "Calling actions from an included
file" subsection documenting all three call forms — including that a
zero-argument included action is referenced by its bare name — and honestly
notes the non-fatal `Undefined action` warning the analyzer emits for a name it
cannot see statically.

## Compatibility

Backward compatible: the change only **relaxes** a currently-fatal error when
`include from` is present, and preserves the fatal path (and the `try`-body
warning downgrade) for every other case. No existing `TestPrograms/` behavior
changes.
