# 2026-11-19 — CodeRabbit follow-up on the `dev` → `main` sync

CodeRabbit left ten unresolved Major threads on #810. This change verifies
each one on current `dev` and fixes the ones that still reproduce. It does
not touch #810's branch.

## What was wrong

1. **Dependabot semver text.** GOVERNANCE §3.10 requires every update to be
   semver-compatible. The issue-policy eligibility rule named only directly
   updated packages, so a lockfile that also bumped a transitive major could
   still look eligible.
2. **Fixer `find all` token.** `find all patterns` is one identifier. A local
   `all patterns` was a rename candidate; the fixer protected only `patterns`,
   so it rewrote the top-level phrase to `find all_patterns`.
3. **Invalid DNS mode without servers.** `plan` rejected `dns_servers_mode`
   typos only when `dns_servers` was set. On Linux, `sometimes` with no list
   warned and still used the system resolvers, contradicting fail-closed
   FR-13.
4. **Silent AAAA after a usable A.** `tokio::join!` waited for both families.
   A server that answered A and dropped AAAA delayed the lookup by the full
   timeout (`2.003s` in the Red test), which can miss the request deadline.
5. **`random_seed` parameter vs file-level alias.** `BuiltinUses::bound` was
   program-wide. An action parameter named `random_seed` suppressed a later
   `store seeded_rng as random_seed` in the same file. #811's REPL session
   bindings did not cover this file lint.
6. **Legacy `find all` after an action.** Runtime looks up `all <name>` at
   the call. The analyzer walked the action body before a later
   `create pattern all needle`, chose the new reading, and reported
   `needle` undefined.
7. **Nested `find all` in the legacy fallback.** Not a bug. `legacy_find_only`
   is the pre-`find all` parse of the outer expression, and it is what keeps
   nested parsing polynomial (`nested_find_all_parses_in_polynomial_steps`).
   Mixing a legacy outer with a new inner would be new meaning, not
   compatibility.
8–10. **Test isolation.** Docs/DNS tests inherited machine-wide `.wflcfg` and
   ambient HTTP proxies; the missing-system-list mixing test was gated
   `cfg(unix)` so it would fail on macOS.
11. **Later top-level `all <name>` rewrote an earlier top-level `find all`.**
    Treating every later binding as live at analysis time made a valid
    nested `find all` take the legacy fallback and report `all letters`
    undefined. Runtime reads top-level bindings in order.
12. **Later alias used as `find all pick of word`.** The fallback is
    `all pick of word`. FunctionCall ignored `assumed_defined`, so a valid
    action-body program failed with `Variable 'all pick' is not defined`.

The Windows flake `could not bind UDP and TCP on the same port` was a related
test-harness issue: a failed TCP bind dropped the UDP socket, and Windows
often reissued the same ephemeral port on the next `:0`.

## What changed

- Issue policy: lockfile (including transitive) versions must be semver-compatible.
- Fixer protects the full merged `all <name>` spelling.
- Invalid `dns_servers_mode` fails closed even without `dns_servers`.
- `ask` returns once either family has addresses; a ready sibling is still merged.
- `BuiltinUses` restores `bound` after each action body, and records whether
  a mention was shadowed at the site so in-body parameter mentions are not
  treated as builtin uses after that restore.
- Analyzer treats later top-level `store` / `create pattern` names as live
  for the legacy `find all` choice only inside action/method bodies.
  Immediate top-level code still reads bindings in order. A later alias
  used as `of` inside a body is assumed callable (signature at runtime).
  The typechecker also consults its live container-property lookup, because
  it typechecks method bodies after the analyzer has restored
  `current_container`.
- Tests load local config only, disable proxies, and gate the mixing test to
  Linux. The fake DNS server holds failed UDP binds so retries get a fresh port.

Red evidence is the test-only commit that precedes this change.
