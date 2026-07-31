# Dev Diary — 2026-07-31: The analyzer's blind spot for `add ... to CONST` (#671)

## Context

Issue #671 came out of the repository-hygiene migration (#672), which converted
the old `syntax_test/test_constant_immutability.wfl` into the asserted
diagnostics fixture `tests/fixtures/diagnostics/constant_immutability.wfl`. The
program mutates one constant five ways:

```wfl
store new constant MAX_SIZE as 100
change MAX_SIZE to 200
add 10 to MAX_SIZE
subtract 5 from MAX_SIZE
multiply MAX_SIZE by 2
divide MAX_SIZE by 2
```

Running it produced **four** `Cannot modify constant` reports, not five. Deleting
each line in turn identified the missing one: the `add` line contributed no
report at all — remove it and you still got four; remove any other line and you
got three.

## Root cause

The four reported forms and the silent one take different routes through the
parser.

`change`, `subtract`, `multiply`, and `divide` all end up as
`Statement::Assignment` — `subtract 5 from x` is desugared by
`parse_arithmetic_operation` into `x = x - 5` (`src/parser/stmt/variables.rs`).
The analyzer's `Assignment` arm resolves the target symbol and, on
`SymbolKind::Variable { mutable: false }`, emits the constant error
(`src/analyzer/mod.rs`).

`add`, uniquely, does **not** go through that path. Because `add 10 to x` is
ambiguous at parse time — `x` may be a number (arithmetic) or a list (append),
and the parser has no type information — `parse_add_operation`
(`src/parser/stmt/collections.rs`) always emits
`Statement::AddToListStatement`, leaving the interpreter to pick the meaning at
runtime. The analyzer's `AddToListStatement` arm only ever checked that the
target name was *defined*. It never checked whether it was *writable*.

So the constant check was never "dropped" by some interaction between the five
statements — it was never performed for `add` in the first place. The reason the
issue looked like a combination-only bug is that `add 10 to MAX_SIZE` **alone**
does still fail: analysis passes, the program runs, and the interpreter's
`Environment::assign` rejects the write with the same message. Put it next to
other mutations and analysis fails first, so the run never happens and the
report never appears.

`remove ... from` and `clear` share the same shape (`list_name: String`, no
constness check) and therefore the same hole.

## What changed

`src/analyzer/mod.rs` gains one helper, `report_constant_mutation`, applied to
the three bare-name mutation statements — `AddToListStatement`,
`RemoveFromListStatement`, and `ClearListStatement`. It resolves the target and
emits the same message the assignment path uses:

```
Cannot modify constant 'MAX_SIZE' - constants are immutable once defined
```

It is deliberately mutually exclusive with the existing
`Variable '<name>' is not defined` report, so an undefined name still produces
exactly one diagnostic.

### `mutable: false` does not mean "constant"

The first version of the helper tested `SymbolKind::Variable { mutable: false }`
— the same predicate the assignment path uses — and immediately broke a shipped
program, `TestPrograms/test_create_list_expression.wfl`:

```wfl
define action called process_list with parameters list_param:
    add "processed" to list_param
    give back list_param
end action
```

`mutable: false` turns out to be badly overloaded. It is set for action
parameters, container-method parameters (on a *separate* code path from action
parameters, which the second attempt then also tripped over), loop variables,
`try`/`when` error bindings, the predefined globals (`newline`, `wfl_version`,
…), and — in `Analyzer::with_parent_variables` — every REPL parent-scope
variable regardless of its real mutability. It means "the analyzer will not let
you rebind this symbol", not "the author wrote `constant`". (That overload is
also why `change p to 5` on a parameter already reports `Cannot modify constant
'p'` — a pre-existing message wart, not something this change introduced.)

So the analyzer now tracks constants explicitly: a `constant_bindings` set of
`SymbolBindingKey`s, populated in the `VariableDeclaration` arm when
`is_constant` is set, and consulted by `report_constant_mutation`. Keying by
binding rather than by name means an inner-scope shadow of a constant name is
not mistaken for the constant itself. Nothing that is merely immutable ends up
in the set. `list_parameters_and_loop_variables_are_not_constants` and
`container_members_are_not_constants` pin both parameter paths.

One consequence worth naming: because only `store new constant` populates the
set, `add 10 to newline` on a predefined global still goes unreported. Extending
the set to cover the predefined globals is easy, but it would also have to
exclude the `with_parent_variables` REPL bindings that are marked immutable
without being constants, so it is left out of this issue.

The repro now reports all five mutations, and the constant-list forms are caught
too:

```wfl
store new constant ALLOWED_ROLES as ["admin" and "editor"]
add "guest" to ALLOWED_ROLES   // now: Cannot modify constant 'ALLOWED_ROLES'
clear ALLOWED_ROLES            // now: Cannot modify constant 'ALLOWED_ROLES'
```

Those two were the more interesting half of the fix. A constant *number* was
already protected at runtime, because the interpreter routes the arithmetic case
through `Environment::assign`. A constant *list* was not: the interpreter pushes
straight into the `Rc<RefCell<Vec<Value>>>`, so the binding is never reassigned
and the constant check is bypassed entirely. `add`/`remove`/`clear` on a
constant list silently succeeded. Catching it during analysis closes that hole
before the program runs.

## Compatibility

Mutating a constant has always been an error the language intends to reject —
the new reports make the analyzer agree with the documented semantics and with
what the runtime already did for numbers. No program in `TestPrograms/` or
`examples/` mutates a constant; the only sources that did were the diagnostics
fixtures written to demonstrate the error.

The one genuine behavior change is the constant-list case described above, which
moves from "silently allowed" to "rejected at analysis time". That is the
documented meaning of `constant`, and no shipped example relied on it.

## Testing

