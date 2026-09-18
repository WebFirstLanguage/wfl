#!/usr/bin/env bash
# Tests for scripts/publish_spaces.sh and scripts/backfill_spaces_checksums.sh.
#
# What is real and what is not
# ----------------------------
# The scripts under test are pure orchestration: they decide *which keys* get
# written, *with which bytes*, and *with which cache headers*. That decision is
# the behaviour these tests verify, and it is verified end to end - the real
# scripts run, the real sha256sum runs, the real jq runs.
#
# The one boundary that is stubbed is the AWS CLI (and the curl calls that read
# back through the CDN), because the other side of it is a live DigitalOcean
# Spaces bucket that a test must not write to. The stub is a recording fake with
# a real object store behind it (a directory), so uploads are readable by later
# reads in the same test and assertions are made against bytes and headers that
# actually moved - not against "the script called aws".
#
# The genuine Spaces boundary is covered where it belongs: publish_spaces.sh
# fetches every object back through the CDN and compares SHA-256 against the
# local file on every real publish, and fails the release if it does not match.
#
# Usage: scripts/test_publish_spaces.sh

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PUBLISH="$REPO_ROOT/scripts/publish_spaces.sh"
BACKFILL="$REPO_ROOT/scripts/backfill_spaces_checksums.sh"

command -v jq >/dev/null || { echo "jq is required to run these tests"; exit 1; }

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

assert_file_exists() { # assert_file_exists <path> <message>
  if [ -f "$1" ]; then ok "$2"; else bad "$2"; echo "         missing: $1"; fi
}

assert_file_absent() { # assert_file_absent <path> <message>
  if [ ! -e "$1" ]; then ok "$2"; else bad "$2"; echo "         unexpectedly present: $1"; fi
}

# ---------------------------------------------------------------------------
# Fake bucket + fake `aws` / `curl`.
#
# The fake bucket is a directory: $FAKE_BUCKET/<key> holds the object bytes.
# Every mutation is appended to $FAKE_LOG as "<verb> <key> <content-type>
# <cache-control>", so a test can assert on headers as well as bytes.
# ---------------------------------------------------------------------------
make_fakes() { # make_fakes <bindir>
  local bin="$1"
  mkdir -p "$bin"

  cat > "$bin/aws" <<'FAKE_AWS'
#!/usr/bin/env bash
set -uo pipefail
sub="${1:-}"; shift || true

key_of() { # key_of s3://bucket/key
  local u="${1#s3://}"
  echo "${u#*/}"
}

case "$sub" in
  s3)
    op="${1:-}"; shift || true
    [ "$op" = "cp" ] || { echo "fake aws: unsupported: s3 $op" >&2; exit 64; }
    src=""; dst=""; ct=""; cc=""
    while [ $# -gt 0 ]; do
      case "$1" in
        --content-type)      ct="$2"; shift 2 ;;
        --cache-control)     cc="$2"; shift 2 ;;
        --endpoint-url|--acl) shift 2 ;;
        --only-show-errors|--quiet) shift ;;
        --*)                 shift ;;
        *) if [ -z "$src" ]; then src="$1"; else dst="$1"; fi; shift ;;
      esac
    done
    case "$dst" in
      s3://*)
        key="$(key_of "$dst")"
        if [ -n "${FAKE_FAIL_KEY_GLOB:-}" ]; then
          # shellcheck disable=SC2254
          case "$key" in
            $FAKE_FAIL_KEY_GLOB) echo "fake aws: injected upload failure for $key" >&2; exit 1 ;;
          esac
        fi
        mkdir -p "$FAKE_BUCKET/$(dirname "$key")"
        cp "$src" "$FAKE_BUCKET/$key" || exit 1
        printf 'PUT\t%s\t%s\t%s\n' "$key" "$ct" "$cc" >> "$FAKE_LOG"
        ;;
      *)
        key="$(key_of "$src")"
        [ -f "$FAKE_BUCKET/$key" ] || { echo "fake aws: no such key: $key" >&2; exit 1; }
        cp "$FAKE_BUCKET/$key" "$dst" || exit 1
        printf 'GET\t%s\t\t\n' "$key" >> "$FAKE_LOG"
        # Lets a test slip a concurrent publisher into the window between a
        # backfill's download and its upload.
        if [ -n "${FAKE_ON_GET_HOOK:-}" ]; then "$FAKE_ON_GET_HOOK" "$key"; fi
        ;;
    esac
    ;;
  s3api)
    op="${1:-}"; shift || true
    case "$op" in
      head-object)
        key=""
        while [ $# -gt 0 ]; do
          case "$1" in
            --key) key="$2"; shift 2 ;;
            --bucket|--endpoint-url|--output) shift 2 ;;
            --*) shift ;;
            *) shift ;;
          esac
        done
        if [ -f "$FAKE_BUCKET/$key" ]; then
          jq -n --arg k "$key" '{ContentLength: 1, Key: $k}'
          exit 0
        fi
        echo "fake aws: Not Found: $key" >&2
        exit 254
        ;;
      list-objects-v2) : ;;
      *) echo "fake aws: unsupported: s3api $op" >&2; exit 64 ;;
    esac
    prefix=""
    while [ $# -gt 0 ]; do
      case "$1" in
        --prefix) prefix="$2"; shift 2 ;;
        --bucket|--endpoint-url|--output) shift 2 ;;
        --*) shift ;;
        *) shift ;;
      esac
    done
    ( cd "$FAKE_BUCKET" 2>/dev/null && find . -type f | sed 's|^\./||' | sort ) \
      | grep -E "^${prefix}" \
      | jq -R -s '{Contents: (split("\n") | map(select(length > 0)) | map({Key: .})), IsTruncated: false}'
    ;;
  *)
    echo "fake aws: unsupported command: $sub" >&2
    exit 64
    ;;
