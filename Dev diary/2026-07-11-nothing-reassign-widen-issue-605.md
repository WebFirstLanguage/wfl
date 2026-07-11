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

## Follow-up (PR #606 review)

1. **When-clause scope (Devin).** Parent-walking `get_symbol_mut` made a latent
   TryStatement gap more dangerous: the type checker used to call
   `get_symbol_mut` on a `when` error name without a child scope, so a
   colliding outer name could be rewritten to `Text`. Fixed by pushing a
   when-clause scope and binding the error name (and `error_message` alias)
   with `define_or_replace_symbol` so only the child scope is written —
   matching runtime's `Environment::define_or_replace`.

2. **Action / method bodies must not permanently widen outer bindings
   (Codex).** Type-checking an action definition walks its body; with parent-
   walking mutability that could refine outer `nothing` bindings even if the
   action is never called. Snapshot symbol types before the body and restore
   after (top-level actions and container methods). Loop bodies still keep
   refinements — that is the intended #605 fix for the idiomatic pattern.

3. **Loop test (CodeRabbit minor).** Loop regression no longer indexes a
   list-of-text element as a map; it uses `length of item` after the widen.

Covered by `test_when_error_name_does_not_clobber_outer_variable_type` and
`test_action_body_does_not_permanently_widen_outer_nothing`.
