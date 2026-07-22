# WFL Concurrency Model — Phase Plan & TODO List

**Audience:** AI implementers (agent-sized PRs) + maintainer review  
**Status:** Ready to execute  
**Locked marker spelling:** `main loop concurrently:`  
*(Owner may override before Phase 1b; do not invent alternatives mid-implementation.)*

**Origin:** Adversarial brainstorm (2026-07-11) — full design rationale lives outside this repo as `brainstorms/2026-07-11-wfl-concurrency-model.md` (Starnet brainstorms). This document is the executable plan; keep status checkboxes updated as PRs land.

---

## Non-negotiable rules (every PR)

Paste into every agent task brief:

```text
HARD RULES:
1. Plain `main loop` stays serial. Concurrency requires visible `concurrently`. No silent same-text semantics swap.
2. No Rc→Arc/Send rewrite of the interpreter. No multicore workers unless this PR is explicitly Phase 3.
3. No nursery / `any of` / `change shared` / `start as handle` unless this PR is explicitly Phase 2.
4. Soundness order: sync critical-section closure (when shared state exists) > clippy backstop > catch_unwind last line.
5. CI must assert panic = "unwind"; without it catch_unwind is a phantom control.
6. TDD: failing tests first for concurrency properties.
7. Docs ship in the same PR for any user-visible behavior change.
8. Prefer unrepresentable hazards over "we'll be careful."
9. Stop and report if the change requires Send/Arc across the interpreter core.
10. One PR = one scope. Do not "while we're here" into the next phase.
```

### Five gates (from brainstorm)

| # | Gate |
|---|------|
| G1 | Concurrency marker visible and required; plain `main loop` serial |
| G2 | Sync critical-section closure is base soundness (when shared state exists) |
| G3 | CI asserts `panic = "unwind"` |
| G4 | Join/nursery engine is its own phased cost — not smuggled into Phase 1 |
| G5 | Document lost-update hazard and worker load-skew/drain as known obligations |

---

## Progress tracker

| Phase | PR | Title | Status |
|-------|-----|--------|--------|
| 0 | 0a | Docs honesty + `panic=unwind` CI | ✅ Done |
| 0 | 0b | `spawn_blocking` for blocking crypto | ✅ Done |
| 0 | 0c | Bound accept/queue (OOM shed) | ✅ Done |
| 1 | 1a | Runtime spike (bridge, no surface) | ✅ Done (folded into 1b) |
| 1 | 1b | `main loop concurrently:` surface + ops defaults | ✅ Done — awaiting maintainer review |
| 1 | 1c | Honesty docs for real concurrent model | ✅ Done |

