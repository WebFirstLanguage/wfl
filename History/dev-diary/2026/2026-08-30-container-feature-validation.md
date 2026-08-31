# 2026-08-30 — Documented container and interface features, validated

## What this is

A pass over every user-facing container/interface claim — the containers
guide, the language spec's `implements` list, keyword examples, and the
key-features container bullet list — against the release binary.

## What already worked

The shipped `create container` / `create new` / `object.action()` /
`object.property` surface ran as documented: typed properties, in-action
mutation, parameterized and returning actions, `extends` with overrides,
multi-level inheritance, `requires action` contracts (including parameters
and interface `extends`), marker interfaces, inherited methods satisfying a
contract, static members, property `defaults`, containers as action
parameter types, and a container implementing more than one interface.

Gated programs that already covered parts of this (`containers_comprehensive.wfl`,
`containers/interface_contracts.wfl`, the four original docs examples, the
interface Rust suite) still pass.

## What did not match the docs

**Interface return types were static-only.**
`Docs/04-advanced-features/containers-oop.md` says a required return type is
checked before the program runs. The type checker already reported
`requires action get_area: Number` vs `action get_area: Text`, but the CLI
treats type diagnostics as warnings and the runtime never compared return
types. A container that failed the contract still defined and continued.

Runtime conformance now stores each method and each `requires action` return
type and rejects a concrete mismatch when the container definition runs —
the same stop as a missing action or a wrong arity.

**Keyword examples used a grammar that does not parse.**
`define container called Person:` / `store pet as new Animal` was still the
example in the reserved-keyword table and in
`TestPrograms/docs_examples/keyword_reference/containers_examples.wfl`
(CI-SKIP'd). Those now show `create container` / `create new`.

**Event handlers were listed as a container feature.**
`key-features.md` claimed "Events and event handlers." Declaring `event` and
`trigger` inside a container works; `on <instance> <event>:` does not parse
a handler body. The bullet now says handlers are not a shipped form.

## Coverage added

- `TestPrograms/containers/documented_features.wfl` — 14 `describe`/`expect`
  cases covering the containers-guide surface plus multiple `implements`,
  defaults, and static members.
- `TestPrograms/error_examples/interface_return_type.wfl` and
  `interface_static_action.wfl` — gated expected-failure programs.
- `tests/interface_contract_test.rs` —
  `container_with_incompatible_return_type_fails_at_runtime`.
- Docs examples for inheritance, override, interface `extends`, marker
  interfaces, and property access, registered in the docs-examples manifest.

## TDD evidence

Red: `wfl TestPrograms/error_examples/interface_return_type.wfl` exited 0
and printed `unreachable: a return-type mismatch must fail` against the
pre-change release binary; the new Rust test encodes that same program and
requires a nonzero exit naming `get_area` and `return`.

Green: after the runtime check, that program exits 1 with
`action 'get_area' returns Text but the interface requires Number`, and the
14 asserted documented-feature tests pass.

Risk class **R3** (backward compatibility / public contract): a program that
claimed `implements` with a concrete return-type mismatch used to run; it
now stops at the container definition, matching the already-documented
static rule.
