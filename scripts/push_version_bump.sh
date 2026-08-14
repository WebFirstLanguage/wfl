#!/usr/bin/env bash
# Push the post-merge version-bump commit to a branch, re-bumping on rejection.
#
# Extracted from the `bump-version` job in .github/workflows/ci.yml so the retry
# behaviour can be tested (inline workflow shell cannot be).
#
# Usage: scripts/push_version_bump.sh <branch>
#
# The two commands this shells out to are overridable so the tests can record
# and control them; the defaults are what CI runs:
#   BUMP_CMD     - re-derives the next version from the fresh tip and commits it
#   HYGIENE_CMD  - proves the tree about to be pushed is hygienic
set -eu

BRANCH="${1:-${GITHUB_REF_NAME:-}}"
[ -n "$BRANCH" ] || { echo "usage: $0 <branch>" >&2; exit 64; }

BUMP_CMD="${BUMP_CMD:-python scripts/bump_version.py --update-all}"
HYGIENE_CMD="${HYGIENE_CMD:-python scripts/check_repo_hygiene.py --mode static}"

# The bump commit must fast-forward main. When another commit lands on main
# during this run (concurrent merges), the first push is rejected as
# non-fast-forward. Re-base the bump onto the new tip and retry. Because two
# concurrent runs can compute the same next version, we reset to the fresh tip
# and re-run the bump so the version is derived from the true current version
# instead of replaying a stale bump.
for attempt in 1 2 3 4 5; do
  if git push origin HEAD:"$BRANCH"; then
    echo "Pushed version bump on attempt $attempt"
    exit 0
  fi
  echo "Push rejected on attempt $attempt (main advanced); re-bumping from latest tip..."
  git fetch origin "$BRANCH"
  git reset --hard "origin/$BRANCH"
  # shellcheck disable=SC2086  # intentional word splitting: command + args
  $BUMP_CMD
  # The re-bump produced a new commit on a new tree - whatever landed on
  # `$BRANCH` concurrently, plus a fresh bump. The check the job ran before the
  # first attempt says nothing about that tree, so hygiene and version agreement
  # must be re-proven here, immediately before this push
  # (REPOSITORY_HYGIENE.md §8).
  # shellcheck disable=SC2086  # intentional word splitting: command + args
  if ! $HYGIENE_CMD; then
    # Abort rather than retry: retrying past a violation would defeat the gate.
    # This leaves the branch un-bumped, which the next push to it recovers.
    echo "::error::Static hygiene check failed on the re-bumped tree (attempt $attempt); refusing to push."
    exit 1
  fi
done
echo "::error::Failed to push version bump after 5 attempts (main kept advancing)."
exit 1
