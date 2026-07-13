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

**Total open at start:** 17 (16 tracked issues + the #610 tracker itself).
**Closed as verified-fixed:** 10. **Remaining open after triage:** ~~6~~ **5** tracked + #610 (this originally read "6"; corrected to 5 after the #573 reclassification below).

> **Correction (post-review):** #573 in the table below was recorded as an open
> "Medium" limitation **in error** — PR #574 had already shipped binary
> serving + MIME (with byte-round-trip tests) before this inventory. Counting
> that correction, only **5** tracked issues remain genuinely open (+#610). See
> the #573 row for details.

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
| #600 | Native TLS: SNI / multiple certificates on one `:443` | **Post-production-readiness** | Single-cert HTTPS works; multi-cert/SNI is a multi-tenant deployment enhancement, not a release-gate blocker. |
| #612 | Make PR #609 resource-budget policies overrideable via `.wflcfg` | **Low** | Explicitly filed "low priority"; safe conservative defaults already ship. Config-surface polish. |

### Severity legend (aligned to #610's gates)

- **Critical** — blocks a mandatory release gate: critical correctness/security,
  data loss, or uncontrolled resource exhaustion.
- **High** — correctness defect on valid/supported programs (incl. fatal
  false-positives) or a confirmed functional bug; must be fixed before RC.
- **Medium** — false diagnostics that don't change runtime results, or real but
  non-blocking feature limitations touching a gate.
- **Low** — polish / configurability with safe current defaults.
- **Post-production-readiness** — genuine enhancement, no release-gate impact.

## Phase 1 exit-gate read

The exit gate for Phase 1 is *"No known production-readiness risk is untracked."*
After this pass **no open issue is Critical**, and the two open High-severity
correctness items (#592, #578) are tracked with reproductions. The remaining
Phase 1 tasks (record scorecard baseline, integrate the shared ExecutionBudget —
note #609 already merged — regression tests, fuzz targets, baseline metrics,
supported-platform definition) are separate checkboxes and out of scope for this
inventory entry.

## Compatibility / resource impact

None. This change is documentation + issue-tracker hygiene only; no code,
runtime, or language behavior was modified.
