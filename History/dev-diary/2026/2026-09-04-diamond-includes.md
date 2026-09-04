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
  that scope **and that installed at least one definition there**. An
  include whose file is already recorded for the current scope (or an
  ancestor it can see) is a no-op, decided before the import-depth ceiling
  is charged. Recycled loop scopes clear the record along with their
  values. A file that defines nothing is never recorded, so a side-effect-
  only file keeps running on every include — no existing program changes
  behavior (anything with a definition already failed). A failed include is
  not recorded either; what it defined before failing stays in the scope,
  as it always did.
- `load module` still runs in an isolated child scope. Its analyzer now
  sees outer actions as callable functions (so calls resolve) but rejects a
  same-name definition up front — the runtime would reject it anyway, and
  the rejection must land before the module's earlier statements run.

## Evidence

- Red: `tests/include_diamond_test.rs` (9 tests, four added from review
  findings: loop-scope recycling, side-effect-only re-include, the
  import-depth boundary, and `load module` outer-action rejection) and
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
