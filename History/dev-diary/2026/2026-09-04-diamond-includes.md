# 2026-09-04 — Diamond includes: the second branch always broke

## Symptom

A multi-file program where two library files both `include from` the same
shared file could not run:

```text
            util.wfl
           /        \
     auth.wfl      render.wfl
           \        /
            main.wfl
```

```text
error[ERROR]: Semantic error in included file 'render.wfl':
  Semantic error at line 3, column 21: 'shout' is not a function
```

The first branch (`auth.wfl`) worked; the second (`render.wfl`) failed while
being *analyzed*, before a single line of it ran. Verified on wfl 26.9.2.

A downstream project had noticed this and concluded that "WFL includes form a
tree, and diamonds break", working around it by chaining every file into one
long line (`util <- db <- auth <- render <- site_ext <- main`). The chain works
only by accident: with every `include from` at the top of its file, nothing
is defined yet when each file is analyzed, so unknown names are downgraded to
warnings and resolve at runtime.

## Root cause

Two separate defects, one hiding the other.

1. **Parent-scope actions were seeded into an included file's analyzer as
   plain variables.** `extract_parent_variables` turned every runtime binding
   into `SymbolKind::Variable`, actions included. Calling one of those
   (`shout of title`) then hit the analyzer's "'shout' is not a function"
   error, which is fatal for included files. This broke any second file that
   used an action an earlier include had defined — the diamond, but also the
   plain sibling order where `render.wfl` does not include `util.wfl` itself.

2. **Re-including a file re-ran it into the same scope.** Once (1) is fixed,
   the second arrival at `util.wfl` executes `store util_loads as 1` and
   `define action called shout` again in a scope that already has both:
   "Variable 'shout' has already been defined at line 0". The cycle check
   only knew about files *currently* loading, not files already finished.

## Fix

- The interpreter now snapshots the enclosing scope as typed variables
  **and** action signatures (`snapshot_parent_scope`), and the analyzer gets
  the actions through `register_parent_actions` as real function symbols
  with their true parameter lists. A same-name definition in the analyzed
  file is treated as an overload under the existing distinctness rules,
  which matches what the runtime already did.
- `Environment` records the canonical paths `include from` has completed in
  that scope. An include whose file is already visible from the current
  scope (recorded there or on an ancestor) is a no-op. Only a file that ran
  to completion is recorded, so a failed include can be retried. `load
  module` is unchanged: it exists to run a file for its side effects.

Behavior that changes: a file included twice into the same scope used to
run twice if it contained only side effects (anything with a definition
already failed). It now runs once; `load module from` is the documented tool
for "run this file every time".

## Evidence

- Red: `tests/include_diamond_test.rs` (5 tests) and
  `TestPrograms/modules/include_diamond.wfl` fail on the unmodified
  interpreter with the errors quoted above.
- Green: same tests pass after the change; `cargo test --workspace`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and the
  gated `TestPrograms/` run are clean.
- Risk class R3 (backward compatibility). Negative paths covered: a genuine
  include cycle is still rejected; an include inside an action body still
  runs per call when nothing enclosing has included the file.

## Residual

The type checker does not follow includes, so a container defined in a
shared file still produces non-fatal "Container type 'X' not found" warnings
in a sibling file that instantiates it. That predates this change and the
program runs correctly; it is a separate issue.
