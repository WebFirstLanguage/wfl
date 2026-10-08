# 2026-10-08 — Unbounded quantifiers matched the shortest run (#709)

## Symptom

```wfl
create pattern digits:
    one or more digit
end pattern

store s as "a1b22c333"
display replace digits with "#" in s
// actual:   a#b##c###
// expected: a#b#c#
```

`find`, `find all`, `split ... on pattern`, `replace`, and `capture` all
returned one character for `one or more digit`. Exit 0, no diagnostic.
`zero or more digit` on `"12345"` matched empty; `at least 2 digit` on
`"a12345b"` matched `"12"`. Bounded `2 to 4 digit` already matched `"1234"`.

The shipped docs example — `one or more letter` over `"The quick brown fox"` as
word extraction — returned 16 letters.

## Root cause

`find_at_position` returned on the first VM state that reached `Match` while
sweeping the BFS frontier. `Split` queues both branches as peers, so the exit
arm of a `OneOrMore` / `ZeroOrMore` / `AtLeast` loop won before the loop-back
arm could consume another character.

`Quantifier::Between` unrolls its optional repetitions inline with no `Jump`.
The try-more thread is first in the next generation and happens to hit `Match`
first — which is why the bounded form looked greedy while the unbounded forms
did not. Extent depended on bytecode shape, not on a stated rule.

`greedy` and `lazy` are lexed and listed as reserved words but are not accepted
in pattern syntax. They do not define a lazy default. The docs treat
`one or more` as a repeated class and as word extraction, so the specified
default is greedy.

## Fix

`find_at_position` is now Pike-style greedy: `Split`'s first branch is higher
priority; a `Match` records a candidate and drops lower-priority peers in that
generation; already-queued higher-priority continuations keep running so the
loop can extend; the last recorded match is the extent.

A cross-thread `(pc, pos)` visited set is **not** used. This VM's generations
are `step()` waves that stop at the next `Split`, so the first arrival at
`(pc, pos)` is the thread that passed the fewest Splits, not the highest-priority
one. Deduping there pruned nested-quantifier/alternation threads and, with
backreferences, dropped the thread that had set the capture (`find` missed
while `matches` still succeeded).

Backreference-free `find` is a lockstep Pike VM: threads advance one input
position at a time, `addthread` follows epsilon in priority order with a
per-position `pc` set, and a Split-choice path keeps left-first `or` from
being overwritten by a longer later arm. Empty quantifier loops terminate
because the same `pc` is not re-entered at the same position. Programs that
contain a `Backreference` keep the no-dedup sweep, with a per-thread
empty-iteration cutoff in `step` and the per-match meter as the bound.

Without Pike, a failing higher-priority arm such as `(one or more (letter
or letter) then "!") or letter` exceeded the default state budget at 12
letters; `main` returned `a` in a handful of steps. That is R3 (untrusted
input / resource exhaustion), not R2.

Boolean `execute_at_position` still returns on first success — it does not
report extent. Ordered `or` stays left-first (`"1" or "12"` on `"12"` is
`"1"`), not POSIX leftmost-longest.

## Verification

Red: `cargo test --lib quantifier_extent_tests` at the test-only commit showed
`"1"` / `""` / `"12"` / 16 letters. Bounded `2 to 4` and left-first
alternation already passed. On the `(pc, pos)` visited revision the real
binary gave `ba` / `1` / no-match for the nested-alt and backreference cases
in the #752 review (the first nested-`one or more` case already returned
`bb` on the binary).

Green: the same unit tests plus binary-level `find` / `find all` / `split` /
`replace` / `capture` in `tests/issue_709_quantifier_greed_test.rs`,
`TestPrograms/patterns/unbounded_quantifier_greed.test.wfl`, and the four
reviewer cases after dropping the visited set. After that drop, `zero or more
""` and `zero or more optional "a"` spun until the step ceiling; the
per-thread empty-iteration cutoff records the empty / greedy match instead.
