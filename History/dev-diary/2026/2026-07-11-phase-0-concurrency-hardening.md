# Dev Diary — 2026-07-11: Phase 0 concurrency hardening

## Context

`Docs/development/concurrency-phase-plan.md` lays out a phased plan to give WFL a
real concurrency model without a risky interpreter rewrite. **Phase 0 ("Kill the
live DoS")** is the first, lowest-risk slice: no interpreter-core redesign, no new
language surface, plain `main loop` stays serial. It closes three concrete gaps
that a public WFL web server exposes today:

1. **CPU-bound crypto stalls the whole process.** Password hashing (Argon2,
   scrypt, bcrypt, PBKDF2 at 600k rounds) is *deliberately* slow and ran inline on
   the single interpreter thread, so one login-like call froze every other request
   — a cooperative-scheduling DoS.
2. **The request queue was unbounded.** The transport→interpreter channel was an
   `mpsc::unbounded_channel`, so a flood of accepted requests could grow memory
   without bound.
3. **The docs claimed parallelism we don't have.** Several pages said handlers run
   "concurrently" / "don't block others" while they actually run one at a time.

This entry covers all three sub-PRs (0a/0b/0c) that landed together on
`claude/phase-0-implementation-rpbdjp`.

## What changed

### 0a — Docs honesty + `panic = "unwind"` gate

- **Panic gate.** The runtime will rely on `catch_unwind` to contain a panicking
  request handler so its siblings survive (Phase 1). That is unsound under
  `panic = "abort"`. Enforced with `#[cfg(panic = "abort")] compile_error!(...)`
  in `src/lib.rs` — evaluated with the crate's *actual* panic strategy, so it
  fails a real abort build but never trips `cargo test` (Cargo force-unwinds test
  harnesses).
  - **Dead end worth recording:** the first attempt used a `build.rs` check on the
    `CARGO_CFG_PANIC` env var. It does not work — build scripts run on the host
    and always see `unwind`, even under `--config 'profile.release.panic="abort"'`
    (verified empirically). A `#[test]` on `cfg!(panic = ...)` is likewise a
    phantom control. The crate-level `compile_error!` is the only mechanism that
    reflects the real target panic strategy.
  - `Cargo.toml` `[profile.release]` now pins `panic = "unwind"` explicitly
    (self-documenting; default was already unwind).
  - `.github/workflows/ci.yml` `clippy-and-test` job gains an "Assert panic=abort
    is rejected" step that forces abort via `--config` and **fails if the build
    succeeds** — the failing-first control proving the gate is live.
- **Docs.** Rewrote overclaims to distinguish the concurrent *transport* (accept /
  TLS) from *serial application handlers*, and preferred "concurrent" over
  "parallel". Primary rewrite in `async-programming.md`; `web-servers.md` was
  already honest and served as the model.

### 0b — `spawn_blocking` for CPU-heavy crypto

- New `src/stdlib/crypto_async.rs` with a name-keyed `route(name, args)` that hops
  the 11 heavy crypto builtins onto Tokio's blocking pool. Chosen over a new
  `Value::AsyncNativeFunction` variant to avoid rippling a new arm through every
  exhaustive `Value` match for zero user-visible benefit.
- The interpreter's two async native-dispatch arms (`FunctionCall`, `ActionCall`
  in `src/interpreter/mod.rs`) now check `route()` first and `.await` it, falling
  back to the synchronous native otherwise.
- The heavy compute stayed in plain-data helpers in `src/stdlib/crypto.rs`
  (`argon2_hash_str`, `*_verify_str`, `pbkdf2_hmac_sha256_str`, …), now
  `pub(crate)`. Arguments are extracted into owned `String`/`u64`/`usize` on the
  interpreter thread *before* the hop; only that plain data crosses into
  `spawn_blocking`; the `Value` is rebuilt after the `.await`.
- **Interpreter core stays `!Send`.** No `Rc`/`RefCell`/`Value`/`Environment` ever
  crosses a thread boundary (HARD RULE 9 not triggered). `zeroize` and the
  `subtle` constant-time compare paths are untouched — no early-exit refactor.

### 0c — Bounded request queue (OOM shed)

- New `.wflcfg` key `web_server_request_queue_bound` (default `256`, zero
  rejected) in `src/config.rs`.
- The transport→interpreter channel is now `mpsc::channel(bound)` instead of
  `unbounded_channel`. The warp handler uses `try_send`: on `Full` it logs a
  structured warning and returns a `503` (with `Retry-After`) via the new
  `overloaded_response()` helper, without blocking the transport task or awaiting
  the per-request oneshot; on `Closed` it keeps the existing rejection.
- WebSocket channels remain unbounded — out of scope, noted as a follow-up.

## Tests

- `src/stdlib/crypto_async.rs` unit tests (in-crate because `route` is
  `pub(crate)`): exact routed-set map; a **deterministic off-thread proof** — on a
  `current_thread` runtime a concurrently-spawned ticker only advances during the
  crypto `.await` if the work was offloaded (independent of core count, no timing
  thresholds); routed hash/verify round-trips (argon2, bcrypt); routed PBKDF2 is
  byte-identical to the direct helper; argument errors still surface.
- Existing crypto suites (`crypto_kdf_test`, `password_hashing_test`,
  `crypto_test` — 44 tests) pass unchanged through the new routed dispatch path.
- `src/interpreter/mod.rs` `queue_bound_tests`: `overloaded_response()` is a
  well-formed 503 (Content-Length matches body); a full bounded channel sheds
  deterministically via `try_send` → `Full`.
- `tests/web_queue_bound_test.rs`: `web_server_request_queue_bound` parsing —
  default 256, valid override applied, zero and non-numeric rejected.
- Existing web-server tests (`http_request_runtime_test`, `respond_headers_test`,
  `route_params_test`, `websocket_test` — 30 tests) pass with the bounded queue at
  the default.

## Docs

- `Docs/04-advanced-features/async-programming.md`, `index.md`,
  `Docs/01-introduction/key-features.md`,
  `Docs/06-best-practices/performance-tips.md`, `Docs/Archive/README.md`,
  `Docs/04-advanced-features/web-servers.md` — honesty pass (concurrent transport
  vs serial handlers; concurrent ≠ parallel).
- `Docs/reference/configuration-reference.md` and `web-servers.md` — document
  `web_server_request_queue_bound` and the 503 shed behavior.
- `Docs/development/concurrency-phase-plan.md` — flipped 0a/0b/0c to ✅.
- `CLAUDE.md` — added a **"Docs must be honest (validate docs)"** policy: docs
  describe what actually ships (planned behavior marked as such), and every
  user-visible change ships validated docs *and* a Dev Diary entry in the same
  change.

## Notes / Follow-ups

- Phase 0 is cooperative only: CPU-bound *non-awaiting* WFL code still stalls the
  process. Concurrent request handlers arrive in Phase 1 (`main loop concurrently:`).
- The crypto auto-call dispatch path (arity-0 natives) is intentionally not
  routed; every heavy crypto builtin takes ≥ 1 argument. A future arity-0 heavy
  builtin would also need routing.
- WebSocket outbound/event channels are still unbounded — a Phase 1 follow-up.
- `#![deny(clippy::await_holding_refcell_ref)]` already guards `src/lib.rs`, which
  is the clippy backstop Phase 1 will lean on.
