# 2026-07-20 — User-Defined Action Overloading

## What shipped

Actions can now be defined more than once with the same name in the same
scope, dispatching on argument count and declared parameter types. This
resolves the long-standing analyzer TODO ("Implement proper overload
resolution based on argument types and count") — and goes further than the
TODO asked, per maintainer direction: instead of only resolving among the
stdlib's multi-arity builtin signatures, users can now write overloads.

```wfl
define action called depict with parameters value as number:
    return "a number: " with value
end action

define action called depict with parameters value as text:
    return "some text: " with value
end action
```

## Design decisions

**Definition-time strictness.** Two same-name definitions are rejected when
they are exact duplicates (same arity, same normalized types) or when no
parameter position has *both* versions declaring concrete, different types.
`f(x)` + `f(x as number)` is rejected — an untyped parameter accepts numbers
too, so no call could be routed deterministically. This keeps dispatch
predictable and is enforced identically in the analyzer (PASS 1) and the
runtime (`Environment::define_or_merge_action`), so include-driven or
dynamically-constructed programs get the same rule.

**Deferred static resolution.** At a call site the analyzer filters
signatures by arity, then by static argument types. Zero survivors is an
Elm-style error listing candidates; exactly one gets the full named-argument
and per-argument type validation (the `of` form now gets the same call
validation the `call` form always had); several survivors — possible only
when an argument's static type is Unknown — defer silently to runtime
dispatch. WFL is dynamically valued, so a hard "ambiguous call" error would
have broken ordinary dynamic code.

**Runtime dispatch.** `Value::Overloaded(Rc<OverloadedFunction>)` wraps the
overload set in definition order. Dispatch filters by arity, drops candidates
whose concrete parameter types reject an argument value, then picks the
candidate with the most concretely-matched parameters (ties → definition
order; ties are only reachable through container-inheritance overlap thanks
to the definition-time rule). `FunctionValue` gained `param_types` — the
runtime previously erased parameter type annotations entirely.

**Typechecker side-table.** `symbol_type` can only hold one
`Type::Function`, so per-overload inferred return types are recorded in
`overload_returns: HashMap<(name, signature index), Type>` and resolved at
call sites; deferred calls take the common return type of the surviving
candidates, else Unknown.

**Zero-arg auto-call preserved.** A bare reference to an overloaded name
auto-calls its zero-parameter overload if one exists (matching single-action
behavior); otherwise the reference is the overload set as a first-class
value, callable through variables.

**Container methods deferred.** Methods live in four name-keyed HashMaps with
inheritance and interface-conformance rules; the current (silent last-wins)
behavior is long-standing. Overloading them is follow-up work tracked in
issue #638; the docs say so explicitly and the storage/registration sites
carry `TODO(#638)` markers.

## Incidental fixes and discoveries

- **Typed `text` parameters never parsed — now fixed.** `text` lexes as
  `KeywordText` (and `nothing` as `NothingLiteral`), but the action parser
  only accepted `Identifier` in type position — so `as text` / `as nothing`
  parameters were unusable. Fixed via `type_from_token`, which accepts the
  keyword/literal token forms (`text`, `pattern`, `nothing`) alongside
  identifiers. `returns <type>` remains unsupported: the lexer folds
  `name returns text` into one multi-word identifier before the parser ever
  sees a `returns` clause, so the typechecker's "WFL has no return-type
  annotation syntax" comment stays accurate and the overload machinery leans
  entirely on inferred return types (issue #575 infrastructure).
- **Pre-existing debug-build stack exhaustion.** Recursion inside an
  `otherwise:` block overflows the 2 MiB test-thread stack after ~2-3 frames
  in debug builds (confirmed on unmodified main; depth 1 passes, depth 3
  aborts). Release builds are fine. The new recursion test runs on a
  64 MiB thread; worth a separate issue.

## Semantics notes

- Overloading is same-scope only; shadowing an outer-scope action (or
  colliding with a variable) keeps its existing errors.
- Merging rebinds the name to a new immutable overload set; a closure that
  captured the name *before* a later overload was defined keeps the value it
  saw (snapshot semantics). Top-level programs defining all overloads before
  first call — the normal shape — never observe this.
- Runtime dispatch order requirement is unchanged from single actions: a
  call executes against the overloads defined so far.

## Tests

`tests/overload_analyzer_test.rs` (11), `tests/overload_typechecker_test.rs`
(5), `tests/overload_interpreter_test.rs` (13), plus
`TestPrograms/action_overloading_comprehensive.wfl` end-to-end. One existing
unit test updated (`test_function_call_type_checking`) because the analyzer
now reports of-form argument type mismatches earlier with a more specific
message.
