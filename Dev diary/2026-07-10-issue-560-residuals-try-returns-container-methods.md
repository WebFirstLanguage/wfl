# Issue #560 Residuals: Returns Inside `try` Blocks and Container Method Results No Longer Typed `Nothing`

**Date:** 2026-07-10

## Background

Issue #560 ("unannotated action return types default to `Nothing`") was fixed
in two rounds: #575 added post-body return-type inference for top-level
actions, and #591 seeded the provisional return type as `Unknown` so
self-recursive calls degrade gracefully. Working on Scriptorium surfaced the
same false `Cannot index into Nothing` diagnostic again, so this pass hunted
down the remaining shapes that still slipped through. Two were found and
fixed; both are static-diagnostics-only bugs (runtime was always correct).

## Residual 1 — `return` inside a `try:` block was invisible to inference

`collect_return_types` (`src/typechecker/mod.rs`) descends into conditionals
and loops to find an action's `return` statements, but it never descended into
`Statement::TryStatement`. An action whose only returns live inside a `try:`
body, a `when error` clause, an `otherwise` clause, or a `finally` block was
inferred as returning `Nothing`, and indexing its call result raised the false
error:

```wfl
define action called load_data:
    try:
        return [1 and 2]
    when error:
        return [3 and 4]
    end try
end action

store xs as call load_data
store x0 as xs[0]      // error[ERROR]: Cannot index into Nothing (false)
```

This is a very common shape — try blocks wrap exactly the file/database/parse
work that helper actions return values from, which is why Scriptorium hit it.

**Fix:** `collect_return_types` now descends into `TryStatement` (body, every
`when` clause, `otherwise`, `finally`) and `WaitForStatement` (its wrapped
inner statement). Its sibling `check_return_statements` — the traversal used
when an annotation *is* present — got the same arms so the two stay in sync.

## Residual 2 — container method results were registered as `Nothing`

The analyzer registers container methods in its container registry with

```rust
return_type: return_type.as_ref().cloned().unwrap_or(Type::Nothing),
```

and the type checker's `Expression::MethodCall` arm reads that registry to
type `instance.method()`. Unannotated value-returning methods (the norm) were
therefore typed `Nothing` at every call site — the container-flavored twin of
the original #560, which #575 only fixed for top-level actions:

```wfl
create container Store:
    property label: Text
    action get_items:
        return [1 and 2]
    end
end
...
store xs as s.get_items()
store x0 as xs[0]      // error[ERROR]: Cannot index into Nothing (false)
```

**Fix,** mirroring the #575 + #591 design for top-level actions:

- The analyzer seeds unannotated instance *and* static methods with a
  provisional `Type::Unknown` (degrades gracefully after #588/#589) instead of
  `Type::Nothing`.
- The type checker's `ContainerDefinition` arm now checks each method body
  with the method's parameters in scope (mirroring the top-level action arm's
  #553 handling), infers the real return type from the body's `return`
  statements, and writes it back into the registry through a new
  `Analyzer::get_container_mut`. Void methods still end up `Nothing`, exactly
  as before.
- Inherited methods get the fix for free: the `MethodCall` parent-walk reads
  the same registry entries.

## Verification (TDD)

`tests/action_return_type_residuals_test.rs` was written first and confirmed
failing (4/4) before the fix:

- return inside `try`/`when error` infers the return type
- return inside `try`/`when`/`otherwise` infers the return type
- unannotated container method result is not typed `Nothing`
- inherited container method result is not typed `Nothing`

All four pass after the fix, alongside the full `cargo test --all` suite,
`cargo clippy --all-targets --all-features -- -D warnings`, and the
`TestPrograms/` backward-compatibility run. A combined Scriptorium-shaped
stress case (recursive action indexing a map returned from inside `try`,
wrapped by a container method) type-checks completely clean.

## Notes

- No syntax, keyword, or runtime behavior changed — this is purely a
  false-positive-diagnostics fix, so no user-facing docs needed updating.
- Remaining known limitation (unchanged by this pass): a method call typed
  *before* its container's definition statement is reached in program order
  still sees the provisional `Unknown` — safe (permissive) but not concrete.
