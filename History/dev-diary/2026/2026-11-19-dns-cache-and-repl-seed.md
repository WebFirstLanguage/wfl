# 2026-11-19 — DNS cache/search fixes and REPL `random_seed`

Devin Review on the `dev` → `main` sync (#810) reported four bugs. All four
reproduced; none were false positives.

## What was wrong

1. **Search-expanded cache TTL.** `lookup` caches the final result under the
   original hostname. After `app.corp.test` returned NXDOMAIN with a short
   negative TTL and `app.` resolved with a long positive TTL, the positive
   entry hid `app.corp.test` for up to an hour. The negative-answer path
   already tracked earlier expiries; the positive path ignored them.
2. **Cache generation vs `resolv.conf`.** `system_config` dropped its lock
   before the lookup read the cache generation. A concurrent reload could
   bump the generation in that gap, so `insert_if_current` accepted an answer
   from the old nameserver as current.
3. **Mixed A/AAAA failures stopped search.** `combine` kept IPv4's timeout
   when AAAA returned SERVFAIL, with `family: None`. `search_one` treated
   that as complete silence, so `lookup` never tried the next search
   candidate (including the bare name). Assumption 38 already said SERVFAIL
   should continue; only a timeout on both families is silence.
4. **REPL `random_seed` variable.** `rng_security_ingredients` scans only the
   current submission. A later mention of a user variable named `random_seed`
   (bound on an earlier line) looked like the builtin, marked the session
   seeded, and blocked crypto.

## What changed

- Bound a search-expanded positive cache entry by preceding negative
  expiries; skip caching when a preceding failure has no TTL.
- Take the resolv.conf snapshot and cache generation together, while the
  configuration lock is still held.
- Prefer a family that actually replied when combining A and AAAA, so
  timeout+SERVFAIL is not stored as complete silence.
- Fold top-level `store` and action names from earlier REPL submissions
  into the security scan. Nested names (action parameters, loop variables)
  are omitted so they cannot hide a later mention of the real builtin.

Red evidence is the test-only commit that precedes this change.