esac
FAKE_AWS

  cat > "$bin/curl" <<'FAKE_CURL'
#!/usr/bin/env bash
# Serves the fake bucket over "HTTP": the CDN host maps onto $FAKE_BUCKET.
#
# FAKE_CURL_FLAKY_KEY / FAKE_CURL_FLAKY_TIMES simulate a CDN edge that has not
# yet propagated a freshly uploaded key: the first N requests for that key fail
# as if the object were not there yet, and the attempts are counted in
# $FAKE_BUCKET/../flaky.count so a test can assert the retry actually happened.
set -uo pipefail
out=""; wfmt=""; fail=0; url=""
while [ $# -gt 0 ]; do
  case "$1" in
    -o) out="$2"; shift 2 ;;
    -w) wfmt="$2"; shift 2 ;;
    --max-time) shift 2 ;;
    -fsS|-fLO|-f) fail=1; shift ;;
    -s|-sS|-sI|-I) shift ;;
    -*) shift ;;
    *) url="$1"; shift ;;
  esac
done
path="${url#*://}"; path="${path#*/}"
if [ -n "${FAKE_CURL_FLAKY_KEY:-}" ] && [ "$path" = "$FAKE_CURL_FLAKY_KEY" ]; then
  count_file="$(dirname "$FAKE_BUCKET")/flaky.count"
  seen=0
  [ -s "$count_file" ] && seen="$(cat "$count_file")"
  if [ "$seen" -lt "${FAKE_CURL_FLAKY_TIMES:-1}" ]; then
    echo "$((seen + 1))" > "$count_file"
    [ -n "$wfmt" ] && printf '404'
    [ "$fail" = "1" ] && exit 22
    exit 0
  fi
fi
if [ -f "$FAKE_BUCKET/$path" ]; then
  [ -n "$out" ] && cp "$FAKE_BUCKET/$path" "$out"
  [ -n "$wfmt" ] && printf '200'
  exit 0
fi
[ -n "$wfmt" ] && printf '404'
[ "$fail" = "1" ] && exit 22
exit 0
FAKE_CURL

  chmod +x "$bin/aws" "$bin/curl"
}

