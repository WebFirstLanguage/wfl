# Dev Diary — 2026-07-26: Typechecker Gradual-Contract Audit

## Scope

This was a systematic audit of WFL's static type system against the parser,
analyzer, standard-library contracts, and interpreter at base commit
`62a1b302200e70797dc399d75ecc778b2ddf6af2`. The motivating symptom was the
amount of `Any` visible in type checking: some uses were legitimate gradual
boundaries, while others were masking information the runtime already knew.

A finite audit cannot prove every possible WFL program correct. The acceptance
standard for this pass was therefore concrete and repeatable:

- every runtime built-in has an arity and value-kind contract;
- every expression and statement operand is traversed;
- concrete information survives collections, branches, loops, actions,
  containers, and runtime-created bindings;
- shared mutable state and captured scalar bindings are widened only where an
  effect can actually reach them;
- `Any`, `Unknown`, `Optional<T>`, and `Error` retain distinct meanings;
- existing valid WFL programs remain compatible;
- checker acceptance and rejection match the corresponding runtime behavior;
- all repository quality gates pass.

## What the audit found

### Built-in contracts were too broad

The runtime inventory had arity information, but many static registrations
used broad `Any` parameter or return types even when the implementation
required or produced a concrete value. Some aliases and optional arities also
diverged between the two layers.

The standard-library type registry now mirrors the installed runtime natives,
including aliases and arity ranges. The call checker applies those contracts
to explicit calls, natural-language action calls, stored references, and
zero-argument auto-calls. Result specialization retains collection element
types for operations such as `slice`, `unique`, `concat`, `pop`, `shift`,
`remove_at`, and `random_from`. `find` now returns `Optional<T>` because its
absence result is WFL `Nothing`.

Dedicated `Date`, `Time`, and `DateTime` types prevent an unrelated user
container with the same name from satisfying a temporal runtime contract.
Historical custom temporal annotations remain compatible where the referenced
container is not statically known.

### Flow analysis selected syntax instead of runtime endpoints

Several branches, loops, and error paths retained the type from whichever
source branch happened to be checked last. Loop bodies were not consistently
rechecked at a stable header, and a `try` handler could miss a mutation made
immediately before the statement that raised the error.

The checker now joins all runtime-reachable endpoints and preserves common
outer structure (`List<Number>` joined with `List<Text>` becomes
`List<Any>`). Persistent and fresh-iteration loops use separate fixed-point
models matching the interpreter's environment behavior. Possibly empty loops
retain their zero-iteration path; a `for each` over a provably non-empty list
retains its guaranteed first-iteration effect. Nested `try` flows incrementally
join reachable intermediate binding and alias states without retaining a full
binding-by-statement snapshot matrix. Capture traversal is charged to the
execution budget. Handlers begin from the structural state at `try` entry, and
`finally` receives a newly created try-local binding only when every reachable
success/error endpoint defines it.

The final `Any` inventory also exposed a runtime mismatch in `repeat while`:
the interpreter tracked the body's last value but discarded it on normal loop
completion, unlike `while` and `repeat until`. It now returns that value, and a
literal-`no` pre-test loop is inferred as `Nothing` instead of an unnecessary
top-level `Any`.

Bindings created in branches escape only when every applicable runtime path
defines them. Deferred actions, methods, tests, event handlers, and WebSocket
handlers restore their outer state after definition-time checking.

### Action results did not model WFL block values

Return inference previously summarized source-level `return` statements after
the body and treated fallthrough as `Nothing`. That lost both the type at the
actual return program point and WFL's runtime rule that a normally completed
block evaluates to its last executed statement.

Each reachable return is now recorded with the type and alias state at that
program point. Inference joins explicit returns with the body's normal
completion value. Annotated actions and methods validate implicit results as
well as explicit returns. Unreachable tails and literal-dead branches do not
widen the result, while a definitely returning `finally` overrides the
primary result as it does at runtime. Overloads keep independent return and
effect summaries.

