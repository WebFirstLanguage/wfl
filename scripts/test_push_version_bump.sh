#!/usr/bin/env bash
# Tests for scripts/push_version_bump.sh.
#
# What is real and what is not
# ----------------------------
# The script under test is release-control machinery: it is the last thing that
# runs before a commit lands on `main`, and the behaviour these tests verify is
# *what it does between a rejected push and the next one* - whether it re-bumps,
# whether it re-proves hygiene on the tree it is about to push, and whether a
# violation stops it. That is verified against a real git repository and a real
# bare remote in a temp dir: `git push` to a local bare repo is a genuine push,
# the rejections are genuine non-fast-forward rejections produced by actually
# advancing the remote from a second clone, and "nothing was pushed" is asserted
# by reading the remote's tip.
#
# The two boundaries that are stubbed are the commands the script shells out to
# by name - the version bumper and the hygiene checker - because the real ones
# rewrite the working repo's version files and shell out to Cargo. Both stubs
# are recording fakes: every invocation appends its name *and the HEAD it saw*
# to a log, so a test can assert on the order and count of the calls and on the
# exact tree each one inspected. That is what makes "hygiene ran on the
# re-bumped tree, before the push" an assertion rather than an assumption.
#
# Nothing about git is faked. The failure path asserts an absence - the remote
# tip is byte-identical to what it was before the run - because "exited
# non-zero" alone would not prove the bad tree stayed off `main`.
#
# Usage: scripts/test_push_version_bump.sh

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PUSH="$REPO_ROOT/scripts/push_version_bump.sh"

PASS=0
FAIL=0

ok()   { PASS=$((PASS + 1)); echo "  ok   $1"; }
bad()  { FAIL=$((FAIL + 1)); echo "  FAIL $1"; }

assert_eq() { # assert_eq <expected> <actual> <message>
  if [ "$1" = "$2" ]; then ok "$3"; else
    bad "$3"
    echo "         expected: $1"
    echo "         actual:   $2"
  fi
}

assert_contains() { # assert_contains <haystack> <needle> <message>
  case "$1" in
    *"$2"*) ok "$3" ;;
    *) bad "$3"; echo "         expected to contain: $2"; echo "         actual: $1" ;;
  esac
}

assert_absent() { # assert_absent <haystack> <needle> <message>
  case "$1" in
    *"$2"*) bad "$3"; echo "         expected NOT to contain: $2"; echo "         actual: $1" ;;
    *) ok "$3" ;;
  esac
}

assert_nonzero() { # assert_nonzero <rc> <message>
  if [ "$1" -ne 0 ]; then ok "$2"; else bad "$2"; echo "         expected a non-zero exit, got 0"; fi
}

# ---------------------------------------------------------------------------
# Sandbox: a real bare remote, the "CI checkout" that pushes to it, and a second
# clone standing in for whatever merges into main while the bump job is running.
#
# $SB/calls records every stub invocation as "<name>\t<HEAD it saw>".
# ---------------------------------------------------------------------------
make_stubs() { # make_stubs <sandbox>
  local d="$1"
  mkdir -p "$d/bin"

  # Stands in for `python scripts/bump_version.py --update-all`: bumps a version
  # file and commits it, exactly as the real one does (it commits itself - the
  # workflow pushes HEAD, not a staged tree).
  cat > "$d/bin/bump" <<'STUB_BUMP'
#!/usr/bin/env bash
set -eu
printf 'bump\t%s\n' "$(git rev-parse HEAD)" >> "$CALL_LOG"
n="$(cat VERSION)"
echo "$((n + 1))" > VERSION
git add VERSION
git commit -q -m "Bump version to $((n + 1)) [skip ci]"
# Lets a test keep main moving under the job, so every attempt is rejected.
if [ -n "${ADVANCE_REMOTE_ON_BUMP:-}" ]; then "$SANDBOX/bin/advance-remote"; fi
exit 0
STUB_BUMP

  # Stands in for `python scripts/check_repo_hygiene.py --mode static`.
  cat > "$d/bin/hygiene" <<'STUB_HYGIENE'
#!/usr/bin/env bash
set -eu
printf 'hygiene\t%s\n' "$(git rev-parse HEAD)" >> "$CALL_LOG"
exit "${HYGIENE_EXIT:-0}"
STUB_HYGIENE

  # A concurrent merge landing on main from somewhere else.
  cat > "$d/bin/advance-remote" <<'STUB_ADVANCE'
#!/usr/bin/env bash
set -eu
git -C "$SANDBOX/other" pull -q --ff-only origin main
git -C "$SANDBOX/other" commit -q --allow-empty -m "concurrent merge"
git -C "$SANDBOX/other" push -q origin main
STUB_ADVANCE

  chmod +x "$d/bin/bump" "$d/bin/hygiene" "$d/bin/advance-remote"
}

new_env() { # new_env -> echoes a fresh sandbox dir
  local d
  d="$(mktemp -d)"
  git init -q --bare -b main "$d/remote.git"
  git init -q -b main "$d/work"
  git -C "$d/work" config user.email "ci@example.invalid"
  git -C "$d/work" config user.name "CI"
  git -C "$d/work" remote add origin "$d/remote.git"
  echo 0 > "$d/work/VERSION"
  git -C "$d/work" add VERSION
  git -C "$d/work" commit -q -m "seed"
  git -C "$d/work" push -q origin main
  git clone -q "$d/remote.git" "$d/other"
  git -C "$d/other" config user.email "other@example.invalid"
  git -C "$d/other" config user.name "Other"
  : > "$d/calls"
  make_stubs "$d"
  echo "$d"
}