new_env() { # new_env -> echoes a fresh sandbox dir with bucket/log/bin/artifacts
  local d
  d="$(mktemp -d)"
  mkdir -p "$d/bucket" "$d/artifacts"
  : > "$d/log"
  make_fakes "$d/bin"
  echo "$d"
}

# put_object <sandbox> <key> <content>  -- seed pre-existing bucket state
put_object() {
  mkdir -p "$1/bucket/$(dirname "$2")"
  printf '%s' "$3" > "$1/bucket/$2"
}

log_line_for() { # log_line_for <sandbox> <key> -> last PUT log line for that key
  awk -F'\t' -v k="$2" '$1 == "PUT" && $2 == k { line = $0 } END { print line }' "$1/log"
}

put_count() { # put_count <sandbox> <key>
  awk -F'\t' -v k="$2" '$1 == "PUT" && $2 == k { n++ } END { print n + 0 }' "$1/log"
}

make_artifacts() { # make_artifacts <sandbox> <version> <sha>
  local d="$1" v="$2" s="$3"
  printf 'tarball bytes for %s\n' "$v" > "$d/artifacts/wfl-$v-linux-x86_64-$s.tar.gz"
  printf 'msi bytes for %s\n'     "$v" > "$d/artifacts/wfl-$v.msi"
  printf 'vsix bytes for %s\n'    "$v" > "$d/artifacts/vscode-wfl-$v.vsix"
}

run_publish() { # run_publish <sandbox> <version> <sha> -> writes $sandbox/out, returns exit code
  local d="$1"
  (
    export PATH="$d/bin:$PATH"
    export FAKE_BUCKET="$d/bucket" FAKE_LOG="$d/log"
    export AWS_ACCESS_KEY_ID=test AWS_SECRET_ACCESS_KEY=test
    "$PUBLISH" "$d/artifacts" "$2" "$3" "${3}deadbeef" main
  ) > "$d/out" 2>&1
}

run_backfill() { # run_backfill <sandbox> [extra args...] -> writes $sandbox/out
  local d="$1"; shift
  (
    export PATH="$d/bin:$PATH"
    export FAKE_BUCKET="$d/bucket" FAKE_LOG="$d/log"
    export AWS_ACCESS_KEY_ID=test AWS_SECRET_ACCESS_KEY=test
    "$BACKFILL" "$@"
  ) > "$d/out" 2>&1
}

sha_line() { # sha_line <file> -> "<hash>  <basename>"
  ( cd "$(dirname "$1")" && sha256sum "$(basename "$1")" )
}

# ---------------------------------------------------------------------------
# publish_spaces.sh
# ---------------------------------------------------------------------------
echo "publish_spaces.sh: per-artifact checksum sidecars"

SB="$(new_env)"
make_artifacts "$SB" "26.7.60" "abc1234"
run_publish "$SB" "26.7.60" "abc1234"
rc=$?
assert_eq "0" "$rc" "publish succeeds ($SB/out)"

TARBALL="wfl-26.7.60-linux-x86_64-abc1234.tar.gz"
MSI="wfl-26.7.60.msi"
VSIX="vscode-wfl-26.7.60.vsix"

for a in "$TARBALL" "$MSI" "$VSIX"; do
  assert_file_exists "$SB/bucket/releases/$a.sha256" "sidecar published for $a"
  if [ -f "$SB/bucket/releases/$a.sha256" ]; then
    want="$(sha_line "$SB/artifacts/$a")"
    got="$(cat "$SB/bucket/releases/$a.sha256")"
    assert_eq "$want" "$got" "sidecar for $a holds the sha256sum line for the artifact"
    assert_contains "$(log_line_for "$SB" "releases/$a.sha256")" "immutable" \
      "sidecar for $a is cached as immutable"
    assert_contains "$(log_line_for "$SB" "releases/$a.sha256")" "text/plain" \
      "sidecar for $a is served as text/plain"
  fi
done

# A sidecar for a rolling pointer would itself be rolling, which is exactly the
# false sense of pinnability this change exists to remove.
assert_file_absent "$SB/bucket/releases/wfl-latest-linux-x86_64.tar.gz.sha256" \
  "no sidecar for the rolling Linux pointer"
