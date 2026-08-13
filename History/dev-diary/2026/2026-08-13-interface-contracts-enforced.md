# 2026-08-13 — Interfaces stop being decorative

## What changed

The containers doc (`Docs/04-advanced-features/containers-oop.md`) promised
"Interfaces define contracts that containers must fulfill." Auditing every
example in that page against the release binary showed the examples themselves
all ran and printed exactly what the doc claims — but the interface promise was
false. `create interface X` parsed only as a bare declaration: no body, no
required actions, and nothing anywhere in the pipeline ever checked that a
container claiming `implements X` provided anything at all. Even
`implements TotallyUndefinedInterface` executed happily at runtime (only the
type checker warned).

Interfaces are now real contracts:

```wfl
create interface Drawable:
    requires action draw
    requires action get_area: Number
end
```

- **Parser** — interface bodies with `requires action <name>`, optional
  `needs` parameter lists, optional `: ReturnType`, and `extends` between
  interfaces (comma-separated list). The `requires` keyword existed in the
  lexer since the beginning and was never consumed by the parser. Bare
  `create interface Name` still parses as an empty contract, so existing
  programs (marker interfaces) keep working.
- **Interpreter** — when a `create container … implements …` definition is
  evaluated, every required action (accumulated through interface `extends`
  chains) must be present with the same parameter count, either on the
  container itself or inherited through its own `extends` chain. A breach is
  a runtime error naming the container, the interface, and the missing or
  mismatched action; an unknown or non-interface name in `implements` is also
  an error now.
- **Analyzer/Type checker** — the analyzer records an `InterfaceInfo`
  registry, and the type checker performs the same conformance check
  statically so tooling (LSP, MCP, `wfl --analyze` users) sees the breach
  before execution.

## Dead code removed

`src/parser/container_ast.rs` (181 lines of duplicate AST definitions) and
`src/parser/container_parser.rs` (an empty comment stub) were never declared
as modules anywhere — not compiled, not referenced. Both deleted.

## TDD evidence

Red commit `test: red tests for interface contract parsing and enforcement`
adds `tests/interface_contract_test.rs`; six of its tests fail against the
prior implementation (body parsing, extends parsing, missing-action rejection,
unknown-interface rejection, inherited satisfaction, conformant execution) and
all pass after the change. Risk class R3 (backward compatibility): the bare
interface form and the entire `TestPrograms/` suite were re-run against the
release binary, and every code example in the containers doc was executed
before and after.

## Coverage added

- `tests/interface_contract_test.rs` — parser + end-to-end binary tests.
- `TestPrograms/containers/interface_contracts.wfl` — positive coverage:
  bodies, extends accumulation, parameterized requirements, inherited
  satisfaction, marker interfaces.
- `TestPrograms/error_examples/interface_missing_action.wfl` — gated
  expected-failure program.
- `TestPrograms/docs_examples/containers/` — four registered doc examples
  (basic container, interface contract, enforcement error, task manager)
  wired into `validate_docs_examples.py`.
- `TestPrograms/containers_comprehensive.wfl` — its interface section now
  uses a real body, so the flagship container test exercises enforcement.

## Doc honesty

The Interfaces section of the containers doc now shows the enforced syntax,
the error a breach produces, interface inheritance, and marker interfaces.
The keyword references had two stale examples (`define interface called
Runnable:`, `requires method run`) that matched no grammar past or present;
both now show the real forms.
