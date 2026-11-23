> [!IMPORTANT]
> **This repository has moved** to [gitlab.wfl.fyi/wfl/wfl](https://gitlab.wfl.fyi/wfl/wfl).
> Please open a merge request into `dev` there instead of a pull request here.
> You need a gitlab.wfl.fyi account; self-service sign-up is currently off, so
> email **info@logbie.com** to request one.

# Summary

Describe the final change and owning issue.

## Compatibility and risk

Risk class, affected contracts, exceptions and unresolved controls.

## Validation

Current SHA, exact commands/results, Actions links, Red/Green or justified
documentation-only N/A. Record missing/skipped checks as Not run with reason.

## Checklist

- [ ] Feature/fix/docs branch targets `dev`; no direct protected-branch push.
- [ ] Current SHA, Actions run links and individual required results are recorded.
- [ ] Red/Green evidence, or justified documentation-only N/A with doc/link checks.
- [ ] Skipped, missing, pending and failed checks are explicit, never called passes.
- [ ] Required independent reviews cover this revision; material fixes have fresh CI and review.
- [ ] Triggered bot reviews finished; findings/discussions are fixed or dispositioned.
- [ ] PR owner has a real monitor/event continuation while checks or reviews are pending.
- [ ] No secrets/private data; environment injection and production boundaries observed.
- [ ] No protection bypass; check/review state is rechecked immediately before merge.
- [ ] Main/release/tag/deploy authority and Brad-reserved decisions follow GOVERNANCE.md.