assert_file_absent "$SB/bucket/releases/wfl-latest-windows-x86_64.msi.sha256" \
  "no sidecar for the rolling Windows pointer"
assert_file_absent "$SB/bucket/releases/SHA256SUMS.sha256" \
  "no sidecar for SHA256SUMS itself"

# Backward compatibility: existing consumers read SHA256SUMS, and it must keep
# describing the current publish exactly as before.
assert_file_exists "$SB/bucket/releases/SHA256SUMS" "SHA256SUMS is still published"
sums="$(cat "$SB/bucket/releases/SHA256SUMS" 2>/dev/null)"
assert_eq "3" "$(printf '%s\n' "$sums" | grep -c .)" "SHA256SUMS still lists all three artifacts"
assert_contains "$(log_line_for "$SB" "releases/SHA256SUMS")" "max-age=60" \
  "SHA256SUMS stays a rolling object"
rm -rf "$SB"

# A publish whose sidecars did not land must not claim success, and must not
# move the rolling pointers that installers follow.
echo "publish_spaces.sh: a failed sidecar upload aborts the publish"
SB="$(new_env)"
make_artifacts "$SB" "26.7.61" "def5678"
(
  export FAKE_FAIL_KEY_GLOB='*.sha256'
  run_publish "$SB" "26.7.61" "def5678"
)
rc=$?
if [ "$rc" -ne 0 ]; then ok "publish fails when a sidecar upload fails"; else
  bad "publish fails when a sidecar upload fails"; fi
assert_file_absent "$SB/bucket/releases/wfl-latest-linux-x86_64.tar.gz" \
  "rolling Linux pointer is not moved by a failed publish"
assert_file_absent "$SB/bucket/status.json" \
  "status.json is not written by a failed publish"

# The artifact itself may have landed before the sidecar failed. That object is
# unreferenced - no pointer, no SHA256SUMS entry, no status.json - and the
# publish is retryable, so the fix is that a re-run completes it rather than
# tripping over its own leftovers. Assert exactly that, because a re-run that
# refused to proceed would strand the release until someone deleted the object
# by hand.
run_publish "$SB" "26.7.61" "def5678"
rc=$?
assert_eq "0" "$rc" "re-running the publish after a sidecar failure succeeds ($SB/out)"
assert_file_exists "$SB/bucket/releases/wfl-26.7.61-linux-x86_64-def5678.tar.gz.sha256" \
  "the re-run completes the sidecar the failed publish left missing"
assert_file_exists "$SB/bucket/releases/wfl-latest-linux-x86_64.tar.gz" \
  "the re-run moves the rolling pointer"
rm -rf "$SB"

# A freshly created key can take a moment to be readable at a CDN edge. Failing
# the whole release for that would be a false negative, and the release job's
# success marker is written after this step, so a blip costs a re-run.
echo "publish_spaces.sh: CDN verification tolerates a slow-propagating sidecar"
SB="$(new_env)"
make_artifacts "$SB" "26.7.62" "aaa1111"
(
  export FAKE_CURL_FLAKY_KEY="releases/wfl-26.7.62-linux-x86_64-aaa1111.tar.gz.sha256"
  export FAKE_CURL_FLAKY_TIMES=2
  run_publish "$SB" "26.7.62" "aaa1111"
)
rc=$?
assert_eq "0" "$rc" "publish survives a sidecar that 404s twice before propagating ($SB/out)"
assert_eq "2" "$(cat "$SB/flaky.count" 2>/dev/null || echo 0)" \
  "the sidecar fetch was actually retried"

# Tolerating a blip must not mean tolerating an absent object.
rm -f "$SB/flaky.count"
rm -rf "$SB/bucket" && mkdir -p "$SB/bucket"
(
  export FAKE_CURL_FLAKY_KEY="releases/wfl-26.7.62-linux-x86_64-aaa1111.tar.gz.sha256"
  export FAKE_CURL_FLAKY_TIMES=99
  run_publish "$SB" "26.7.62" "aaa1111"
)
rc=$?
if [ "$rc" -ne 0 ]; then ok "publish still fails when the sidecar never becomes readable"; else
  bad "publish still fails when the sidecar never becomes readable"; fi
