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

## Follow-up: maintainer deep-review fixes (same day)

Two maintainer review rounds found five gaps, all fixed in the same PR:

1. **Container-typed overloads were statically rejected** — parameters
   annotate as `Custom("Dog")` but instances infer as
   `ContainerInstance("Dog")`, and neither static compat function paired
   them. Added `Analyzer::container_is_or_extends` (depth-guarded `extends`
   walk) and Custom↔ContainerInstance arms (with ancestry) to both
   `is_type_compatible` and `are_types_compatible`.
2. **`nothing` passed static resolution but failed runtime dispatch** —
   `value_matches_type` now accepts `Null`/`Nothing` for every parameter
   type, mirroring the static `(_, Nothing) => true` rule; ties fall to
   definition order.
3. **Stored actions were not callable statically** — new analyzer
   `action_aliases` (variable → action, recorded for bare `store h as f`
   references, cleared on reassignment); both analyzer call arms and both
   typechecker call paths resolve through the alias, so aliased calls get
   full overload resolution and per-overload return types.
4. **Temporal dispatch hole** — a call between two overload definitions ran
   the lone first overload's body with a non-matching argument
   (`call_function` only checked arity). Declared parameter types are now
   enforced in `call_function` itself: a concrete annotation that rejects an
   argument value is a runtime error, so a single typed action behaves like
   an overload set of one and calls dispatch on "the overloads defined so
   far". (`nothing` and `any`/untyped parameters still accept everything.)
5. **`Value::Overloaded` equality** — added `Rc::ptr_eq` arms to both the
   `PartialEq` fast path and `eq_with_visited`, so an overload set equals
   itself and distinct sets compare unequal.

Full-pipeline regression tests (analyze → typecheck → interpret) cover each
finding in `tests/overload_test.rs::full_pipeline`.

## Follow-up: maintainer deep-review round 3 (same day)

Round 3 found four P1 gaps in the round-2 fixes, all in the new machinery:

1. **Alias snapshots.** The analyzer aliased only `variable → action name` and
   resolved against the complete signature list, contradicting the runtime's
   snapshot semantics. `AliasState::Bound { action, visible_signatures }` now
   records how many overloads were lexically defined before the binding (a
   PASS-2 walk counter; PASS 1 pushes signatures in lexical order, so the
   count is a prefix length), and both analyzer call arms and both
   typechecker call paths resolve against that prefix only.
2. **`call ... with` aliases in the typechecker.** Only the `of`-form path
   was alias-aware. Both paths now consult the analyzer's per-call-site
   record (below).
3. **Temporally sound alias state.** Alias mutations no longer leak from
   code that hasn't executed: action/method/handler bodies save and restore
   the alias map (and overload counters), and control-flow constructs
   (`check if`, single-line if, all loops, `try`) analyze each branch from
   the entry state and join the results — a name whose state differs across
   paths degrades to `AliasState::Dynamic`, which skips static validation
   and defers to runtime dispatch instead of misjudging. The typechecker no
   longer reads the analyzer's *final* alias map at all: the analyzer records
   what each alias call site resolved to in `alias_call_sites` keyed by
   (callee, line, column), so the typechecker observes per-statement state.
4. **Runtime type enforcement scoped to overloads.** Round 2's unconditional
   `call_function` check turned every annotation into a runtime guard,
   breaking legacy dynamically-typed calls to typed actions defined only
   once (a
   backward-compatibility violation). `FunctionValue.enforce_param_types`
   (a `Cell<bool>`) now gates it: set for names the interpreter's program
   pre-scan finds defined more than once in the same block (so the first
   member of a future overload set enforces during the temporal window), and
   set on every member when `define_or_merge_action` merges (covering
   include-driven overloads; the shared `Rc` also flags captured snapshot
   references). Single-definition actions keep their historical dynamic
   behavior.

One adaptation from the review's illustrative programs: `change h to 0` on a
function-typed variable is a pre-existing static type error (incompatible
assignment), so the regression tests exercise the alias-clobber scenarios by
reassigning to a *different action* of compatible shape instead.

## Tests

All overload coverage lives in the single `tests/overload_test.rs` crate
(consolidated so CI runners don't run out of disk linking extra test
binaries), organized as `analyzer`, `typechecker`, `interpreter`, and
`full_pipeline` modules — 48 tests through the round-3 review, 49 after the
post-review polish added a negative-path `call ... with` alias rejection —
plus `TestPrograms/action_overloading_comprehensive.wfl` end-to-end. One
existing unit test updated (`test_function_call_type_checking`) because the
analyzer now reports of-form argument type mismatches earlier with a more
specific message.

## Round 4 (fourth-pass review): path-sensitive counters and per-block enforcement

The maintainer's fourth pass found three P1 defects in the round-3
temporal/snapshot machinery itself:

1. **The visible-overload counter was not path-sensitive.** Control-flow
   handlers snapshot/joined only the alias map; the `defined_overloads`
   counter kept counting through branch bodies, so a definition inside a
   never-executed branch inflated the prefix a later alias captured — the
   alias could statically "see" overloads (including lexically later,
   PASS-1-registered ones) that the runtime value never held. The counter
   value type is now `OverloadCount::{Exact, Unknown}` and travels with the
   alias map through a combined `FlowState` snapshot/join in every branch
   construct (`check if`, single-line if, the three analyzed loop forms,
   `try`): paths agreeing on an exact count keep it; disagreement degrades
   to `Unknown`, which makes aliases bound afterwards `Dynamic` — static
   validation steps aside and runtime dispatch (always correct) decides.
   A related latent drift the review didn't name: an alias bound *inside*
   a branch could see a counter larger than the PASS-1 signature list
   (branch-nested definitions are counted but never registered), and the
   old clamped prefix statically rejected calls the runtime accepts;
   `alias_call_target` now detects the drift and defers to runtime too.

