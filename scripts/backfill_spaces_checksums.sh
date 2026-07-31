#!/usr/bin/env bash
# Write the missing `.sha256` sidecars for artifacts already published to the
# DigitalOcean Spaces bucket that backs https://wfl.nyc3.cdn.digitaloceanspaces.com.
#
# Why this exists
# ---------------
# publish_spaces.sh writes an immutable `<artifact>.sha256` next to every
# versioned object it uploads, so a pinned build stays verifiable for as long as
# it stays downloadable. Artifacts published before that change have no sidecar:
# their only attestation was `releases/SHA256SUMS`, which is rewritten on every
# publish and therefore describes only the newest build (issue #662).
#
# This script repairs that history. It reads each artifact back from the bucket,
# hashes the bytes the bucket actually holds, and uploads the sidecar.
#
# It is safe to run at any time, and is meant to be:
#
#   * idempotent  - an artifact that already has a sidecar is skipped, never
#                   re-uploaded. Published checksums are immutable; rewriting
#                   one would defeat the point of publishing it.
#   * additive    - it only ever creates `*.sha256` keys. No artifact, rolling
#                   pointer, SHA256SUMS or status.json is touched.
#   * unattended  - failures are reported per object and summarised, so one bad
#                   object does not hide the rest of the run.
#
# Rolling keys (`wfl-latest-*`, `SHA256SUMS`, `status.json`) are deliberately
# skipped: a checksum published beside a key whose bytes change would be wrong
# the next time it changed, which is precisely the bug this whole change fixes.
#
# Usage: backfill_spaces_checksums.sh [--dry-run] [--prefix releases/]
#
# Required env: AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY
# Optional env: SPACES_BUCKET (default wfl), SPACES_ENDPOINT (default nyc3)

set -euo pipefail

DRY_RUN=0
PREFIX="releases/"

while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) DRY_RUN=1; shift ;;
    --prefix)  PREFIX="${2:?--prefix needs a value}"; shift 2 ;;
    # Prints the whole header block rather than a hardcoded line range, which
    # silently truncates the moment the header grows a line.
    -h|--help) awk 'NR > 1 && /^#/ { print; next } NR > 1 { exit }' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 64 ;;
  esac
done

BUCKET="${SPACES_BUCKET:-wfl}"
ENDPOINT="${SPACES_ENDPOINT:-https://nyc3.digitaloceanspaces.com}"

command -v jq >/dev/null || { echo "::error::jq is required but not installed"; exit 1; }

# Same Spaces incompatibility publish_spaces.sh documents: AWS CLI v2.23+ sends
# CRC32 integrity headers that Spaces rejects with an opaque 400.
export AWS_REQUEST_CHECKSUM_CALCULATION="${AWS_REQUEST_CHECKSUM_CALCULATION:-when_required}"
export AWS_RESPONSE_CHECKSUM_VALIDATION="${AWS_RESPONSE_CHECKSUM_VALIDATION:-when_required}"
export AWS_DEFAULT_REGION="${AWS_DEFAULT_REGION:-nyc3}"

IMMUTABLE="public, max-age=31536000, immutable"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ---------------------------------------------------------------------------
# Read the current bucket contents. The AWS CLI paginates list-objects-v2 for
# us, so a bucket larger than one page still yields every key here.
# ---------------------------------------------------------------------------
aws s3api list-objects-v2 \
  --bucket "$BUCKET" \
  --prefix "$PREFIX" \
  --endpoint-url "$ENDPOINT" \
  --output json > "$WORK/listing.json"

jq -r '.Contents[]?.Key' "$WORK/listing.json" | sort > "$WORK/keys.txt"

TOTAL_KEYS="$(grep -c . "$WORK/keys.txt" || true)"
echo "s3://${BUCKET}/${PREFIX}: ${TOTAL_KEYS} object(s)"

# An artifact is a versioned, immutable object. Everything else in the bucket
# either changes over time or is itself metadata about a publish.
is_artifact() { # is_artifact <key>
  local key="$1" base="${1##*/}"
  case "$base" in
    wfl-latest-*)         return 1 ;;  # rolling pointer
    SHA256SUMS|status.json) return 1 ;;
    *.sha256)             return 1 ;;  # a sidecar, not something to hash
  esac
  case "$key" in
    *.tar.gz|*.msi|*.vsix) return 0 ;;
    *)                     return 1 ;;
  esac
}

CREATED=0
SKIPPED=0
FAILED=0

while IFS= read -r key; do
  [ -n "$key" ] || continue
  is_artifact "$key" || continue

  if grep -qxF "${key}.sha256" "$WORK/keys.txt"; then
    SKIPPED=$((SKIPPED + 1))
    continue
  fi

  base="${key##*/}"

  if [ "$DRY_RUN" -eq 1 ]; then
    echo "  would create ${key}.sha256"
    CREATED=$((CREATED + 1))
    continue
  fi

  # Hash what the bucket serves, not what CI once built: the sidecar has to
  # attest to the bytes a consumer will actually download.
  if ! aws s3 cp "s3://${BUCKET}/${key}" "$WORK/$base" \
        --endpoint-url "$ENDPOINT" --only-show-errors; then
    echo "::warning::could not download ${key} - skipping"
    FAILED=$((FAILED + 1))
    continue
  fi

  ( cd "$WORK" && sha256sum "$base" ) > "$WORK/$base.sha256"

  # The listing above is a snapshot. A publish can land between it and this
  # upload - the on-demand workflow and the nightly can genuinely overlap - and
  # the publisher's sidecar is the authoritative one: it describes the bytes it
  # just uploaded, where this run's describes the bytes it read earlier. Never
  # replace it. (The workflows also share a concurrency group, so this window is
  # the residual case rather than the expected one.)
  if aws s3api head-object --bucket "$BUCKET" --key "${key}.sha256" \
        --endpoint-url "$ENDPOINT" >/dev/null 2>&1; then
    echo "  ${key}.sha256 was published while this run was working; leaving it alone"
    SKIPPED=$((SKIPPED + 1))
    rm -f "$WORK/$base" "$WORK/$base.sha256"
    continue
  fi

  if ! aws s3 cp "$WORK/$base.sha256" "s3://${BUCKET}/${key}.sha256" \
        --endpoint-url "$ENDPOINT" \
        --acl public-read \
        --content-type text/plain \
        --cache-control "$IMMUTABLE" \
        --only-show-errors; then
    echo "::warning::could not upload ${key}.sha256"
    FAILED=$((FAILED + 1))
    rm -f "$WORK/$base" "$WORK/$base.sha256"
    continue
  fi

  echo "  created ${key}.sha256  $(cut -d' ' -f1 < "$WORK/$base.sha256")"
  CREATED=$((CREATED + 1))
  rm -f "$WORK/$base" "$WORK/$base.sha256"
done < "$WORK/keys.txt"

if [ "$DRY_RUN" -eq 1 ]; then
  echo "dry run: ${CREATED} sidecar(s) would be created, ${SKIPPED} already present"
else
  echo "created ${CREATED} sidecar(s), skipped ${SKIPPED} already present, ${FAILED} failed"
fi

# A partial repair is still worth keeping - the sidecars that landed are
# correct - but the run must not report success, or a scheduled invocation
# would quietly leave gaps behind.
[ "$FAILED" -eq 0 ]