rm -rf "$SB"

# `Cache-Control: immutable` governs caches, not bucket writes. A scheduled
# nightly rebuild of an already-published version (MSI bytes are not
# reproducible) used to abort the whole release and turn Nightly Build red.
# Versioned keys stay write-once: different bytes skip that artifact and
# succeed, identical bytes stay a no-op, and rolling pointers must not move
# to bytes that were not accepted under the versioned key.
echo "publish_spaces.sh: an immutable key is never replaced with different bytes"
SB="$(new_env)"
make_artifacts "$SB" "26.7.63" "bbb2222"
run_publish "$SB" "26.7.63" "bbb2222"
assert_eq "0" "$?" "first publish of 26.7.63 succeeds"

# Same version, different bytes: the rebuild case. Skip, do not fail, and do
# not point "latest" at the rejected rebuild.
printf 'DIFFERENT tarball bytes\n' > "$SB/artifacts/wfl-26.7.63-linux-x86_64-bbb2222.tar.gz"
before="$(cat "$SB/bucket/releases/wfl-26.7.63-linux-x86_64-bbb2222.tar.gz")"
before_sidecar="$(cat "$SB/bucket/releases/wfl-26.7.63-linux-x86_64-bbb2222.tar.gz.sha256")"
before_latest="$(cat "$SB/bucket/releases/wfl-latest-linux-x86_64.tar.gz")"
: > "$SB/log"
run_publish "$SB" "26.7.63" "bbb2222"
rc=$?
assert_eq "0" "$rc" "already-published different bytes skip without failing the publish ($SB/out)"
assert_eq "$before" "$(cat "$SB/bucket/releases/wfl-26.7.63-linux-x86_64-bbb2222.tar.gz")" \
  "the published artifact is left byte-identical"
assert_eq "$before_sidecar" "$(cat "$SB/bucket/releases/wfl-26.7.63-linux-x86_64-bbb2222.tar.gz.sha256")" \
  "the published sidecar is left byte-identical"
assert_eq "$before_latest" "$(cat "$SB/bucket/releases/wfl-latest-linux-x86_64.tar.gz")" \
  "the rolling Linux pointer stays on the published bytes, not the rejected rebuild"
assert_eq "0" "$(put_count "$SB" "releases/wfl-26.7.63-linux-x86_64-bbb2222.tar.gz")" \
  "a skipped rebuild does not PUT the versioned tarball"
assert_contains "$(cat "$SB/out")" "already published with different bytes" \
  "the skip names the immutable collision"

# Re-publishing identical bytes is the retry case, and must still work.
make_artifacts "$SB" "26.7.63" "bbb2222"
: > "$SB/log"
run_publish "$SB" "26.7.63" "bbb2222"
rc=$?
assert_eq "0" "$rc" "re-publishing identical bytes succeeds ($SB/out)"
assert_eq "0" "$(put_count "$SB" "releases/wfl-26.7.63-linux-x86_64-bbb2222.tar.gz")" \
  "re-publishing identical bytes does not rewrite the artifact"
assert_eq "0" "$(put_count "$SB" "releases/wfl-26.7.63-linux-x86_64-bbb2222.tar.gz.sha256")" \
  "re-publishing identical bytes does not rewrite the sidecar"
assert_file_exists "$SB/bucket/releases/wfl-latest-linux-x86_64.tar.gz" \
  "re-publishing identical bytes still refreshes the rolling pointer"
rm -rf "$SB"