2. **The runtime pre-scan missed describe-nested tests.** The old
   whole-program `collect_overloaded_action_names` walk had no
   `DescribeBlock` arm, so an interleaved wrong-type call inside a test
   under `describe` executed the wrong body instead of erroring.

3. **The pre-scan's flat name set over-guarded sibling blocks.** One
   block's overloads runtime-guarded a different block's lone typed action
   of the same name, breaking the restored backward compatibility.

Fixes 2 and 3 share one mechanism: the global pre-scan is gone. Each
statement block now computes its own immediate-slice duplicate set at
execution entry (`scan_block_overload_dups`, installed via a small RAII
`BlockDupsScope` guard in `_execute_block`, the top-level program loop, and
the describe/test executors — restored on drop, so `?` early returns and
awaits are safe). `ActionDefinition` execution seeds `enforce_param_types`
from the *current block's* set only; `define_or_merge_action`'s merge-time
flagging still covers include-driven and cross-block merges after they
happen. Block identity is inherent (no identity keys needed), describe
setup/tests/teardown and test bodies are three ordinary blocks, and module
statement blocks gain first-definition temporal enforcement they previously
lacked — a strict improvement, noted here so it is not mistaken for a
regression. The scan is one discriminant-only pass over the immediate slice
per block entry (no allocation unless a name repeats), noise next to
per-statement dispatch cost.

Notes for future rounds: `RepeatWhileLoop`/`RepeatUntilLoop`/`ForeverLoop`/
`MainLoop` bodies are not visited by this analyzer's PASS-2 walk at all (a
pre-existing gap — their nested definitions and aliases are invisible to
semantic analysis), and static analysis cannot yet resolve calls to actions
defined inside action bodies, which is why the sibling-block leniency
scenario is pinned in `tests/overload_test.rs` rather than the TestProgram.

Round-4 coverage: six new tests (55 total) — branch/loop counter-leak
deferral, the maintainer's exact program as a dispatch guardrail, in-branch
alias drift, describe-nested temporal rejection (via `set_test_mode` +
`get_test_results`), and sibling-block leniency.

## Post-round-4 dispatch refinement: `nothing` and specificity

Copilot spotted that `value_matches_type`'s early accept for `nothing`
inflated `select_overload`'s specificity count: a `nothing` argument earned
"concrete match" credit for typed parameters, so a version with more
annotations beat the documented definition-order rule (e.g. `f(a as number,
b)` vs `f(a as text, b as number)` called with two `nothing`s picked the
second). `nothing` now earns credit only for an explicit `as nothing`
parameter — its exact match, which correspondingly wins specificity — and
otherwise leaves versions tied so definition order decides. Two regression
tests (57 total) plus a TestProgram section; the docs' `nothing` bullet and
the spec sentence now state both halves of the rule.

## Round 5 (fifth-pass review): block-entry arming and abrupt-exit joins

The maintainer's fifth pass found two P1 gaps:

1. **Cross-block merges left a temporal window open.** The per-block
   duplicate scan (round 4) only saw names defined twice in the *same*
   slice. A block whose definition merges with an action already present in
   the same scope (branches execute against the enclosing environment)
   left the existing member lenient until the merge — an interleaved
   dynamic call before the in-block definition ran the wrong body.
   `enter_block_overloads` now arms `enforce_param_types` on same-scope
   existing function members for every name the entering block defines
   (inherited names are untouched — defining over them is a shadowing
   error, not a merge). Wired through `_execute_block`, the top-level
   program loop (which also covers a REPL interpreter reused across
   snippets), and the describe/test executors. Arming is lexical, matching
   the established "defined more than once in the block" rule.

2. **Endpoint-only flow joins missed abrupt exits.** The `FlowState` join
   compared construct entry with the state after the whole body — but a
   `break`/`continue` (or an error transferring to a `when` handler) can
   exit while an alias holds an intermediate binding the body later
   restores, making the endpoints equal and the join wrong in the
   *false-rejection* direction: static analysis rejected calls the runtime
   accepts. Rather than collecting states at every exit point, the analyzer
   now tracks which alias names are written at any point inside each
   loop/`try` body (`alias_mutation_frames`, a stack whose pop merges into
   the parent frame so inner-body mutations count for outer bodies) and
   degrades exactly those names to `AliasState::Dynamic` after loop joins
   and at `try`-handler entry. Names the body never touched keep their
   precise state; endpoint joins remain sound for branch constructs, which
   cannot exit mid-branch. Overload counters need no frames — they are
   monotonic, so endpoint disagreement already catches every mutation.

Test-shape note: reassigning an alias across differently-shaped actions is
constrained by the pre-existing function-type assignment check (parameter
types compare positionally; untyped parameters are compatible with
anything), so the round-5 regressions reassign to an untyped-parameter
action — the same leniency real dynamic code would use.

Round-5 coverage: three new tests (60 total) — cross-block temporal
enforcement, loop-break intermediate binding, and try-handler intermediate
binding (both formerly static false rejections).
