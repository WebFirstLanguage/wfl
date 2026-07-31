# Dev Diary — 2026-07-13: Issue #610 Phase 1 — Open-Issue Inventory & Classification

## Context

Issue #610 ("WFL Production Readiness: 8/10 by January 1, 2027") opens Phase 1
("Baseline and containment", target July 12 – August 15, 2026) with:

> - [ ] Inventory all open issues and classify them as critical, high, medium,
>   low, or post-production-readiness

This entry records that inventory. Every "closed as fixed" decision below was
verified by **running the issue's own minimal reproduction against a fresh
release build** — `target/release/wfl`, **WFL 26.7.36**, commit `fc21f2f`,
Linux — not inferred from commit messages. This upholds the project's
"validate, don't assert" policy: an issue is only closed when its repro
demonstrably behaves correctly today.

## Method

1. Enumerated all open issues via the GitHub API (17 open, including #610 itself).
2. Cross-referenced each against merged fix PRs in `git log`.
3. Rebuilt the release binary and re-ran each candidate issue's minimal repro.
4. Closed only issues whose repro passed; left the rest open with a severity class.

## Inventory result

**Total open on GitHub at start:** 17 (16 tracked issues + the #610 tracker itself).

**Reconciliation of the 16 tracked issues** — stated explicitly because "open"
was previously conflated between two senses (*open on GitHub* vs. *genuinely
unresolved*):

| Bucket | Count | Issues |
|---|---|---|
| Verified-fixed **and** closed on GitHub | 10 | the *Closed* table below |
| Verified-fixed but still **open on GitHub**, pending a close click | 1 | #573 |
| **Genuinely unresolved** | 5 | #592, #578, #555, #600, #612 |

Arithmetic: `10 + 1 + 5 = 16` tracked `+ #610 = 17`, matching the start count. So
**6** tracked issues are still *open on GitHub* (the 5 unresolved **plus** #573),
but only **5** are *genuinely unresolved*. #573 is verified-fixed — PR #574
shipped binary read/write + MIME with byte-round-trip tests **before** this
inventory, and the issue is open on GitHub only pending a close click (it was
originally recorded here as an open "Medium" limitation in error). See the #573
row below.

### Closed — verified fixed against 26.7.36

| Issue | Title (short) | Class (at filing) | Fix PR | Verification |
|---|---|---|---|---|
| #557 | Date-unit locals fatal in included files | High (correctness / fatal) | #587 | prints `2,3`, no outer-scope fatal |
| #566 | `ends with` / `starts with` swallowed as identifiers | High (correctness) | #587 | `css` / `starts-ok` |
| #567 | `Any`/`Unknown` rejected by strict type rules | Medium (false diagnostic) | #587 | `20`, no type ERROR |
| #569 | Action return type inferred as `Nothing` | Medium (false diagnostic) | #575 | `hel`, no "found Nothing" |
| #571 | Precedence / `/` / `finally` / `between` etc. | High (correctness) | #577 | precedence, `/`, `between` all correct |
| #580 | `of`-form include-exposed action fatal | High (correctness / fatal) | #581 | `HI-bob` at top level |
| #582 | Parameter overridden by same-named global | **Critical** (silent wrong result) | #587 | `got: arg` |
| #583 | String `"[]"` coerced to empty list | Medium (silent wrong type) | #587 | `Text` / `Text` |
| #588 | `store x as …` ERROR when callee type Unknown | Medium (false diagnostic) | #589 | `4`, no "Could not infer" |
| #590 | Self-recursive result typed `Nothing` | Medium (false diagnostic) | #591 | `0`, no "Cannot index into Nothing" |

### Remaining open — classified

| Issue | Title (short) | Class | Rationale / evidence |
|---|---|---|---|
| #592 | Zero-arg include-exposed action by bare name is fatal | **High** | Fatal (`exit 3`, `Variable 'greet' is not defined`) on valid natural multi-file API; the third call form #580/#581's fix did not cover. Repro still fails on 26.7.36. |
| #578 | Remaining #571 rough edges (glob, pattern-VM, text→number, inference) | **High** | Confirmed functional bugs (wrong result/crash, not doc drift). Verified `list files … with pattern "*.txt"` still returns `0` on 26.7.36. |
| #555 | Aspirational skipped tests + broken keyword_reference docs examples | **Medium** | Core websockets landed (#593), but session/CSRF/cookie middleware, direct-index syntax, and 10 docs examples remain; 3 `CI-SKIP` TestPrograms still present. Docs-examples-in-CI is a mandatory release gate. Feature parts are effectively post-production. |
| ~~#573~~ | Web server cannot serve binary content (fonts, images) | **Fixed (correction)** | **Reclassified: this was recorded open in error.** PR #574 shipped binary read (`read binary from …`), binary write, lossless byte round-trip, and MIME helpers *before* this inventory, guarded by `web_server_binary_test.rs`, `binary_io_test.rs`, and `binary_file_and_mime_test.wfl`. The issue's own latest verification (2026-07-06) recommends closing; it is open on GitHub only pending a close click. |
| #600 | Native TLS ergonomics: SNI / multiple certificates on one `:443` | **Post-production-readiness** (SNI) · Dependabot alert #49 = *vulnerable code not used* | **Correction (source-level re-review).** An earlier revision reclassified this **High (security)**, treating the *presence* of `rustls-webpki` (alert #49, [GHSA-82j2-j2ch-gfr8](https://github.com/advisories/GHSA-82j2-j2ch-gfr8)) in the dependency graph as WFL exploitability. That overreached. The advisory's panic requires opt-in `RevocationOptions` **and** attacker-controlled CRL bytes; default rustls configs are unaffected. WFL's only TLS setup is `warp::serve(routes).tls().cert_path(…).key_path(…)` (`src/interpreter/mod.rs:6441`); warp 0.3.7 defaults client auth to `TlsClientAuth::Off` / `with_no_client_auth()`, and WFL configures **no** CRL / `RevocationOptions` anywhere (verified by grep) — so the vulnerable path is **not reachable**. Disposition: record/dismiss alert #49 as *"vulnerable code not used."* #600 itself is the **SNI / multi-cert enhancement** (post-production); it is **not** a reachable High WFL security defect, and its TLS rewrite is not established as required remediation on this evidence. The literal *no-open-High-severity-security* policy gate may remain administratively open until #49 is formally triaged. SNI priority is tracked on #600 independently. |
| #612 | Make PR #609 resource-budget policies overrideable via `.wflcfg` | **Low** | Explicitly filed "low priority"; safe conservative defaults already ship. Config-surface polish. |

### Severity legend (aligned to #610's gates)

- **Critical** — blocks a mandatory release gate: critical correctness/security,
  data loss, or uncontrolled resource exhaustion.
- **High** — correctness defect on valid/supported programs (incl. fatal
  false-positives), a confirmed functional bug, **or a *reachable* high-severity
  security advisory** — the vulnerable code path must actually be exercised by
  WFL's usage; the mere *presence* of a vulnerable dependency does **not** qualify
  (see the #600 row); must be fixed before RC.
- **Medium** — false diagnostics that don't change runtime results, or real but
  non-blocking feature limitations touching a gate.
- **Low** — polish / configurability with safe current defaults.
- **Post-production-readiness** — genuine enhancement, no release-gate impact.

## Phase 1 exit-gate read

The exit gate for Phase 1 is *"No known production-readiness risk is untracked."*
After this pass **no open issue is Critical**, and the two open High-severity items
are both **correctness** defects (#592, #578), tracked with reproductions.
Separately, Dependabot **alert #49** (`rustls-webpki`, high severity) is present in
the dependency graph but its vulnerable code path is **not reachable** in WFL's
usage (see the #600 row) — disposition *"vulnerable code not used."* It is therefore
**not** classified as a reachable High WFL defect, and #600's TLS rewrite is not
established as its required remediation; the literal *no-open-high-severity-security*
policy gate may remain administratively open until the alert is formally
triaged/dismissed on the Security tab. The remaining Phase 1 tasks (record scorecard
baseline, integrate the shared ExecutionBudget — note #609 already merged —
regression tests, fuzz targets, baseline metrics, supported-platform definition) are
separate checkboxes and out of scope for this inventory entry.

## Compatibility / resource impact

None. This change is documentation + issue-tracker hygiene only; no code,
runtime, or language behavior was modified.