# Nightly run 570: Linux tarball bytes match the published key; the MSI does
# not. That must not fail the workflow, overwrite the MSI, or move the Windows
# latest pointer onto the rejected rebuild.
echo "publish_spaces.sh: same-version MSI rebuild skips while identical tarball is accepted"
SB="$(new_env)"
make_artifacts "$SB" "26.9.6" "3bf6521f"
run_publish "$SB" "26.9.6" "3bf6521f"
assert_eq "0" "$?" "first publish of 26.9.6 succeeds"
published_msi="$(cat "$SB/bucket/releases/wfl-26.9.6.msi")"
published_msi_sidecar="$(cat "$SB/bucket/releases/wfl-26.9.6.msi.sha256")"
published_latest_msi="$(cat "$SB/bucket/releases/wfl-latest-windows-x86_64.msi")"
published_tarball="$(cat "$SB/bucket/releases/wfl-26.9.6-linux-x86_64-3bf6521f.tar.gz")"
printf 'rebuilt MSI bytes that are not bit-identical\n' > "$SB/artifacts/wfl-26.9.6.msi"
: > "$SB/log"
run_publish "$SB" "26.9.6" "3bf6521f"
rc=$?
assert_eq "0" "$rc" "identical tarball plus different MSI succeeds ($SB/out)"
assert_eq "$published_msi" "$(cat "$SB/bucket/releases/wfl-26.9.6.msi")" \
  "the published MSI is left byte-identical"
assert_eq "$published_msi_sidecar" "$(cat "$SB/bucket/releases/wfl-26.9.6.msi.sha256")" \
  "the published MSI sidecar is left byte-identical"
assert_eq "$published_latest_msi" "$(cat "$SB/bucket/releases/wfl-latest-windows-x86_64.msi")" \
  "the rolling Windows pointer stays on the published MSI"
assert_eq "$published_tarball" "$(cat "$SB/bucket/releases/wfl-26.9.6-linux-x86_64-3bf6521f.tar.gz")" \
  "the published tarball is left byte-identical"
assert_eq "0" "$(put_count "$SB" "releases/wfl-26.9.6.msi")" \
  "the different MSI is not re-uploaded"
assert_eq "1" "$(put_count "$SB" "releases/wfl-latest-linux-x86_64.tar.gz")" \
  "identical tarball still refreshes the rolling Linux pointer"
assert_eq "0" "$(put_count "$SB" "releases/wfl-latest-windows-x86_64.msi")" \
  "the rejected MSI rebuild does not refresh the rolling Windows pointer"
assert_contains "$(cat "$SB/out")" "wfl-26.9.6.msi" \
  "the skip names the MSI that already exists"
rm -rf "$SB"

# If every versioned artifact is already published with different bytes, the
# run is a complete no-op: do not rewrite SHA256SUMS or status.json as if a
# new publish landed, and do not fail the way an empty artifact dir fails.
echo "publish_spaces.sh: all-already-published different bytes is a successful no-op"
SB="$(new_env)"
make_artifacts "$SB" "26.9.6" "3bf6521f"
run_publish "$SB" "26.9.6" "3bf6521f"
assert_eq "0" "$?" "seed publish of 26.9.6 succeeds"
before_sums="$(cat "$SB/bucket/releases/SHA256SUMS")"
before_status="$(cat "$SB/bucket/status.json")"
printf 'DIFFERENT tarball bytes\n' > "$SB/artifacts/wfl-26.9.6-linux-x86_64-3bf6521f.tar.gz"
printf 'DIFFERENT msi bytes\n' > "$SB/artifacts/wfl-26.9.6.msi"
printf 'DIFFERENT vsix bytes\n' > "$SB/artifacts/vscode-wfl-26.9.6.vsix"
: > "$SB/log"
run_publish "$SB" "26.9.6" "3bf6521f"
rc=$?
assert_eq "0" "$rc" "all-skipped republish succeeds ($SB/out)"
assert_eq "$before_sums" "$(cat "$SB/bucket/releases/SHA256SUMS")" \
  "SHA256SUMS is left describing the real published artifacts"
assert_eq "$before_status" "$(cat "$SB/bucket/status.json")" \
  "status.json is not rewritten by an all-skipped republish"
assert_eq "0" "$(grep -c '^PUT' "$SB/log")" \
  "an all-skipped republish uploads nothing"
rm -rf "$SB"

