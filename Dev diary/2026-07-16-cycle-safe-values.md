# Dev Diary — Cycle-safe values (2026-07-16)

## Context

Lists and objects are reference-counted mutable values, so valid WFL can build
self-referential and mutually recursive graphs. Value equality already handled
those graphs, but `Display`, `Debug`, and the deep clone used for module
isolation still traversed them recursively without cycle detection. Displaying
a cyclic value (or including one in a diagnostic) could therefore exhaust the
native stack, and cloning one could do the same during isolated lookup.

## What changed

- `Display` and `Debug` now carry per-format traversal state. A container that
  reappears on the active path renders as `<cycle>`; shared acyclic values still
  render normally each time they appear.
- Formatting stops after 64 nested containers and renders `<max-depth>`, keeping
  very deep acyclic graphs comfortably below the native stack limit.
- Formatting uses `try_borrow`, so an incidental outstanding mutable borrow is
  rendered as a marker instead of causing a `RefCell` panic.
- `Value::deep_clone` now memoizes list, object, and container-instance
  placeholders before cloning their contents. Cycles point into the cloned
  graph, shared references remain shared within that graph, and the clone stays
  isolated from the source.
- Container parent links are cloned through the same memo instead of retaining
  a reference into the source graph.

## Compatibility

Acyclic values below the depth limit retain their existing display and debug
forms. Only values that previously recursed indefinitely, exceeded the new
nesting guard, or were formatted while mutably borrowed receive marker text.

## Tests

- Rust-level self-cycle and list/object mutual-cycle formatting regressions.
- Deep-clone assertions for source isolation, back-reference preservation, and
  shared identity.
- A depth-bound regression for acyclic nesting.
- An interpreter regression that constructs a self-referential list with WFL's
  `push` statement and displays it as `[<cycle>]` without aborting.
