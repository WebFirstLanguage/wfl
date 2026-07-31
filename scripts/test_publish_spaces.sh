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
        ;;
    esac
    ;;
  s3api)
    op="${1:-}"; shift || true
    [ "$op" = "list-objects-v2" ] || { echo "fake aws: unsupported: s3api $op" >&2; exit 64; }
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
