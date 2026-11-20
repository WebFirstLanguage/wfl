# Summary

Describe the final change and owning issue.

## PRD

`Implements PRD-NNNN: REQ-01, REQ-02 (AC-01–AC-03)`, or
`PRD: not needed — <reason>` (bug fix, refactor, docs, tests, maintenance).
See Docs/contributing/prd-process.md.

## Compatibility and risk

Risk class, affected contracts, exceptions and unresolved controls.

## Validation

Current SHA, exact commands/results, Actions links, Red/Green or justified
documentation-only N/A. Record missing/skipped checks as Not run with reason.

## Checklist

- [ ] Feature/fix/docs branch targets `dev`; no direct protected-branch push.
- [ ] A feature names an Approved PRD and its REQ/AC IDs, stays inside it, and does not edit the PRD file.
- [ ] Every behavior change has Red/Green evidence and an end-to-end test on the real shipped artifact (testing.md TEST-01).
- [ ] Current SHA, Actions run links and individual required results are recorded.
- [ ] Red/Green evidence, or justified documentation-only N/A with doc/link checks.
- [ ] Skipped, missing, pending and failed checks are explicit, never called passes.
- [ ] Required independent reviews cover this revision; material fixes have fresh CI and review.
- [ ] Triggered bot reviews finished; findings/discussions are fixed or dispositioned.
- [ ] PR owner has a real monitor/event continuation while checks or reviews are pending.
- [ ] No secrets/private data; environment injection and production boundaries observed.
- [ ] No protection bypass; check/review state is rechecked immediately before merge.
- [ ] Main/release/tag/deploy authority and Brad-reserved decisions follow GOVERNANCE.md.