# The bump the workflow's own "Bump version" step already made before the push
# step runs. Committed with plain git so it does not show up in the call log.
seed_bump() { # seed_bump <sandbox>
  local d="$1"
  echo 1 > "$d/work/VERSION"
  git -C "$d/work" add VERSION
  git -C "$d/work" commit -q -m "Bump version to 1 [skip ci]"
}

advance_remote() { # advance_remote <sandbox>
  SANDBOX="$1" "$1/bin/advance-remote"
}

run_push() { # run_push <sandbox> -> writes $sandbox/out, returns exit code
  local d="$1"
  (
    cd "$d/work" || exit 1
    export SANDBOX="$d" CALL_LOG="$d/calls"
    export BUMP_CMD="$d/bin/bump" HYGIENE_CMD="$d/bin/hygiene"
    "$PUSH" main
  ) > "$d/out" 2>&1
}

remote_tip() { git -C "$1/remote.git" rev-parse main; }
local_head() { git -C "$1/work" rev-parse HEAD; }
call_order() { cut -f1 "$1/calls" | tr '\n' ' ' | sed 's/ $//'; }
call_count() { awk -F'\t' -v n="$2" '$1 == n { c++ } END { print c + 0 }' "$1/calls"; }
call_head()  { awk -F'\t' -v n="$2" '$1 == n { print $2 }' "$1/calls"; }

# ---------------------------------------------------------------------------
# Happy path: nothing landed on main, so there is no retry and nothing to
# re-check. The one hygiene check the workflow already ran before this script
# still covers the tree that gets pushed.
# ---------------------------------------------------------------------------
echo "push_version_bump.sh: first push succeeds"

SB="$(new_env)"
seed_bump "$SB"
run_push "$SB"
rc=$?
assert_eq "0" "$rc" "push succeeds on the first attempt ($SB/out)"
assert_contains "$(cat "$SB/out")" "Pushed version bump on attempt 1" \
  "the script reports the attempt it succeeded on"
assert_eq "$(local_head "$SB")" "$(remote_tip "$SB")" "the remote advances to the bump commit"
assert_eq "" "$(call_order "$SB")" "no re-bump and no re-check when there is no retry"
rm -rf "$SB"

# ---------------------------------------------------------------------------
# Retry path: a concurrent merge lands on main, the first push is rejected, and
# the script re-bumps onto the new tip. That produces a NEW commit on a NEW tree
# - whatever landed on main concurrently, plus a fresh bump - and the hygiene
# check the job ran before the first attempt says nothing about it. It must be
# re-proven before this tree is pushed (REPOSITORY_HYGIENE.md §8).
# ---------------------------------------------------------------------------
echo "push_version_bump.sh: a retry re-proves hygiene on the re-bumped tree"

SB="$(new_env)"
seed_bump "$SB"
advance_remote "$SB"
run_push "$SB"
rc=$?
assert_eq "0" "$rc" "push succeeds on the retry ($SB/out)"
assert_contains "$(cat "$SB/out")" "Push rejected on attempt 1" \
  "the first attempt was genuinely rejected"
assert_eq "bump hygiene" "$(call_order "$SB")" \
  "the retry re-bumps and then re-checks hygiene"
assert_eq "1" "$(call_count "$SB" hygiene)" "hygiene is checked once per retry"
assert_eq "$(local_head "$SB")" "$(remote_tip "$SB")" "the remote advances to the re-bumped commit"
# The load-bearing assertion: the tree hygiene inspected is the tree that was
# pushed, not the pre-retry one.
assert_eq "$(remote_tip "$SB")" "$(call_head "$SB" hygiene)" \
  "hygiene inspected the exact commit that was pushed"
rm -rf "$SB"

# ---------------------------------------------------------------------------
# Failure path (R3): the concurrent merge carried a hygiene violation, so the
# re-bumped tree fails the check. Aborting leaves main un-bumped, which the next
# push to main recovers; retrying past the violation would defeat the gate.
# ---------------------------------------------------------------------------
echo "push_version_bump.sh: a hygiene violation on a retry aborts instead of pushing"

SB="$(new_env)"
seed_bump "$SB"
advance_remote "$SB"
before="$(remote_tip "$SB")"
(
  export HYGIENE_EXIT=1
  run_push "$SB"
)
rc=$?
assert_nonzero "$rc" "the script fails when hygiene fails on a retry"
assert_eq "$before" "$(remote_tip "$SB")" \
  "nothing was pushed - the remote tip is exactly where it was"
assert_eq "1" "$(call_count "$SB" hygiene)" "hygiene ran on the re-bumped tree"
assert_absent "$(cat "$SB/out")" "Push rejected on attempt 2" \
  "it aborts rather than retrying past the violation"
assert_contains "$(cat "$SB/out")" "hygiene" "the failure names what it refused to do"
rm -rf "$SB"

# ---------------------------------------------------------------------------
# Exhaustion: main keeps moving under the job. Unchanged behaviour - five
# attempts, then a hard failure with the message CI greps for.
# ---------------------------------------------------------------------------
echo "push_version_bump.sh: five consecutive rejections give up"

SB="$(new_env)"
seed_bump "$SB"
advance_remote "$SB"
(
  export ADVANCE_REMOTE_ON_BUMP=1
  run_push "$SB"
)
rc=$?
assert_nonzero "$rc" "the script fails after five rejections"
assert_contains "$(cat "$SB/out")" \
  "::error::Failed to push version bump after 5 attempts (main kept advancing)." \
  "it emits the workflow error annotation"
assert_contains "$(cat "$SB/out")" "Push rejected on attempt 5" "all five attempts were made"
assert_eq "5" "$(call_count "$SB" bump)" "it re-bumped once per rejection"
rm -rf "$SB"

echo
echo "passed: $PASS   failed: $FAIL"
[ "$FAIL" -eq 0 ]
