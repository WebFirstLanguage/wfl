# Dev Diary — 2026-07-24: concurrent-loop ordering guard + header-key type validation

Two review-driven hardening changes on the runtime-streaming branch, each with
Red→Green evidence at the lowest useful layer (semantic analysis / type check).

## 1. `main loop concurrently:` must begin with `wait for request`

**Problem (reviewer, CodeRabbit on `src/interpreter/mod.rs`):** the concurrent
main loop refills its handler set by starting `execute_block(body, …)` for every
slot up to the concurrency cap. Each future runs the body from the top. If the
body has any statement *before* the first `wait for request`, that statement runs
once per slot — speculatively, before a single request has been dequeued. For a
body that starts with `wait for request` the future simply parks on the request
channel, so nothing runs early; the hazard only exists for out-of-order bodies.

**Fix:** rather than restructure the loop into a heavier dequeue-then-run engine,
enforce the invariant the well-formed case already satisfies — a
`main loop concurrently:` body must begin with `wait for request`. Semantic
analysis now rejects a concurrent loop whose first statement is anything else,
with an actionable message that tells the author to move setup above the loop.
Serial `main loop` is unaffected (it runs one iteration at a time, so there is no
speculative fan-out). This is new surface (`concurrently` shipped on this branch),
so no existing program is affected; every example and test already starts with
`wait for request`.

- Code: `src/analyzer/mod.rs` — split the merged `ForeverLoop | MainLoop` arm so
  the concurrent case is checked; extracted the shared body walk into
  `analyze_loop_body`. The ordering error is pushed *before* the body is analyzed
  so it is not swept up by the handler-body error→warning demotion.
- **Risk class R3** (concurrency/lifecycle).
- **Red→Green:** `test_concurrent_main_loop_requires_wait_for_request_first`
  (inline in `src/analyzer/mod.rs`) parses real WFL source and asserts: (a) a
  concurrent loop with `store … ` before `wait for request` is rejected naming
  the ordering rule; (b) the *same* body under serial `main loop` is accepted;
  (c) a concurrent loop that starts with `wait for request` is accepted. Red was
  confirmed by neutralizing the guard (test failed), then restored (test passed).
- Docs: `Docs/04-advanced-features/web-servers.md` states the requirement; the
  existing `TestPrograms/docs_examples/web_servers/concurrent_main_loop.wfl`
  already begins with `wait for request`, so it needed no change.

## 2. HTTP header maps must have text keys

**Problem (reviewer, Copilot on `src/typechecker/mod.rs`):** the header type
checks for outbound HTTP (`http … with headers`), streaming responses, and
`respond … and headers` all accepted any `Map<_, _>`. HTTP header names must be
text, so `Map<Number, _>` passed typechecking even though it can never be a valid
header set — and the error message already promised "header names."

**Fix:** a single `is_valid_header_map_type` helper, used at all four sites, that
accepts a map only when its key type is `Text` (or `Unknown`/`Any`/`Error`, so a
header set the checker cannot fully resolve — map literals often infer an unknown
key — is never falsely flagged) and rejects a map with a concrete non-text key.

- Code: `src/typechecker/mod.rs` — helper + four call sites collapsed onto it.
- **Red→Green:** `test_header_map_type_requires_text_keys` covers accepted
  (`Map<Text,…>`, loose-key maps, `Unknown`/`Any`) and rejected (`Map<Number,…>`,
  `Map<Boolean,…>`, non-map) cases. Because map literals infer `Map<Text,…>`
  keys today, a concretely non-text-keyed header map is not reachable from source
  — this is a defensive guard, so the honest evidence is a unit test on the
  boundary that changed. Red confirmed by broadening the key match, then restored.

## 3. CI: validate Windows too; ignore the docs cache

Also on this branch (reviewer, Copilot on `.github/workflows/ci.yml`): the
Integration Tests job ran docs-example validation and the web-server integration
tests on Linux only, and trusted the committed validation cache. Now:

- Docs validation runs on both OSes with `--force` (ignores the cache so CI
  always re-validates).
- The web-server suite runs on Windows too via the existing
  `scripts/run_web_tests.ps1` (`shell: pwsh`), matching the testing profile's
  requirement that web tests run in CI rather than leaving Windows unvalidated.

## Residual risk

- The concurrent-loop guard is a static structural rule, not a runtime rewrite:
  it removes the speculative-side-effect footgun by construction, but the loop
  still starts its slots eagerly (each parked on `wait for request`). A full
  dequeue-then-run engine remains future work.
- Enabling the Windows web-test suite may surface pre-existing Windows-only
  behavior; if it does, that is a real signal to fix, not to re-hide.