# A later canonical version is a new key and must still publish. The skip
# path is only for the already-published versioned object, not a blanket
# "once anything exists, stop writing".
echo "publish_spaces.sh: a new version still publishes after a skipped same-version rebuild"
SB="$(new_env)"
make_artifacts "$SB" "26.9.6" "3bf6521f"
run_publish "$SB" "26.9.6" "3bf6521f"
assert_eq "0" "$?" "seed 26.9.6 publish succeeds"
printf 'rebuilt MSI bytes\n' > "$SB/artifacts/wfl-26.9.6.msi"
run_publish "$SB" "26.9.6" "3bf6521f"
assert_eq "0" "$?" "same-version MSI skip succeeds"
rm -rf "$SB/artifacts"
mkdir -p "$SB/artifacts"
make_artifacts "$SB" "26.9.7" "ddddddd"
run_publish "$SB" "26.9.7" "ddddddd"
assert_eq "0" "$?" "new version 26.9.7 still publishes ($SB/out)"
assert_file_exists "$SB/bucket/releases/wfl-26.9.7.msi" "new version MSI is written"
assert_eq "msi bytes for 26.9.7" "$(cat "$SB/bucket/releases/wfl-26.9.7.msi")" \
  "new version MSI holds the new bytes"
assert_eq "msi bytes for 26.9.6" "$(cat "$SB/bucket/releases/wfl-26.9.6.msi")" \
  "previous version MSI is left untouched"
assert_eq "msi bytes for 26.9.7" "$(cat "$SB/bucket/releases/wfl-latest-windows-x86_64.msi")" \
  "rolling Windows pointer advances only for the new version"
rm -rf "$SB"

# A missing artifact directory is still a failed publish. Skipping already-
# published keys must not weaken the empty-input guard.
echo "publish_spaces.sh: no artifacts still fails closed"
SB="$(new_env)"
run_publish "$SB" "26.9.7" "ccccccc"
rc=$?
if [ "$rc" -ne 0 ]; then ok "publish with no artifacts still fails"; else
  bad "publish with no artifacts still fails"; fi
assert_contains "$(cat "$SB/out")" "nothing was published" \
  "empty input still names the missing publish"
assert_file_absent "$SB/bucket/status.json" \
  "empty input does not write status.json"
rm -rf "$SB"

# ---------------------------------------------------------------------------
# backfill_spaces_checksums.sh
# ---------------------------------------------------------------------------
echo "backfill_spaces_checksums.sh: repairs history without touching it"

SB="$(new_env)"
put_object "$SB" "releases/wfl-26.7.57-linux-x86_64-2d74737.tar.gz" "old linux bytes"
put_object "$SB" "releases/wfl-26.7.57.msi"                         "old msi bytes"
put_object "$SB" "releases/vscode-wfl-26.7.57.vsix"                 "old vsix bytes"
put_object "$SB" "releases/wfl-26.7.59-linux-x86_64-579eb80.tar.gz" "new linux bytes"
put_object "$SB" "releases/wfl-26.7.59-linux-x86_64-579eb80.tar.gz.sha256" "pre-existing sidecar"
put_object "$SB" "releases/wfl-latest-linux-x86_64.tar.gz"          "new linux bytes"
put_object "$SB" "releases/wfl-latest-windows-x86_64.msi"           "new msi bytes"
put_object "$SB" "releases/SHA256SUMS"                              "whatever"
put_object "$SB" "status.json"                                      "{}"

run_backfill "$SB"
rc=$?
assert_eq "0" "$rc" "backfill succeeds ($SB/out)"

for k in "wfl-26.7.57-linux-x86_64-2d74737.tar.gz" "wfl-26.7.57.msi" "vscode-wfl-26.7.57.vsix"; do
  assert_file_exists "$SB/bucket/releases/$k.sha256" "backfilled sidecar for $k"
  if [ -f "$SB/bucket/releases/$k.sha256" ]; then
    want="$(sha_line "$SB/bucket/releases/$k")"
    assert_eq "$want" "$(cat "$SB/bucket/releases/$k.sha256")" \
      "backfilled sidecar for $k hashes the object's real bytes"
    assert_contains "$(log_line_for "$SB" "releases/$k.sha256")" "immutable" \
      "backfilled sidecar for $k is cached as immutable"
  fi