The action parser recognizes return annotations only in unambiguous
double-colon headers such as `name: List of Text:`. CodeFixer emits that syntax
recursively for list, map/binary, and optional return types. A lexer-merged
`"<name> returns <type>"` remains a legacy multi-word action name.

### Shared lists and captured scalars needed effect tracking

WFL list values use shared `Rc<RefCell<_>>` storage. Name-only inference was
therefore unsound after copying, nesting, inserting, clearing, filling, or
passing a list through a closure.

List aliases are now keyed by stable lexical binding identity and structural
depth. Provenance survives nested list/map construction, extraction,
insertion, aggregate self-assignment, branch/error joins, loop backedges, and
fresh versus shared action returns. Projection returns rebase their full
descendant closure. Replacement and clear operations detach stale descendants
only for proven strong updates, retaining them across may-alias joins. Named
action overloads receive list and captured-scalar effect summaries, including
forward-call dependencies.

An opaque user-code boundary cannot provide such a summary. Stored native or
method references, container construction and methods, events, dynamically
resolved calls, and WebSocket handler execution therefore invalidate affected
refinements and conservatively escape shared lists. Optional scalar guards are
restored to the original optional type rather than being trusted across a call
that may mutate the captured binding.

### Runtime-created values and statement operands were under-typed

The checker now reconstructs concrete types for request objects, response and
outbound streams, file and database handles, patterns, calendar values,
WebSocket server/connection values, container instances, and other implicit
runtime bindings in every relevant scope.

HTTP response, streaming, file/process, database, header, event, and WebSocket
statements now visit and validate all operands. Static and instance container
members have separate contracts; property initialization, constructors,
inheritance, parent calls, events, and stored static method references agree
with runtime lookup. Declared property types are preserved across direct method
assignments and typed-list insertions, including statement and built-in
mutation forms on bare, inherited-instance, and static property accesses.
Method parameters and locals outrank properties through nested control-flow
scopes; properties in turn outrank true outer bindings. Instance and static
property mutations completed before a later method error are persisted instead
of being accidentally rolled back.

Every explicit runtime result/iteration binder now creates the same local
binding the analyzer and checker model, including file, database, HTTP,
process, collection, calendar, pattern, container, and WebSocket statements.
Nested action, container, and interface declarations follow the same rule.
Ordinary `store` and `change` still target a same-named property, while a
constant declaration is rejected instead of diverging at runtime.

Container inheritance cycles are rejected without recursive lookup. An
incompatible concrete inherited-property redeclaration emits a compatibility
warning because the runtime flattens both declarations to one mutable slot.
Static method context is now poll-local for asynchronous handlers: nested
calls and park/resume boundaries persist completed static-property writes
without allowing concurrent handlers to borrow or overwrite each other's
active context stacks.

Static `Nothing` parity now covers both historical no-value runtime variants.
Core predicates and equality treat both variants as no value, while list
search, JSON null, and random seeding preserve their legacy `Nothing` identity
and therefore preserve existing `typeof` output. Static optionality describes
both without forcing a runtime compatibility migration.

### Checker reuse leaked program state

Analyzer and typechecker state now resets between programs. A failed editor or
LSP check cannot carry symbols, diagnostics, aliases, overload results, or a
budget breach into the next run.

## What `Any` means after this pass

`Any` remains intentional in five cases:

1. external or dynamically shaped data such as parsed payloads, unknown
   imports, and heterogeneous database records;
2. incompatible concrete results for which WFL has no general union type
   (`Number | Text`, for example);
3. a heterogeneous position inside retained structure (`List<Any>` or
   `Map<Text, Any>`);
4. shared mutable data crossing an opaque effect boundary;
5. a runtime construct whose completion value is deliberately dynamic.

An empty collection uses `Unknown` because evidence is missing. A
`T | Nothing` result uses `Optional<T>`. A prior diagnostic propagates
`Error`. These states must not be replaced by `Any` merely to suppress a
diagnostic.