`tests/constant_mutation_analyzer_test.rs`, added as a **test-only Red commit**
before the fix:

| Test | Red | Green |
|---|---|---|
| `add_to_constant_is_rejected_on_its_own` | 0 reports, expected 1 | pass |
| `every_mutation_form_of_a_constant_is_reported` | 4 reports, expected 5 | pass |
| `list_mutation_statements_reject_constant_targets` | 0 reports, expected 3 | pass |
| `mutable_targets_are_still_accepted` | pass | pass |
| `list_parameters_and_loop_variables_are_not_constants` | pass | pass |
| `container_members_are_not_constants` | pass | pass |

The last three tests passed in both states on purpose: they are the negative
assertions that the new check does not over-report — ordinary mutable variables
and lists, and the immutable-but-not-constant symbols (parameters, loop
variables, container properties), must stay untouched. The two parameter tests
were added in response to the over-reporting bug described above, which the
full-suite and `TestPrograms/` runs caught.

`tests/diagnostics_fixtures_test.rs::constant_mutation_is_rejected_for_every_mutation_form`
pinned `>= 4` reports with a comment pointing at this issue; it is now an exact
`== 5`.

Also run: `cargo test --all --no-fail-fast` (151 test binaries, all green),
`cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all --
--check`, the CI program sweep over `TestPrograms/` against the release binary
(142 passed, 0 failed, 49 skipped), `python scripts/validate_docs_examples.py`
(21/21), and `python3 -m unittest discover -s tests/tooling` (33 passed).

Risk class: **R3**. The first draft said R1 on the grounds that this only adds a
front-end diagnostic — but the same document records a real behavior change
(`add`/`remove`/`clear` on a constant list moves from silently allowed to
rejected at analysis time), and `testing.md` §5 puts **backward compatibility**
in R3 outright, with the class never to be lowered to dodge a gate. R3 is the
correct classification and R1 was wrong.

The R3 evidence is the backward-compatibility trigger rather than §11.3: this
change involves no concurrency, cancellation, lifecycle, streaming, untrusted
input, or crypto, so those risk-triggered tests do not apply. What does apply is
failure-path and negative coverage, which is present:

- Negative assertions that the new check does not over-report —
  `mutable_targets_are_still_accepted`,
  `list_parameters_and_loop_variables_are_not_constants`,
  `container_members_are_not_constants`.
- A compatibility sweep of every shipped program (all non-skipped
  `TestPrograms/`, plus the docs examples) against the release binary, which is
  what caught the action-parameter over-report before it could ship.
- `outer_constants_survive_an_intervening_try`, pinning that unrelated scope
  handling does not silently drop a constant marker.

## Documentation

`Docs/03-language-basics/variables-and-types.md` claimed "Currently, all
variables can be changed. True constants (immutable values) are planned for
future versions." That has been false since `store new constant` shipped. The
section now documents the real syntax, shows every rejected mutation form, and
covers constant lists. `Docs/06-best-practices/naming-conventions.md` had the
same stale "true immutability is limited today" framing and now points at the
real declaration form — with a note that `wfl --lint` still flags
SCREAMING_SNAKE_CASE names as non-snake_case, which is advisory and does not
fail a build.

Two validated examples were added and registered in the docs-examples manifest:
`constants_01.wfl` (executable, exit 0) and `constants_immutable_01.wfl`
(error example, expected to fail semantic analysis with `Cannot modify
constant`).

## What review caught

Three findings from PR review were real and are fixed here.

**The docs overclaimed.** The first draft said WFL "refuses to modify" a
constant and that "every mutation form is rejected before the program runs" —
false while `push` remains unchecked, exactly the kind of thing CLAUDE.md's
"Docs Must Be Honest" rule exists to stop. Both pages now enumerate the forms
that *are* checked and carry an explicit known-gap callout pointing at #673,
plus a note that constants fix the binding and not the contents reached through
an alias.

**The constant marker was lost across a branch merge.** A constant declared in
both arms of a `check` is re-defined into the parent scope by the `IfStatement`
and `SingleLineIf` arms, under a new binding key — so `constant_bindings` lost
track of it. The symbol keeps `mutable: false`, so `change` still reported while
`add`/`remove`/`clear` went silent: #671 all over again, one scope up. The
merge now carries the marker with the binding, mirroring the existing
`mutable: then && else` rule (constant on either arm ⇒ constant after). Verified
before the fix: `add 1 to LIMIT` after such a branch escaped analysis entirely
and was only caught at runtime — and for a constant *list* it would not have
been caught at all. `pop_scope_promoting_except` migrates the marker too,
alongside the alias state it already moved.

**A negative test did not test what it claimed.** The loop-variable case used
`add entry to gathered`, where the loop variable is the *value* and `gathered`
is the target — it proved nothing about loop bindings. Rewritten so the loop
variable is the mutation target. That immediately surfaced something worth
recording: `subtract 1 from entry` and `multiply entry by 2` on a loop variable
*do* report "Cannot modify constant", because they desugar to `Assignment`,
whose long-standing `mutable: false` check treats loop variables and parameters
as constants. That is pre-existing and out of scope here — the same message wart
noted above — so the test now covers only the bare-name statements this change
touches.

## Follow-ups not taken

`push with <list> and <value>` takes an arbitrary *expression* for its target
rather than a bare name, so it does not share the statement shape fixed here and
still mutates a constant list without complaint. Closing that needs the analyzer
to resolve write-targets through expressions, which is a larger change than this
issue calls for. Filed as **#673**, and now called out in the constants
documentation so no reader is told a guarantee the language does not provide.

The `Assignment` path reporting "Cannot modify constant 'p'" for action
parameters and loop variables — which are immutable but not constants — is a
misleading message that predates this change. Worth a separate issue; not
touched here.
