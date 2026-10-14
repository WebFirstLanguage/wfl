# Container constructor arguments

Issue #786 completes the source path that PR #785 prepared. The interpreter
could run an `initialize` action with constructor arguments supplied directly
in the AST, including an inherited action and a child override. The parser
never populated those arguments from a WFL program.

The Maintainer approved `create new K with 3 as k: ... end`. The parser now
fills the existing positional argument field. The property body runs before
`initialize`, and the old argument-free form does not call it. The type
checker walks the container's inheritance chain and uses the first
`initialize` it finds. It withholds a missing-action error when an included
ancestor is unavailable during static analysis. Named arguments and overloads
are outside this change.

The LSP now suggests the complete source form, shows an inherited
`initialize` signature when hovering the container type, and provides
signature help while the `with` clause is unfinished. The layout linter
recognizes the new header, and the fixer retains the required property block.

The executable example is `TestPrograms/containers/constructor_arguments.wfl`.
It asserts that `initialize(3)` can call `this.configure()` and leaves the
property value at 6. Focused Rust tests cover parsing, runtime, inherited and
overridden signatures, diagnostics, and LSP behavior.

Review follow-up (October 14): constructor arguments now use the ordinary
argument boundary, so variables named `headers` or `content_type` and text
concatenation ending in `with headers` parse correctly. Constructor hover is
limited to the type token's source span, and the unused-variable check counts
references inside constructor arguments. Each regression has a test-only red
commit before its fix.
