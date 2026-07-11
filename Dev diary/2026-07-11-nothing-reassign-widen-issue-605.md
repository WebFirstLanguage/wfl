# Issue #605: Nothing-Initialized Variables Widen on Reassignment

**Date:** 2026-07-11

## Background

The static type checker reported a false `Cannot index into Nothing` when a
variable was initialized to `nothing`, later reassigned via `change`, and then
indexed. Runtime was always correct; the diagnostic cluttered load-time output
(notably wfl-web's admin media-upload path).

```wfl
store file_part as nothing
for each part in parts:
    change file_part to part
end for
store file_bytes as file_part["content_bytes"]  // false: Cannot index into Nothing
```

## Root causes

Two cooperating bugs:

1. **Pinned `Nothing` on assignment.** `Statement::Assignment` checked
   compatibility against the existing type but never updated it. After
   `store x as nothing`, `x` stayed `Nothing` forever, even when
   `change x to <map-or-value>` ran successfully at runtime. Assigning a
   concrete non-Nothing value onto Nothing was also rejected as incompatible.

2. **`get_symbol_mut` only mutated the innermost scope.** Loop bodies push a
   child scope. Even after (1) tried to widen the outer variable, the update
   hit only the current scope's map (where the outer name is not bound) and
   was silently dropped. On `pop_scope` the outer binding was still `Nothing`.

## Fix

- On `change` of a `Nothing`-typed variable, replace the stored type with the
  assigned expression's type (skipping `Error`; leaving pure `Nothing` alone).
  Concrete type mismatches on non-Nothing variables still error (e.g. Number →
  Text).
- `Scope::resolve_mut` / `Analyzer::get_symbol_mut` walk parent scopes so type
  refinements from loop/try bodies update the real binding and survive
  `pop_scope` (the boxed parent clone is what pop restores).

Unassigned `nothing` still rejects indexing — only reassignment widens.

## Verification (TDD)

`tests/nothing_reassign_widen_test.rs` was written first and confirmed failing
(3/5) before the fix:

- nothing → reassign in for-each → index (issue minimal repro)
- nothing → change to map → index (wfl-web shape)
- nothing → change to text is allowed
- nothing without reassign still rejects indexing
- Number → Text reassignment still errors
