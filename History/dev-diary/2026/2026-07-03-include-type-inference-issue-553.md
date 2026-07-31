# Include Type Inference: Remaining RHS Forms (Issue #553)

**Date:** 2026-07-03

## What Changed

Issue #553 (follow-on to #551/#552) reported that four right-hand-side forms
still aborted with a fatal `Could not infer type for variable 'v'` when they
appeared inside an `include from` file:

```wfl
store v as parts[0]              # list index
store v as rec["k"]              # object index (parse_json result)
store v as a is equal to b       # comparison result
store v as length of a           # length of (already fixed by #552's builtin table)
```

Investigating end to end showed the failures were not include-specific at all.
The same forms failed inference in the **main file** too — but `main.rs`
reports type errors as non-fatal warnings and keeps running, while the include
pipeline turned the first type error into a fatal `RuntimeError`. That
asymmetry is what made includes look uniquely broken.

### Root causes and fixes

1. **The type checker had no scope for action bodies**
   (`src/typechecker/mod.rs`). The analyzer creates a child scope for each
   action body during analysis and then discards it, so when the type checker
   later walked the AST, neither parameters nor body-local variables resolved
   to any symbol. `store parts as string_split of a and "-"` inferred
   `List<Text>` but had nowhere to record it, so `parts[0]` on the next line
   inferred `Unknown`. The `ActionDefinition` branch now pushes a scope,
   defines parameter symbols (declared type, or `Unknown` when untyped), and
   the `VariableDeclaration` branch defines body-local symbols with their
   inferred types so later statements can see them.

2. **Comparisons with `Unknown` operands inferred `Unknown`.** A comparison
   yields a Boolean no matter what the operand types turn out to be; only
   arithmetic results depend on the operand types. `BinaryOperation` inference
   now returns `Boolean` for `Equals`/`NotEquals`/ordered comparisons/
   `And`/`Or`/`Contains` even when an operand is `Unknown` or `Any`.

3. **Indexing an `Any` collection was a type error.** `parse_json` results
   are typed `Any` (the shape is only known at runtime), but `IndexAccess`
   had no `Any` arm and fell through to `Cannot index into Any`. Indexing
   `Any` now yields `Any`, and `Unknown`/`Any` index values are tolerated
   for list and text indexing.

4. **Include type errors are now non-fatal warnings**
   (`src/interpreter/mod.rs`). `include from` executes in the parent scope —
   the code is semantically part of the main program — so its type-check
   findings are now reported exactly like the main file's: printed as
   `Type checking warnings in included file '...'` and execution continues.
   This closes the class of bug behind #551/#553 rather than the four
   instances: any future inference gap degrades to a warning instead of a
   show-stopping abort. Parse and semantic errors in included files remain
   fatal.

5. **Bonus: false `ANALYZE-UNUSED` fix** (`src/analyzer/static_analyzer.rs`).
   The issue's own repro flagged `Unused variable 'parts'` after
   `store v as parts[0]` inside an action: `mark_used_variables` had no
   `VariableDeclaration` arm, so right-hand sides of `store` statements inside
   action/loop bodies never counted as uses. Added the arm.

### No new syntax

No new syntax was needed, so nothing was added to the language surface. The
change is purely semantic and aligns with the WFL foundation principles of
clear and actionable error reporting (principle 4) and type safety with
inference where practical (principle 5): inference now understands the
bread-and-butter forms, and what it cannot infer degrades to a readable
warning instead of a fatal abort.

## Tests

`tests/docs_parser_and_include_fixes_test.rs` gained regression tests: each
of the four RHS forms inside an included action called from a main program,
main-file inference guards for list-index and comparison results, and a guard
that a genuinely uninferable store (`a plus b` on untyped parameters) in an
included file runs to completion instead of aborting.

## Docs

- `Docs/04-advanced-features/modules.md`: documented include type-check
  behavior (warnings, not fatal).
- `Docs/reference/error-codes.md`: refreshed the "Could not infer type"
  entry.