> **Phase 1 landed in one change** (`Dev diary/2026-07-22-concurrent-request-handlers.md`),
> not the staged 1a→1b→1c sequence. What is covered: `main loop concurrently:`
> surface (locked marker); plain `main loop` byte-compatible serial (tested);
> `FuturesUnordered` of `!Send`, `&self`-borrowing handler futures on the
> existing runtime; isolated-per-request **environment** scopes; a slow handler
> not blocking a fast sibling (tested); in-flight cap
> (`CONCURRENT_HANDLER_LIMIT`), with 503/504/500 provided by the existing
> transport layer (bounded queue → 503, response deadline → 504,
> `ResponseCompletion` drop → 500); the empty-set busy-spin trap is avoided
> (cap ≥ 1 keeps the set non-empty). Cooperative, not parallel: handlers
> interleave only at await points, so a CPU-bound handler with no await still
> holds the interpreter thread (documented in `web-servers.md`).
>
> **Containment:** *runtime-error* containment is tested (an erroring handler
> does not kill the server). *Panic* containment is by construction via
> `catch_unwind` on each handler future but is **not** covered by a dedicated
> test — a deterministic WFL-level Rust panic is not readily expressible from
> the language, so the guarantee rests on the wrapper + the `panic = "unwind"`
> gate, not a regression test.
>
> **Run-state isolation (review gap — FIXED):** the concurrent loop already
> isolates the **environment** per handler; it now also isolates interpreter
> run-state. Each handler carries its own `RunState` (`current_count`/
> `in_count_loop`, `call_depth`, `call_stack`, and the block overload-dup set),
> and an `IsolatedHandler` poll wrapper swaps that state into the interpreter
> only for the duration of each `poll`, swapping it back out the instant the poll
> returns (ready *or* pending). While a handler is suspended at an `await`, its
> count-loop/recursion/call-stack bookkeeping is parked in its own `RunState`, so
> a sibling polled next neither sees nor clobbers it. Serial execution is
> untouched (the wrapper is used only by `execute_concurrent_main_loop`).
> Regression: `tests/concurrent_main_loop_test.rs::test_concurrent_handlers_do_not_share_count_loop_state`
> (Red without the swap: `/a` observed `/b`'s entire count range).
>
> **Also lighter than the full 1b checklist:** request-ID *structured* logging is
> not yet added; the eval-core `RefCell`-across-await audit is enforced
> mechanically by the crate-wide `#![deny(clippy::await_holding_refcell_ref)]`
> backstop rather than a written per-site walkthrough.
>
> **This is the maintainer STOP/review point** — please review before Phase 2.
| 2 | 2a | Structured nursery + join engine | ⬜ Not started |
| 2 | 2b | `change shared` critical region | ⬜ Not started |
| 3 | 3a | Multi-process workers (if profiling forces) | ⬜ Deferred |

**Legend:** ⬜ Not started · 🟡 In progress · ✅ Done · ⏸️ Blocked · ❌ Cancelled

---

## Phase 0 — Kill the live DoS (hours; ship first)

**Goal:** Stop whole-site stalls from CPU-bound crypto and fix false docs. Zero interpreter-core concurrency redesign.  
**Depends on:** Nothing  
**Out of scope:** Concurrent request handlers, nursery, shared state, multicore

### PR-0a — Docs honesty + panic profile CI

**Goal:** Tell the truth about today's serial model; make catch_unwind viable later.

#### TODOs

- [ ] Audit docs for overclaims: "parallel", "don't block others", "run concurrently" where handlers are still serial
  - Primary suspects: `Docs/04-advanced-features/async-programming.md`, indexes, archive blurbs
  - Keep accurate notes in `web-servers.md` (already more honest about sequential responses)
- [ ] Rewrite claims to distinguish:
  - **Transport** may accept/TLS concurrently
  - **Application handlers** today process one request at a time
  - Prefer "concurrent" over "parallel" for single-thread cooperative work
- [ ] Add CI assertion that the build/profile uses `panic = "unwind"` (hard gate for Phase 1)
  - Fail CI if profile is `abort` for the runtime that will use catch_unwind
- [ ] Dev Diary note: honesty fix + why the gate exists

#### Acceptance tests / checks

- [ ] Docs validation still passes (`python scripts/validate_docs_examples.py` if examples touched)
- [ ] CI fails when `panic = "abort"` is forced (or equivalent gate test)
- [ ] No remaining user-facing claim that serial `main loop` handlers "don't block others"

#### Out of scope

- Runtime behavior changes
- Crypto / spawn_blocking
- Syntax changes

---

### PR-0b — `spawn_blocking` for launch-blocking crypto

**Goal:** Move known CPU-heavy builtins off the interpreter thread (libuv / Node pattern).

#### TODOs

- [ ] Inventory all CPU-heavy crypto/hash builtins that run on the interpreter thread:
  - [ ] `pbkdf2_hmac_sha256`
  - [ ] `pbkdf2_hash` / `pbkdf2_verify`
  - [ ] `argon2_hash` / `bcrypt_hash` / `scrypt_hash` / `hash_password` (and verify counterparts)
  - [ ] WFLHASH / file hashing paths if they can monopolize the thread
  - [ ] `constant_time_equals`, `secure_random_bytes` only if measured blocking (likely lower priority)
- [ ] Implement `tokio::task::spawn_blocking` (or equivalent) hop for each listed heavy path
  - Keep interpreter core `!Send`; only plain data crosses the boundary
- [ ] Verify crypto properties survive the hop:
  - [ ] `zeroize` still applies to sensitive buffers
  - [ ] Constant-time compare path still used (no "helpful" early-exit refactor)
  - [ ] Same outputs / error mapping as before
- [ ] Add dual acceptance tests (see below)
- [ ] Update any misleading comments that claim native KDF alone prevents whole-site DoS (native-on-interpreter-thread still blocks cooperative work)

#### Acceptance tests

- [ ] **Responsiveness:** under a long PBKDF2/login-like call, a concurrent health/simple request still progresses (once Phase 1 exists this is stronger; for Phase 0 alone, at minimum prove the blocking work is off the runtime thread used for other awaits — design the test for what is actually concurrent today)
- [ ] **Correctness:** existing crypto unit/integration tests still pass
- [ ] **Invariants:** zeroize / constant-time paths covered or explicitly regression-tested
- [ ] No change to WFL call syntax for these builtins

#### Out of scope

- Concurrent `main loop`
- Eval-core RefCell audit
- Unbounded queue fix (PR-0c)

---

### PR-0c — Bound accept / queue (OOM shed)

**Goal:** Stop unbounded in-flight growth independent of concurrent handlers.  
**Priority:** Do if production site is public; can ship after 0b but should not wait for Phase 1.

#### TODOs

- [ ] Locate transport → interpreter request queue (today: unbounded)
- [ ] Introduce a hard bound (config default TBD; align with Phase 1 semaphore thinking, e.g. 256)
- [ ] When full: shed with clear failure (503 when response path exists; or refuse accept / drop with log)
- [ ] Structured log when shedding
- [ ] Document the limit in web-server / config docs

#### Acceptance tests

- [ ] Load/stress test cannot grow memory without bound via queued requests alone
- [ ] Over-cap path is deterministic and tested
- [ ] Existing happy-path server TestPrograms still pass

#### Out of scope

- Concurrent handler execution
- Per-request timeout semantics (Phase 1)

---

## Phase 1 — Cooperative concurrent request loop

**Goal:** Opt-in concurrent web handlers on one thread; isolation-by-default; ops defaults.  
**Depends on:** PR-0a (panic gate), PR-0b recommended first  
**Out of scope:** Nursery join engine, `change shared`, multicore, Rc→Arc

**Cost note:** Spike may be hours; shippable Phase 1 is days with the full test matrix.

### PR-1a — Runtime spike (feature-flag or throwaway-friendly)

**Goal:** Prove the bridge before committing surface syntax.

#### TODOs

- [ ] Multi-thread acceptor remains as today
- [ ] Bounded mpsc into a **current-thread** runtime
- [ ] Drive handlers with `FuturesUnordered` of `&self`-borrowing / `!Send` / non-`'static` futures
- [ ] Only plain data crosses the channel (compiler enforces: no `Rc`/`RefCell` across threads)
- [ ] `select!` between accept/recv and `futs.next()`
- [ ] **Guard empty `FuturesUnordered`** — empty set must not busy-spin (`Ready(None)` trap)
- [ ] Handler boundary: `catch_unwind(AssertUnwindSafe(...))` → one handler fails, siblings survive
- [ ] Confirm under `panic = "unwind"` only
- [ ] Deliberately trigger and document:
  - [ ] Empty-set busy-spin (prove guard fixes it)
  - [ ] Panic in one future (prove containment)
  - [ ] Borrow-across-await panic (characterize; full fix may be 1b audit + later shared-state closure)
- [ ] Do **not** refactor into `spawn_local` (needs `'static`)
- [ ] Spike report: what works, residual hazards, measured costs

#### Acceptance tests

- [ ] Empty handler set: no 100% CPU spin for N seconds
- [ ] Panic in handler A → contained; handler B completes
- [ ] Two concurrent slow awaits interleave (both complete without full serial wall-clock sum, within tolerance)
- [ ] Build still uses `panic = "unwind"`

#### Out of scope

- Final syntax / parser changes (optional minimal flag OK)
- Nursery, shared state, docs polish

---

### PR-1b — Surface: `main loop concurrently:` + ops defaults

**Goal:** User-visible opt-in concurrent loop; serial path untouched.

#### TODOs — language / runtime

- [ ] Parser: `main loop concurrently:` (and matching `end`)
- [ ] Analyzer / typechecker / keyword docs if needed
- [ ] Runtime: concurrent path uses proven 1a bridge
- [ ] **G1:** plain `main loop` remains serial and byte-compatible
- [ ] Isolated-per-request scopes (default)
- [ ] Semaphore / in-flight cap (default e.g. 256) → shed **503**
- [ ] Per-request timeout (default e.g. 30s) → **504** (only at await points — document cliff)
- [ ] Request-ID structured logging on accept / complete / fail / shed / timeout
- [ ] catch_unwind boundary → **500**, siblings survive
- [ ] Eval-core audit: every `RefCell` borrow/borrow_mut on await paths drops before `.await`
  - [ ] PR description lists each site and drop-before-await story
- [ ] clippy `await_holding_refcell_ref` enabled/enforced where applicable (backstop only)

#### TODOs — tests (write first where possible)

- [ ] Serial `main loop` still processes one request at a time (no silent upgrade)
- [ ] Concurrent loop: slow handler does not block fast sibling
- [ ] Cap exceeded → 503
- [ ] Timeout → 504
- [ ] Panic in A → 500; B still completes
- [ ] No empty-set busy-spin
- [ ] Request IDs present in logs (if testable)
- [ ] Existing web TestPrograms still pass on serial path

#### TODOs — docs (minimum for ship; full honesty pass may be 1c)

- [ ] Document `main loop concurrently:` syntax with validated example
- [ ] Keyword reference updates if new/repurposed keyword `concurrently`
- [ ] Note: cooperative; CPU-bound non-awaiting work still stalls the process

#### Out of scope

- `wait for all of` / `any of` / nursery blocks
- `change shared`
- Multicore workers

---

### PR-1c — Honesty docs for the real concurrent model

**Goal:** Docs match Phase 1 reality; no new overclaims.

#### TODOs

- [ ] Update `async-programming.md` and `web-servers.md` for concurrent loop
- [ ] Explicit sections:
  - [ ] Concurrent ≠ parallel (single-thread cooperative)
  - [ ] Yield cliff: timeouts cannot interrupt tight non-awaiting loops
  - [ ] Isolation-by-default; shared mutable state not in Phase 1
  - [ ] 503 / 504 / 500 semantics
- [ ] Validate all new examples (`TestPrograms/docs_examples/` + MCP / validate script)
- [ ] Dev Diary entry for Phase 1 ship

#### Acceptance

- [ ] No false "don't block others" without the cooperative caveat
- [ ] Examples validate
- [ ] Docs are part of the feature (already shipped surface in 1b; 1c finishes the story)

---

## Phase 2 — Structured concurrency + shared state (gated units)

**Depends on:** Phase 1 test matrix green  
**Rule:** Separate PRs; separate cost estimates; G4 applies

### PR-2a — Structured nursery + join engine

**Goal:** Net-new join/cancel engine. `Value::Future` today is eager completed-result — do not pretend it is a task handle.

#### TODOs

- [ ] Written cost estimate before coding (especially `any of` cancel-by-drop)
- [ ] Design freeze for vocabulary (do not invent new keywords mid-flight). Candidate surface from brainstorm:
  - [ ] `run these at the same time: ... end` (or final owner-approved phrasing)
  - [ ] `wait for all of ...`
  - [ ] `wait for any of ...` (expensive half — cancel losers)
  - [ ] Unstructured escape: `start <action> as <handle>` + `wait for <handle>`
- [ ] Pressure-test against reserved-keyword inventory and No-Unlearning
- [ ] Implement join engine as its own module; wire carefully into interpreter
- [ ] Cancel-by-drop for losers of `any of` — memory-safe given no borrow guard across await
- [ ] Error propagation / sibling cancel policy (document and test)
- [ ] Prefer growing `wait for` over a parallel mental model where possible
- [ ] Full docs + validated examples + Dev Diary

#### Acceptance tests

- [ ] `all of`: all children complete before parent continues
- [ ] `any of`: first wins; losers cancelled; no leak / no double-complete hazard
- [ ] Child failure cancels siblings and propagates (if structured policy says so)
- [ ] Unstructured `start`/`wait` does not outlive process carelessly (document lifetime rules)
- [ ] No regression on Phase 1 web concurrent loop

#### Out of scope

- Multicore
- Free-form shared mutable state (PR-2b)

---

### PR-2b — `change shared` critical region

**Goal:** Sound shared mutable state with await-in-critical-region **unrepresentable**.

#### TODOs

- [ ] Implement as **synchronous critical-section closure** (`with_borrow_mut`-style internally)
  - Inner `.await` is a **compile error** (Rust side) / unrepresentable in WFL surface
- [ ] WFL surface: `change shared: ... end change` (or owner-approved English)
- [ ] Must still read as natural English (validate ergonomics, not only soundness)
- [ ] Document **logical** hazards: copy-in/mutate/copy-out → stale snapshot / lost update (G5)
- [ ] Prefer isolation / external store for cross-request truth; shared state is the exception
- [ ] Optional later: tiny concurrent-safe primitives (atomic counter, map with defined conflict policy) — do not invent free-form races
- [ ] Compile-fail / negative tests for await inside critical region
- [ ] clippy backstop remains; catch_unwind remains last line
- [ ] Docs + examples + Dev Diary

#### Acceptance tests

- [ ] Await inside `change shared` cannot be written / fails validation
- [ ] Critical region serializes mutations without panicking under concurrent handlers
- [ ] Documented lost-update behavior has at least one illustrative test or documented example
- [ ] Phase 1 concurrent loop still healthy under contended shared updates

#### Out of scope

- Actors / message-passing redesign
- Multicore

---

## Phase 3 — Multicore only if profiling forces it

**Depends on:** Real load data showing single-thread cooperative + spawn_blocking insufficient  
**Default:** Deferred. Shared-nothing multi-process only. **Rc→Arc rewrite is last resort, not a plan.**

### PR-3a — Multi-process workers (decision required first)

#### Pre-decision TODOs (owner / research — not agent free-fire)

- [ ] Profile: is the bottleneck CPU interpreter, lock/contention, or I/O wait?
- [ ] Choose load strategy:
  - [ ] `SO_REUSEPORT` (idiomatic Rust; watch connection-count skew for variable-cost interpreters)
  - [ ] Master-accept + fd-passing (load-aware; single point of failure; unidiomatic)
- [ ] Design **connection drain on restart** (naive close drops queued connections)
- [ ] Keep Phase 1 surface **worker-agnostic** (no single-process assumptions in syntax)

#### Implementation TODOs (only after decision)

- [ ] Worker process model + lifecycle
- [ ] Drain story implemented and tested
- [ ] Load-skew mitigations or documented operational limits
- [ ] Ops docs: how to run N workers, health, restart
- [ ] G5: load-skew and drain documented as known obligations

#### Explicit non-goals unless everything else fails

- [ ] Full interpreter `Rc→Arc` + `Send` rewrite
- [ ] Actors/message-passing as default model (reconsider only with new design doc)

---

## Tracked unknowns (do not silently assume)

| ID | Unknown | Action |
|----|---------|--------|
| U1 | Generalized `await_holding_borrow` clippy lint (rust-clippy #13328) status | Re-check before relying on it; treat current lint as best-effort backstop |
| U2 | Multicore load balancing: `SO_REUSEPORT` vs master-accept | Owner decision before Phase 3 code |
| U3 | Exact default semaphore size / timeout seconds | Pick defaults in PR-1b; make configurable if config system already supports |
| U4 | Phase 0 responsiveness test shape before Phase 1 exists | Design test around actual await concurrency available post-0b |
| U5 | Final nursery English phrasing | Freeze in writing before PR-2a coding |

---

## Suggested execution order (AI workers)

```text
0a  →  0b  →  0c (if public site)
         ↓
        1a (spike report)
         ↓
        1b (surface + ops)  →  1c (docs)
         ↓
        STOP for maintainer review + cost 2a/2b
         ↓
        2a  then  2b  (separate PRs)
         ↓
        Phase 3 only if profiling forces it
```

### Agent prompt template

```text
You are implementing WFL concurrency PR-<id>: <title>.

Read: Docs/development/concurrency-phase-plan.md
      (optional design rationale: Starnet brainstorms/2026-07-11-wfl-concurrency-model.md)

Scope: ONLY the TODOs under PR-<id>.
Out of scope: everything listed under that PR's Out of scope + later phases.

HARD RULES: (paste Non-negotiable rules block)

TDD: write failing tests first for each acceptance bullet.
Stop and report if you need Send/Arc across the interpreter core.
When done: summarize files changed, tests added, and any residual risk.
Update the progress tracker status for PR-<id> in this plan.
```

---

## Definition of done (whole program)

- [ ] Phase 0: live crypto DoS mitigated; docs not lying about serial handlers; panic=unwind gated
- [ ] Phase 1: `main loop concurrently:` works; serial unchanged; 503/504/500; no busy-spin; sibling survival
- [ ] Phase 2 (if pursued): nursery costed and shipped; shared critical region unrepresentable-await; lost-update documented
- [ ] Phase 3: only with profiles + drain story; no surprise Arc rewrite
- [ ] All user-facing behavior changes include validated docs
- [ ] Dev Diary entries for non-trivial phases

---

## Related docs

- [Architecture overview](architecture-overview.md)
- [Async programming (user)](../04-advanced-features/async-programming.md)
- [Web servers (user)](../04-advanced-features/web-servers.md)
- [WFL foundation](../wfl-foundation.md)

---

*Derived from 4-expert brainstorm (2026-07-11) + AI-execution constraints. Marker locked to `main loop concurrently:` unless owner overrides before PR-1b.*
