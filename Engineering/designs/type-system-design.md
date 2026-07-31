# Type System Design

WFL uses a gradual static type system. The checker rejects operations that are
provably invalid, preserves concrete type information whenever the syntax and
runtime contracts provide it, and defers genuinely dynamic cases to runtime.

This page documents the contract contributors must preserve when changing the
parser, analyzer, type checker, standard library, or interpreter.

## The four inference states

`Type` contains ordinary concrete types plus three states that have distinct
jobs:

| State | Meaning | Checker behavior |
|---|---|---|
| A concrete type | The value is known to be a number, text, boolean, list element type, container instance, temporal value, and so on | Enforce the operation's declared contract |
| `Unknown` | Inference does not have enough evidence yet | Allow compatible evidence to refine it; otherwise defer rather than inventing a concrete type |
| `Any` | The value is intentionally dynamic or is a known union that WFL does not represent more precisely | Traverse and validate the surrounding expression, but defer checks that depend on the runtime member |
| `Error` | An earlier check already reported an error | Propagate it without producing misleading cascaded diagnostics |

`Unknown` and `Any` are not synonyms. An empty list starts with an unknown
element type because its declaration provides no evidence; that uncertainty is
kept conservative rather than narrowed from a later statement. Parsed JSON, a
heterogeneous database row, or another dynamic boundary uses `Any` because its
runtime shape is intentionally not statically fixed.

`Optional<T>` is a separate inferred composite type meaning that execution
produces either `T` or `Nothing`. It is used for operations such as `find` and
for control flow whose runtime-reachable results include both cases. Unlike
`Any`, `Optional<Text>` does not satisfy an operation that requires a definite
`Text`. Optionality is preserved through collection, control-flow, return, and
overload joins instead of collapsing back to `Any` or `Unknown`. A direct
`isnothing` or `is not nothing` check on a variable narrows it inside the
corresponding branch.

Adding `Any` to silence a checker error is not an acceptable repair. A new
`Any` must correspond to a real runtime union or dynamic boundary, and its
children must still be traversed so errors inside the expression are not lost.

## Collection inference

List literals join the types of all their elements:

- a homogeneous literal retains its concrete element type;
- an empty literal is `List<Unknown>`;
- a heterogeneous literal is `List<Any>`;
- an element that is already `Error` keeps the expression on the error path.

List-returning built-ins preserve or specialize the receiver's element type
where their runtime behavior permits it. Element-returning operations such as
`pop`, `shift`, `remove_at`, and `random_from` return the known element type.
They raise a runtime error rather than returning `Nothing` when no element can
be selected. `find` returns `Optional<T>` because absence does produce
`Nothing`. Shape-preserving operations such as `slice` and `unique` keep the
list type. `concat` joins both element types. Appending mutations widen the
stored list binding only when the inserted value requires it; `fill` replaces
the element type because it overwrites every existing element.

Maps retain key and value types when both can be inferred. Runtime records
whose fields deliberately have unrelated types, such as database rows, use a
text-keyed map with `Any` values.

Runtime lists use shared reference-counted storage, so copying a list variable
creates an alias rather than a deep copy. The checker tracks aliases by stable
lexical binding identity. Mutating either name propagates the exact append or
replacement effect across the group, including aliases promoted from
conditionals and error handlers. Direct calls to named actions use
overload-specific summaries of captured and argument list effects. Calls
without a statically identifiable body, including stored native or method
references, container constructors, methods, events, and dynamically resolved
code, are opaque effect boundaries. At those boundaries, reachable lists are
conservatively escaped. A mutable binding that opaque code can rebind may
become top-level `Any`, while an immutable binding that can only expose shared
list storage retains its outer shape as `List<Any>`. Extracting a list through
an indexed or opaque property path conservatively escapes that source path.

Aliases are tracked structurally through nested lists and maps, list insertion,
aggregate reassignment, and action returns. Return summaries distinguish fresh
local storage from captured or parameter-derived storage and rebase the full
descendant shape when a projection is returned. Operations that replace or
clear aggregate storage detach stale descendant paths only for a proven strong
update; may-alias descendants remain conservative.