Some boundaries remain deliberately conservative. WFL does not yet represent
arbitrary unions, and opaque container/event or stored native/method calls do
not have body-specific effect summaries. Those cases may lose precision,
including top-level `Any` when opaque code can rebind a mutable captured
variable, but they do so at an identified runtime mutation boundary rather
than through accidental fallback.

## Compatibility work

- Existing heterogeneous and dynamically imported programs remain gradual.
- Older temporal custom annotations and colon-style container annotations keep
  their historical interpretation.
- The existing `Nothing`-then-assignment behavior inside a `for each` remains
  valid when the source is provably non-empty, without making possibly empty
  loops unsound.
- Existing write/flush ambiguity, zero-argument action auto-call, overload,
  include, and parser compatibility suites remain green.
- Three scope-isolation fixtures that used literal text as a fake HTTP request
  were updated to create a real typed request. The stricter request/event
  operand checks were retained because those fake operands would fail at
  runtime.

## Compatibility-limited findings

The review also identified four property migrations that cannot be completed
as a local checker patch without changing supported runtime behavior:

- Concrete properties without defaults are absent on instances or `Null` in
  static state, but existing programs may initialize them later or omit an
  inherited property. Required-field rejection needs definite-initialization
  or optional-state modeling plus the governance deprecation path.
- List properties share their runtime storage with values passed into, read
  from, returned from, or projected through them. Direct property mutations
  are now contract-checked, but complete protection requires property-aware
  alias paths and interprocedural summaries. Deep-copying would break WFL's
  existing shared-list semantics.
- Plain action and method annotations are historically static hints, not
  runtime guards. Gradual or `Nothing` arguments can therefore be laundered
  through a typed parameter before reaching a concrete property. A compatible
  repair needs runtime casts/guards or property-effect summaries and a
  migration plan; globally changing call compatibility would silently alter
  established gradual behavior.
- Child containers have historically been able to redeclare inherited
  properties with incompatible annotations. The analyzer now warns, but hard
  invariance would reject supported programs and therefore requires the
  governance deprecation path.

These are recorded explicitly in the type-system design instead of being
hidden behind `Any` or described as solved. The practical rule for current
code is to give concrete properties compatible defaults, keep inherited
annotations compatible, and not expose a mutable typed-property list through
ordinary aliases when its element invariant matters.

## TDD and review

Regressions were added before their corresponding fixes for built-in
inventory and value kinds, expression traversal, collection inference,
optionality, action completion, branch/loop/try joins, alias effects, captured
scalars, static and inherited container members, runtime-created bindings,
checker reuse, runtime `Nothing` parity, static-handler concurrency, cyclic
inheritance, and method-local declaration/binder parity.

The focused type-system matrix covers hundreds of cases across the new and
expanded integration-test binaries. Review findings about static stored
methods, property invariants, scalar effects beyond direct calls, structured
alias/return provenance, definite `try` bindings, nested error reachability,
and snapshot cost were incorporated before the final gates.

## Verification

All commands completed successfully on Windows:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features --jobs 1 -- -D warnings`
- focused type-system integration tests
- `cargo test --all --jobs 1`
- `cargo build --release --jobs 1`
- `scripts/run_integration_tests.ps1 -TestOnly`
  - Rust integration preflight: passed
  - WFL programs: 111 passed, 0 failed, 24 documented skips
- `scripts/run_web_tests.ps1`
  - 2/2 HTTP tests passed
  - TLS fixture skipped because OpenSSL was unavailable
- `python scripts/validate_docs_examples.py --ci --force`
  - 18 passed, 0 failed

The full test run still prints pre-existing unused-code warnings from several
LSP test fixtures. The integration preflight also printed a non-fatal
read-only Cargo cache bookkeeping warning. Neither affected command status or
test results.

## Documentation

`Docs/development/type-system-design.md` is the contributor contract for
gradual states, collections and aliases, built-ins, control-flow joins, action
results, containers, intentional `Any` boundaries, and runtime-parity review.
It is linked from the development index and compiler internals.