done

# Never rewrite what is already published: an existing sidecar is authoritative,
# and re-uploading it would defeat the immutability the fix depends on.
assert_eq "pre-existing sidecar" \
  "$(cat "$SB/bucket/releases/wfl-26.7.59-linux-x86_64-579eb80.tar.gz.sha256")" \
  "an existing sidecar is left untouched"
assert_eq "0" "$(put_count "$SB" "releases/wfl-26.7.59-linux-x86_64-579eb80.tar.gz.sha256")" \
  "an existing sidecar is not re-uploaded"

assert_file_absent "$SB/bucket/releases/wfl-latest-linux-x86_64.tar.gz.sha256" \
  "rolling Linux pointer gets no sidecar"
assert_file_absent "$SB/bucket/releases/wfl-latest-windows-x86_64.msi.sha256" \
  "rolling Windows pointer gets no sidecar"
assert_file_absent "$SB/bucket/releases/SHA256SUMS.sha256" \
  "SHA256SUMS gets no sidecar"
assert_file_absent "$SB/bucket/status.json.sha256" \
  "status.json gets no sidecar"

# Running it twice is the normal case (it is wired into the nightly publish), so
# the second run must be a no-op rather than a second round of uploads.
: > "$SB/log"
run_backfill "$SB"
rc=$?
assert_eq "0" "$rc" "second backfill run succeeds"
assert_eq "0" "$(grep -c '^PUT' "$SB/log")" "second backfill run uploads nothing"
rm -rf "$SB"

# The bucket listing is a snapshot. A publish that lands between the snapshot and
# the upload would otherwise have its sidecar replaced by one this run computed
# from the bytes it downloaded earlier - and if the artifact was rebuilt in
# between, that checksum describes bytes nobody can download any more.
echo "backfill_spaces_checksums.sh: a sidecar published mid-run is not overwritten"
SB="$(new_env)"
KEY="releases/wfl-26.7.57-linux-x86_64-2d74737.tar.gz"
put_object "$SB" "$KEY" "old linux bytes"
cat > "$SB/concurrent-publisher.sh" <<EOF
#!/usr/bin/env bash
# Stands in for a nightly publish winning the race: the sidecar appears after
# the backfill listed the bucket, while it is reading the artifact.
[ "\$1" = "$KEY" ] || exit 0
printf 'sidecar from the concurrent publisher' > "$SB/bucket/$KEY.sha256"
EOF
chmod +x "$SB/concurrent-publisher.sh"
(
  export FAKE_ON_GET_HOOK="$SB/concurrent-publisher.sh"
  run_backfill "$SB"
)
rc=$?
assert_eq "0" "$rc" "backfill succeeds when a publisher wins the race ($SB/out)"
assert_eq "sidecar from the concurrent publisher" "$(cat "$SB/bucket/$KEY.sha256")" \
  "the concurrently published sidecar is left intact"
assert_eq "0" "$(put_count "$SB" "$KEY.sha256")" \
  "the backfill uploads nothing over the concurrently published sidecar"
rm -rf "$SB"

echo "backfill_spaces_checksums.sh: --dry-run"
SB="$(new_env)"
put_object "$SB" "releases/wfl-26.7.57-linux-x86_64-2d74737.tar.gz" "old linux bytes"
run_backfill "$SB" --dry-run
rc=$?
assert_eq "0" "$rc" "dry run succeeds"
assert_eq "0" "$(grep -c '^PUT' "$SB/log")" "dry run uploads nothing"
assert_contains "$(cat "$SB/out")" "wfl-26.7.57-linux-x86_64-2d74737.tar.gz.sha256" \
  "dry run reports the sidecar it would create"
rm -rf "$SB"

echo
echo "passed: $PASS   failed: $FAIL"
[ "$FAIL" -eq 0 ]
