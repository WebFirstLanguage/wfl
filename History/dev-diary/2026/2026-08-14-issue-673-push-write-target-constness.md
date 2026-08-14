# Dev Diary — 2026-08-14: Resolving `push` write targets to their root binding (#673)

## Context

#671 closed the analyzer's blind spot for the bare-name mutation statements
(`add ... to`, `remove ... from`, `clear`). Its own diary entry named the piece
it deliberately left open, and filed it as #673:

```wfl
store new constant ROLES as ["admin"]
push with ROLES and "guest"
display ROLES                    // [admin, guest], exit 0
```

No analyzer report, no runtime error. Nested targets escaped the same way:

```wfl
store new constant CONFIG as [["a"]]
push with CONFIG[0] and "b"
display CONFIG                   // [[a, b]], exit 0
```

## Root cause

`add`/`remove`/`clear` each carry their target as a `list_name: String`, which
`report_constant_mutation` resolves directly. `push` does not:
`Statement::PushStatement` carries `list: Expression` (`src/parser/ast.rs`),
because `parse_push_statement` parses a whole primary expression for the target.
The analyzer's arm therefore had nothing to look up and merely walked both
sub-expressions.

There is no runtime backstop either. The interpreter mutates in place through
the list's `Rc<RefCell<Vec<Value>>>` and never reassigns the binding, so
`Environment::assign` — the only place constness is enforced at run time — is
never reached. Exactly the constant-*list* hole #671 described, arriving through
a statement shape its fix could not see.

## What changed

One analyzer-only change (`src/analyzer/mod.rs`). A new pure helper,
`write_target_root_binding`, resolves a write-target expression to the binding
the write ultimately reaches:

- `Expression::Variable` → itself.
- `IndexAccess` / `MemberAccess` / `PropertyAccess` → recurse into the
  collection or object, so `CONFIG[0]`, `CONFIG[0][1]`, and `CONFIG.entries`
  all resolve to `CONFIG`. Mutating an element mutates the collection the
  binding names.
- Anything else (a call result, a literal) → `None`. There is no binding whose
  constness could be violated, so nothing is reported.

The `PushStatement` arm feeds that root name to the existing
`report_constant_mutation`, so `push` now emits exactly the message
`add`/`remove`/`clear` already emit:

```
Cannot modify constant 'ROLES' - constants are immutable once defined
```

The constness test itself is unchanged — it is still `constant_bindings`
membership, not `mutable: false`, so the immutable-but-not-constant symbols
(action and container-method parameters, loop variables, REPL parent-scope
bindings) stay untouched. The missing piece was only the root-binding walk.

Undefined names still produce exactly one diagnostic: `analyze_expression(list)`
reports `Variable '<name>' is not defined`, and an unresolvable name has no
binding key, so `report_constant_mutation` stays silent.

Nothing in `src/interpreter/`, `src/parser/`, or `src/typechecker/` was touched.

## Explicitly out of scope

The aliasing hole the issue also documents:

```wfl
store new constant ROLES as ["admin"]
store alias as ROLES
add "guest" to alias     // still mutates ROLES, exit 0
```

`alias` is a legitimately mutable binding, so no write-target analysis can
reject this. What `constant` means for a reference value is a language-design
question, and the issue says plainly it must not be folded into this fix. The
constants documentation continues to state the limitation outright.

## Compatibility

Risk class **R2** — analyzer diagnostics gate whether a program runs at all
(`wfl <file>` exits 3 on an analysis error), so this is public CLI/contract
behavior, and pushing onto a constant list moves from "silently allowed" to
"rejected at analysis time".

That is the documented meaning of `constant`, and nothing shipped relies on the
old behavior: sweeping every gated `TestPrograms/` and `examples/` program
through `--analyze` with the release binary produced **zero** new
`Cannot modify constant` reports. The only source that pushes onto a constant is
the docs error-example written to demonstrate the diagnostic.

## Testing

Added to `tests/constant_mutation_analyzer_test.rs` as a **test-only Red
commit** before the fix:

| Test | Red | Green |
|---|---|---|
| `push_onto_a_constant_list_is_rejected` | 0 reports, expected 1 | pass |
| `push_onto_an_indexed_constant_target_names_the_root_binding` | 0 reports, expected 1 | pass |
| `push_targets_without_a_root_binding_are_accepted` | pass | pass |
| `push_onto_a_mutable_list_is_accepted` | pass | pass |
| `push_onto_a_parameter_or_loop_variable_is_accepted` | pass | pass |

The last three pass in both states on purpose: they are the negative assertions
that the new walk does not over-report, and they guard the same shapes #671's
over-reporting bug hit. The mutable case covers both a bare and an indexed
target; the no-root-binding case covers both a call result and a literal.

Also run: `cargo test --all --no-fail-fast`, `cargo clippy --all-targets
--all-features -- -D warnings`, `cargo fmt --all`,
`python scripts/validate_docs_examples.py` (25/25), and the `--analyze` sweep
described above.

## Documentation

`Docs/03-language-basics/variables-and-types.md` carried a **Known gap**
callout saying `push` was not checked and telling readers to "use `add ... to`
when you want the constant to be enforced". That claim is now false, so it is
gone; the constant-list section lists `push` alongside `add`/`remove`/`clear`
and adds the indexed-target case. The alias caveat below it stays — it is still
true. `Docs/06-best-practices/naming-conventions.md` had the matching "the one
form that is not yet checked" pointer and now names `push` as rejected.

`TestPrograms/docs_examples/basic_syntax/constants_immutable_01.wfl` carried a
comment explaining that `push` was "deliberately absent"; it now exercises both
the bare and indexed push forms (10 reports, up from 8), with its manifest
`doc_purpose` updated to match.
