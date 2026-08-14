# 2026-08-14 — `bump-version` pushed to `main` unchecked on any push retry (#678)

## Symptom

None observed in production — and that is the point. The gap needs a genuine
race to surface: another commit must land on `main` between the `bump-version`
job's checkout and its push. Rare enough to go unnoticed, and the place it
surfaces is the branch the check exists to protect.

Split out of the #674 review, where CodeRabbit raised it. Pre-existing; #674 did
not touch it.

## Root cause

`.github/workflows/ci.yml` ran the hygiene gate exactly once, as its own step,
with a comment stating the requirement plainly:

```yaml
# This job writes directly to main, so it must prove hygiene and version
# agreement itself, immediately before pushing (REPOSITORY_HYGIENE.md §8).
- name: Static hygiene and version check before push
  run: python scripts/check_repo_hygiene.py --mode static
```

The `Push changes` step then retried up to five times, and each retry rebuilt
the commit from scratch:

```bash
git fetch origin "$BRANCH"
git reset --hard "origin/$BRANCH"
python scripts/bump_version.py --update-all
```

That is a **new commit on a new tree** — the concurrent merge's content plus a
fresh bump — and it went out without the gate running again. The step's own
comment was true only of the first attempt. A concurrent merge carrying a
hygiene violation, or a bump that drifted against the new tree, reached `main`
unchecked and was caught only by the *next* run, after it had landed.

## The fix

The retry loop moved to `scripts/push_version_bump.sh`, which re-runs the
hygiene check on the re-bumped tree before each subsequent push. A violation
**aborts**:

```bash
if ! $HYGIENE_CMD; then
  echo "::error::Static hygiene check failed on the re-bumped tree (attempt $attempt); refusing to push."
  exit 1
fi
```

Aborting, not skipping to the next attempt — retrying past a violation would
defeat the gate. Aborting leaves `main` un-bumped, which the next push to `main`
recovers. The non-retry path is unchanged: still exactly one check, in the
workflow step, before the first attempt.

## Why it moved out of the YAML

Inline workflow shell cannot be tested — it only ever executes in production, on
pushes to `main`, on the rare retry path. That is the same hazard
`scripts/test_publish_spaces.sh` exists to close for the release scripts, and it
gets the same treatment here.

`scripts/test_push_version_bump.sh` runs the real script against a **real git
repository and a real bare remote**, with genuine non-fast-forward rejections
produced by a second clone committing to the remote mid-flight. Only the two
shelled-out commands are stubbed — `BUMP_CMD` and `HYGIENE_CMD`, as recording
fakes that log the HEAD each one saw — because the production versions mutate
the working repo and run Cargo. `git push` is never stubbed; the assertions are
made against where the remote tip actually moved.

## Risk class

**R3** — release controls (root `testing.md` §5), which requires negative and
failure-path coverage. The failure-path case asserts the *absence* of the write:
when hygiene fails on a retry, the script exits non-zero **and** the remote tip
is byte-identical to where it started.

## Red evidence

Against the faithful extraction of the buggy loop, 7 of 19 assertions failed.
The one that matters:

```text
FAIL nothing was pushed - the remote tip is exactly where it was
       expected: 0e721debb32938a8cf5da424cc1b7bb75d3f0f02
       actual:   b169625d9ecf277cb4acd3d316750132847f2a32
```

The old loop pushed `b169625` — the re-bumped tree carrying the concurrent
merge — to the remote despite hygiene rejecting it. That is the defect, observed
rather than argued.

Red: `6bab170`. Green: `e60b8c9`. 19/19 after the fix.

## Coverage added

`Release Script Tests` now runs `scripts/test_push_version_bump.sh` alongside
`scripts/test_publish_spaces.sh`, so the retry path is exercised on every PR
rather than only during a real race on `main`.
