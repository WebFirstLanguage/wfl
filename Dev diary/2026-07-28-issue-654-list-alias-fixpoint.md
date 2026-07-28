# 2026-07-28 — issue #654: the list-alias relation had no fixpoint

## Symptom

Starting with 26.7.53, `wfl lib/scribe.wfl` from
[WebFirstLanguage/wfl-web](https://github.com/WebFirstLanguage/wfl-web) never
returned. One core pinned at ~100%, RSS climbing, **no diagnostic of any kind**.

The practical failure is worse than a hard error: running the full site, the
process stays alive, logs its usual `ANALYZE-*` warnings, never prints
`listening on port`, and never accepts a connection. `systemd` sees a healthy
live process. This is what pinned https://wfl.fyi to 26.7.47 in production.

A `perf` profile from the reporter put the hot loop in
`TypeChecker::list_alias_members_for_path`, with `hashbrown::map::HashMap::insert`
at 26% of samples — an unbounded insert loop.

## Root cause

`ListAliasPath` is the key of the flow-sensitive may-alias relation:

```rust
struct ListAliasPath {
    binding: SymbolBindingKey,
    index_depth: usize,          // unbounded
}
```

Two operations *synthesize* paths at a translated depth rather than only
reading existing ones:

* `list_alias_members_for_path` walks every ancestor relation on the same
  binding and re-reports its members at `alias.index_depth + offset`.
* `add_structural_list_alias` materializes known descendants at
  `target.index_depth + descendant.index_depth - source.index_depth`.

Both are correct for a *tree*. Neither terminates on a *cycle*. When a binding
transitively aliases itself at a different depth, each translation yields a
strictly deeper path; that path is a brand-new `HashMap` key; and on the next
pass it qualifies as a "descendant" to be translated again. The relation's key
space was `bindings × ℕ` — infinite — so it had no fixpoint to converge to.

Because the growth happens *within* a single statement, the run-budget poll in
`check_statement_types` never gets a turn, which is why nothing was ever
reported.

## Reducing it

The reporter could not reduce this below the original 2374-line file and
suspected it needed "a critical mass of interacting alias edges". It does not.
Bisecting on push count, the minimum is **three lines**:

```wfl
store scope as [1]
push with scope and scope
push with scope and scope
```

One self-push always terminated; two never did. That matches the mechanism
exactly — the first push records `scope@0 <-> scope@1`, and from then on
every `list_alias_members_for_path(scope@0)` reports both depths, so the second
push translates the relation upward and the depth compounds from there.
Measured directly, depth reached 2081 after 64 applications and was still
climbing.

The reporter's `binds`-inside-`sc_scope` shape is the same cycle drawn through
a map (list → map → list), which is why no single statement in `scribe.wfl`
looks suspicious.

## Fix

Bound the depth the relation tracks:

```rust
const MAX_LIST_ALIAS_INDEX_DEPTH: usize = 8;
```

This makes the key space `bindings × 9` — finite — so the relation is
*guaranteed* to stabilize. Enforced at the three chokepoints where a path can
enter or escape the relation, so no synthesized path can slip past:

1. `add_list_may_alias_edge` — the sole write path for a new relation.
2. `union_list_alias_bindings_in` — the join write path.
3. `list_alias_members_for_path` — the read path, whose synthesized members
   every caller feeds straight back in.

Two decisions worth recording:

**Drop from the relation, don't clamp into it.** Saturating a too-deep path
onto the bound as a relation *key* would make `apply_effect_at_list_path` apply
a mutation at the wrong nesting level, where `ListMutationEffect::Replace`
could overwrite a type the program never named. Dropping the key degrades to
"not tracked this deep", which is what the checker did before aggregate paths
were tracked at all.

**But the effect still has to land.** Review caught that dropping the *effect*
along with the key breaks valid programs, and it reproduces: ten nested lists,
`inner` extracted from the outermost one, a push at `inner@8` (so `level9@9`,
one past the bound), and reading the element back through the original path is
rejected with "Cannot assign value of incompatible type" for a widening the
program performed legally. Nesting past the bound is still *finite structure*,
not a cycle, so the aggregate really was widened and the checker was left
holding its stale `Number` element type. `list_alias_effect_members_for_path`
therefore reports an over-deep member clamped, flagged, and the effect applied
there is `Escape` rather than the real one: widening the deepest tracked
ancestor to `Any` subsumes anything the mutation would have done further down.
Escape is the most permissive of the three effects, so this can only cost
precision at those depths; unlike a clamped `Replace` it cannot pin an
unrelated depth to a narrower type. Nothing re-enters the relation, because
every depth it returns is capped and the paths it records on the deferred
effect stack are replayed as `Escape` anyway, so the key space stays finite and
the fixpoint argument above is untouched.

**The bound is measured, not guessed.** The static type cannot supply it —
`Type::Any` makes `type_at_alias_path` answer at every depth, so a gradually
typed program has no structural ceiling. Instrumenting
`add_list_may_alias_edge` across the whole `TestPrograms/` corpus — including
the comprehensive container, pattern, list and web-server programs — the
deepest path any acyclic program reaches is **2**. Eight leaves 4× headroom.

The constant matters for more than correctness: a saturated cyclic relation
costs roughly its fifth power, because each push iterates roots × depths ×
sources × descendants × members. At 16 a pathological twelve-line program took
14s; at 8 it takes 452ms.

## Result

| Program | Before | After |
|---|---|---|
| `lib/scribe.wfl` (2374 lines, the report) | hang (killed at 60s+) | **553 ms**, exit 0 |
| 3-line minimal repro | hang | 19 ms |
| 12-line list/map/list cycle | hang | 452 ms |
| `TestPrograms/list_alias_cycle_termination.wfl` | hang | 49 ms |

## Tests

Red evidence is commit `c760ec3`, a test-only ancestor of the fix. Three
layers, per the Logbie Testing Policy — risk class **R3** (backward
compatibility, and a liveness/termination property):

* **Unit** (`src/typechecker/mod.rs`) — drives the relation directly and
  asserts it reaches a fixpoint, plus that synthesized members never escape
  the relation's ceiling. Deliberately written so a non-converging relation
  *fails an assertion* rather than hanging the harness.
* **Integration** (`tests/typechecker_list_alias_depth_test.rs`) — type-checks
  each cyclic shape on a worker thread under a 30s deadline: direct self-push,
  the list/map/list cycle, a cycle established inside a loop, a cycle closed
  across an action boundary, mutual pushes, and — the guard on the bound
  itself — deep *acyclic* nesting that must still type-check cleanly.
* **End-to-end** (`TestPrograms/list_alias_cycle_termination.wfl`) — the same
  shapes as a program that must run to completion.

The negative-direction test matters most for regression: the bound must fire
only on cycles, never on real structure, so `deeply_nested_acyclic_lists_still_type_check`
asserts *no* diagnostics rather than merely asserting termination.
`mutation_below_the_depth_bound_still_reaches_the_original_aggregate` extends
that guard past the bound itself: it failed on the first bounded implementation
with the false "Cannot assign value of incompatible type" diagnostic described
above, which is the Red evidence for the escape-instead-of-drop refinement.

## Follow-up not taken

The report also asked for a typechecker that "times out with a diagnostic
rather than hanging". Deliberately not added: with a finite key space,
non-convergence is now structurally impossible, so such a timeout would be
unreachable code. The existing per-statement budget poll is the remaining
safety net, and it works again now that each statement terminates.
