# Container constructor arguments

**Status:** Design approved for issue [#786](https://github.com/WebFirstLanguage/wfl/issues/786#issuecomment-6109888901), 2026-10-11. Implementation pending.
**Risk:** I3 language syntax; R3 compatibility and user-input handling.

## Purpose

Let a program pass values to a container's `initialize` action while creating
the instance. Today the parser always leaves `ContainerInstantiation.arguments`
empty. PR #785 already made the interpreter's AST-level constructor call bind
the receiver and resolve inherited `initialize`; this design exposes that path
in source code.

## Language contract

```wfl
create new Counter with 3 as counter:
    value is 0
end
```

The grammar adds optional `with <expression> [and <expression> ...]` between the
container type and `as <instance>`. Arguments are positional. The existing
property body and `end` remain required. The body sets properties first;
`initialize` then runs on the new instance. The nearest definition of
`initialize` in the `extends` chain wins, including a subclass override.

`initialize` is a conventional action name, not a reserved keyword. The old
`create new Counter as counter: ... end` form keeps its behavior and does not
call a zero-argument `initialize` implicitly. A program can still call
`counter.initialize(3)` explicitly after creation.

Named constructor arguments, constructor overloads, optional initializer
bodies, and alternate spellings are outside this change. Container methods are
currently stored as one action per name, so overloads would require a separate
language design.

## Compiler and editor behavior

- The parser fills the existing `arguments` AST field and stops the argument
  list at `as`. It rejects named arguments and reports the accepted form for
  misplaced `with` or parenthesized constructor syntax. The fixer and linter
  preserve the new clause and inspect its expressions.
- The analyzer already checks argument expressions for defined names. The type
  checker resolves `initialize` through inheritance, validates argument count
  and types, and names the container and action in diagnostics. It reports a
  missing `initialize` only when the hierarchy is known; runtime remains the
  fallback when an included definition cannot be resolved statically.
- The interpreter keeps the receiver-aware dispatch merged in PR #785. No new
  constructor execution path is introduced.
- The LSP exposes the new form in completion and hover, and provides parameter
  signature help inside its `with` clause. Its diagnostics come from the
  compiler checks rather than a separate rule set.

## Verification and release material

Use red-to-green tests for the source form: parser AST, a real `wfl` program
where `initialize(3)` calls `this.configure()` and prints `value=6`, inherited
and overridden actions, missing action, wrong count/type, named-argument and
wrong-form errors, and no-argument compatibility. Check fixer/linter round
trips and LSP completion, hover, signature help, and diagnostics. Keep the
AST-level runtime tests from PR #785.

Update the container guide, both keyword references, JavaScript migration
guide, CHANGELOG, a gated `TestPrograms/containers/` example, and the Dev
Diary. Validate executable examples and run the repository's R3 gates on the
final code and PR head.