Structural tracking is bounded to a fixed nesting depth
(`MAX_LIST_ALIAS_INDEX_DEPTH`, currently 8); paths deeper than that are not
tracked. The bound is what makes the alias relation's key space finite, and so
what guarantees it reaches a fixpoint. Without it, an aggregate that
transitively contains itself — a list holding a map whose value is a list
holding the original — translates its own paths upward forever and the checker
never terminates (issue #654). Real structure does not approach the bound:
across the whole `TestPrograms/` corpus the deepest path any acyclic program
reaches is 2. A gradually typed program cannot supply the bound from its static
type, because `Any` admits an alias path at every depth.

The bound limits what the *relation* records, not which mutations propagate.
Nesting deeper than the bound is still finite structure, so a mutation through
an over-deep alias is applied at the deepest tracked ancestor as an escape,
widening it to `Any`, rather than being discarded. Discarding it would leave a
genuinely aliased aggregate holding a stale, narrower element type, and a later
read through the original path would be rejected on a type the program legally
widened. Escaping only loses precision at those depths; it never reports an
error the unbounded relation would not have reported.

## Built-in function contracts

The runtime built-in inventory and its accepted arities live in
`src/builtins.rs`. Static parameter and return contracts live in
`src/stdlib/typechecker.rs`, and `src/typechecker/mod.rs` applies those
contracts to every supported call form.

The inventory is checked as a unit. An implemented runtime native must have a
static contract, aliases must resolve to the same contract, and reserved but
unimplemented names must not masquerade as callable functions. Generic
contracts may use `Any` for a parameter position, but result specialization
should recover the caller's concrete type when the runtime operation preserves
it.

When adding or changing a built-in:

1. update the runtime inventory and implementation;
2. update the static contract and aliases;
3. add positive and negative contract tests;
4. test concrete return-type propagation into a downstream operation.

An arity-only test is not enough: the checker and interpreter must agree on
accepted value kinds and the result type.

## Control-flow joins and definite bindings

The checker snapshots bindings at control-flow boundaries and joins every
runtime-reachable endpoint.

- A name becomes available after a conditional only when the runtime
  guarantees that an applicable path defines it.
- Reassignments join their value types instead of blindly taking the last
  syntactic branch. Joins preserve common outer structure, so two different
  list element types become `List<Any>` rather than top-level `Any`.
- Loop headers are checked to a conservative fixed point so a later iteration
  cannot invalidate an operation that was accepted using only first-iteration
  types. Diagnostics from the first header remain valid even if a later header
  widens.
- A `for each` over a list that is provably non-empty applies its first
  iteration before joining later backedges. Possibly empty collections retain
  the zero-iteration path. Cardinality facts are deliberately discarded after
  opaque calls and mutations that can remove elements.
- Pre-test loop conditions are checked at both the first and stable headers.
  A statically false condition still validates its body as source code, but the
  unreachable body's type and alias effects do not reach the continuation.
- `while`, `repeat until`, and `repeat while` retain their runtime environment
  across iterations. `for each`, `count`, `forever`, and `main loop` create or
  clear an iteration child: their local declarations reset while mutations of
  parent bindings still flow around the backedge.
- `while`, `repeat until`, and `repeat while` complete with the last executed
  body value. A pre-test loop whose condition is the literal `no` completes
  with `Nothing`; other loop completions remain gradual until WFL can represent
  the body value plus the zero-iteration case without losing structure.
- A `try` handler starts from the structural state at entry plus the streamed
  join of mutations completed before each possible error transfer. A new
  try-local name is available in `finally` only when every success, handled,
  otherwise, and unmatched endpoint defines it. A same-named binding that
  already existed at entry keeps its clause-local shadowing semantics.
  Temporary error aliases remain clause-local.
- Capturing error-transfer state is charged proportionally to the execution
  budget and joined incrementally; the checker does not retain a full
  binding-by-statement snapshot matrix.
- Deferred handlers, tests, and container methods use child scopes so their
  declarations and refinements do not leak merely because the body was
  checked.

These rules model runtime visibility. They must be updated alongside any
interpreter change that creates, promotes, or isolates an environment.

## Actions, overloads, and containers

Action parameters and declared returns are checked in their lexical scope.
Unannotated returns are inferred from every runtime-reachable `return` and the
normal completion value of the action body. WFL blocks evaluate to the last
executed statement's value, so an action ending in a value expression can
infer that type without an explicit `return`. Explicit returns and normal
completion join at their exact program points rather than being selected by
source order. A body whose reachable results are `T` and `Nothing` infers
`Optional<T>`; a body whose results are incompatible concrete types that WFL
cannot represent as a union infers `Any`.

Named actions also record overload-specific effects on captured scalar
bindings. A call invalidates refinements that those assignments can change.
An opaque call cannot use such a summary, so it restores an optional variable
to its pre-guard type instead of trusting a stale narrowing. Guards using
`isnothing`, `is_nothing`, or direct equality/inequality with `nothing` refine
a direct variable inside the guarded branch.
Nested action definitions become visible when execution reaches their
definition, while top-level signatures remain available for forward
references. The canonical source spelling for a declared return is the framed
double-colon header `name: Type:`; recursive list, map, binary, and optional
types use the same form. Text such as `calculate returns schedule` remains a
valid legacy multi-word action name rather than being reinterpreted as an
annotation.

Overload selection uses arity, compatibility, and specificity. Static
selection must mirror runtime selection. In particular, a dedicated temporal
type is more specific than a compatible historical custom annotation.

Instance and static container members have separate contracts. Static defaults
are checked at definition time; static methods can access static properties but
not bare instance properties. Instance methods receive the inverse context.
Inherited members follow the same rule at typecheck time and runtime.
Initializers, direct assignments, and direct typed-list insertions preserve a
declared property contract, including when the source is gradual. The same
element check applies to statement and built-in mutation forms and to direct
instance, inherited, and static property accesses. A fresh empty list literal
is safe for any declared list element type; a shared `List<Unknown>` is not
equivalent evidence. Property names shadow same-named outer lexical bindings
inside methods, while parameters and method locals retain their normal
inner-scope precedence through nested control-flow scopes. Explicit
iteration, result, collection, pattern, and nested declaration binders are
method locals too; ordinary `store`/`change` statements still target a
same-named property, and a constant declaration cannot silently replace one.

Container inheritance cycles are fatal analyzer errors and inherited member
lookup remains cycle-safe. A child that redeclares a concrete inherited
property with an incompatible concrete type receives a compatibility warning:
the runtime flattens that member to one mutable slot, so the two annotations
cannot both be enforced as independent storage contracts.

Static method state is synchronized at nested-call boundaries and whenever an
asynchronous handler parks or resumes. Concurrent handlers keep independent
active-method context stacks while merging completed static-property writes
back to the shared container definition.

## Compatibility-limited property work

Four property cases require a broader language/runtime migration and are not
silently redefined by the checker:

- A property with a concrete annotation but no default is represented as
  absent on a new instance and as `Null` for static state, while older WFL
  programs may populate it later or omit an inherited property deliberately.
  Treating every such read as a definite `T` is not fully sound, but immediately
  making it required would reject supported programs. The compatible repair
  needs definite-initialization or `Optional<T>` state, an initialization
  transition, diagnostics, and the governance deprecation period (at least one
  year for an unavoidable break).
- Runtime list values are shared. A list supplied to a typed property, read
  back from that property, returned by a method, or reached through a nested
  projection can retain the same storage under an ordinary lexical alias.
  Direct mutations of the property are checked, but a complete invariant also
  needs protected-alias/path constraints that follow storage in both
  directions and through user-code summaries. Deep-copying at the property
  boundary is not an equivalent quick fix because it would change intentional
  sharing semantics.
- Plain action and method annotations are historically static hints rather
  than runtime guards, and gradual or `Nothing` arguments are accepted at
  those call boundaries. A typed parameter can therefore carry a runtime value
  that would be rejected by a direct concrete-property assignment. Closing
  that path requires either runtime casts/guards (with a compatibility
  migration) or property-effect summaries that impose stricter requirements
  only on callers whose arguments reach a concrete property.
- A child container can historically redeclare an inherited property using an
  incompatible annotation. The analyzer now warns because runtime lookup
  exposes one flattened slot, but making the override a hard error immediately
  would reject supported programs. Enforcing invariance requires the
  governance deprecation path and migration diagnostics.

Until those migrations land, contributors must not describe concrete
no-default properties, aliased mutable property storage, unchecked parameter
boundaries, or incompatible inherited overrides as fully statically sound. New
code should give concrete properties compatible defaults, keep inherited
property annotations compatible, and avoid exposing mutable property lists
through aliases when a persistent element contract is required.

## Intentional uses of `Any`

`Any` remains part of the design, but every occurrence should fit one of these
categories:

- data whose runtime schema is explicitly dynamic, such as parsed external
  payloads, heterogeneous database records, or unknown imports;
- an unrepresentable union of incompatible concrete results, such as `Number`
  on one reachable path and `Text` on another;
- the varying member position inside a preserved outer shape, such as
  `List<Any>` or `Map<Text, Any>`;
- a value that crossed an opaque mutation boundary where shared runtime state
  can no longer be proven precise;
- a runtime construct whose completion value is intentionally dynamic.

Outer structure must be retained whenever possible. For example, joining
`List<Number>` with `List<Text>` produces `List<Any>`, not top-level `Any`.
`Unknown` is used for missing evidence, and `Optional<T>` is used for
`T | Nothing`; neither should be replaced with `Any` merely for convenience.

## Temporal identity and compatibility

Runtime dates, times, and date-times have dedicated `Date`, `Time`, and
`DateTime` types. This prevents a user container with a similar name from being
accepted accidentally by a temporal built-in.

Historical source annotations that parsed as custom temporal names remain
compatible when they are not a statically known same-named container. Unknown
imports keep this decision gradual: the checker permits the call and the
runtime validates the actual value. A known `ContainerInstance` never passes a
temporal built-in contract merely because its container is named `Date`,
`Time`, or `DateTime`.

The source spelling rules are compatibility-sensitive. Older colon-style
container annotations treated lowercase or mixed-case identifiers such as
`number` as custom type names; parser changes must not silently reinterpret
those existing programs.

## Runtime parity checklist

Before considering a type-system change complete, verify all of the following:

- every expression and statement operand is traversed;
- checker reuse does not carry symbols or diagnostics between programs;
- every implicit runtime binding has an analyzer and checker type;
- static acceptance has a corresponding successful runtime path;
- a statically rejected concrete type would also fail at runtime;
- overload ranking and container inheritance agree in both layers;
- dynamic values remain gradual without hiding errors in their child
  expressions;
- formatting or auto-fixing preserves type identity on reparse;
- tests cover both valid and invalid programs, plus downstream use of inferred
  results.

Run the repository's complete formatting, Clippy, Rust test, release build,
integration, web, and documentation-example gates after focused tests pass.

---

**Related:** [Compiler Internals](compiler-internals.md) |
[Architecture Overview](architecture-overview.md) |
[Testing Guide](../../Docs/guides/testing-guide.md)
