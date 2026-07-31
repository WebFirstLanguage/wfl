#!/usr/bin/env bash
# Publish WFL release artifacts to the DigitalOcean Spaces bucket that backs
# https://wfl.nyc3.cdn.digitaloceanspaces.com.
#
# Spaces is the CANONICAL download location; the GitHub Release is a mirror.
# This script is the only writer, so the bucket layout is defined here:
#
#   releases/wfl-<version>-linux-x86_64-<sha>.tar.gz   immutable, versioned
#   releases/wfl-<version>.msi                         immutable, versioned
#   releases/vscode-wfl-<version>.vsix                 immutable, versioned
#   releases/<any-of-the-above>.sha256                 immutable, versioned
#   releases/wfl-latest-linux-x86_64.tar.gz            rolling pointer
#   releases/SHA256SUMS                                checksums for THIS publish
#   status.json                                        last-publish record
#
# Note the split between the two kinds of checksum. `SHA256SUMS` describes the
# publish that wrote it and nothing else: it is rewritten every run, so a
# consumer pinned to an older version finds their hash gone from it the moment a
# newer nightly lands, even though the artifact they pinned is immutable and
# still served. That is issue #662. The per-artifact `.sha256` sidecars exist so
# a pinned build stays verifiable for as long as it stays downloadable - they
# are written once, next to the object they describe, and never rewritten.
#
# Usage: publish_spaces.sh <artifact-dir> <version> <short-sha> <commit-sha> <branch>
#
# Required env: AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY
# Optional env: SPACES_BUCKET (default wfl), SPACES_ENDPOINT (default nyc3)

set -euo pipefail

ARTIFACT_DIR="${1:?artifact dir required}"
VERSION="${2:?version required}"
SHORT_SHA="${3:?short sha required}"
COMMIT_SHA="${4:?commit sha required}"
BRANCH="${5:-main}"

BUCKET="${SPACES_BUCKET:-wfl}"
ENDPOINT="${SPACES_ENDPOINT:-https://nyc3.digitaloceanspaces.com}"

# `status.json` is serialized with jq rather than a here-doc, so fail loudly and
# early if it is missing instead of halfway through a publish.
command -v jq >/dev/null || { echo "::error::jq is required but not installed"; exit 1; }

# AWS CLI v2.23+ sends CRC32 integrity headers by default. DigitalOcean Spaces
# rejects them with a 400, so every upload fails with an opaque error unless
# these are relaxed. This is the single most common reason "aws s3 cp works
# against S3 but not Spaces".
export AWS_REQUEST_CHECKSUM_CALCULATION="${AWS_REQUEST_CHECKSUM_CALCULATION:-when_required}"
export AWS_RESPONSE_CHECKSUM_VALIDATION="${AWS_RESPONSE_CHECKSUM_VALIDATION:-when_required}"
export AWS_DEFAULT_REGION="${AWS_DEFAULT_REGION:-nyc3}"

# The CDN hostname is <bucket>.<region>.cdn.digitaloceanspaces.com, so the region
# in it has to be the region we actually uploaded through. Hardcoding one would
# make the verification below fetch a host the objects were never written to the
# moment SPACES_ENDPOINT pointed elsewhere, failing a publish that succeeded.
ENDPOINT_HOST="${ENDPOINT#*://}"
ENDPOINT_HOST="${ENDPOINT_HOST%%/*}"
case "$ENDPOINT_HOST" in
  *.digitaloceanspaces.com) REGION="${ENDPOINT_HOST%%.*}" ;;
  *)                        REGION="$AWS_DEFAULT_REGION" ;;
esac
CDN="https://${BUCKET}.${REGION}.cdn.digitaloceanspaces.com"

IMMUTABLE="public, max-age=31536000, immutable"
# The rolling pointers must NOT inherit the CDN's 1-hour default TTL, or an
# installer can fetch a stale "latest" for an hour after a successful publish.
ROLLING="public, max-age=60, must-revalidate"

put() { # put <local-file> <remote-key> <content-type> <cache-control>
  aws s3 cp "$1" "s3://${BUCKET}/$2" \
    --endpoint-url "$ENDPOINT" \
    --acl public-read \
    --content-type "$3" \
    --cache-control "$4" \
    --only-show-errors
  echo "  -> s3://${BUCKET}/$2"
}

sha256_of() { sha256sum "$1" | cut -d' ' -f1; }

# The sidecar carries sha256sum's own line format ("<hash>  <name>") rather than
# a bare hash, so `sha256sum -c wfl-<version>....tar.gz.sha256` verifies the
# download in place with no shell plumbing on the consumer's side.
sha256_line_of() { ( cd "$(dirname "$1")" && sha256sum "$(basename "$1")" ); }

find_one() { find "$ARTIFACT_DIR" -type f -name "$1" | head -1; }

TARBALL="$(find_one 'wfl-*-linux-x86_64-*.tar.gz')"
MSI="$(find_one '*.msi')"
VSIX="$(find_one '*.vsix')"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
: > "$WORK/SHA256SUMS"

PUBLISHED=()
# Local paths of the immutable objects, parallel to PUBLISHED, so the CDN check
# at the end can compare what the CDN serves against the bytes we uploaded.
PUBLISHED_PATHS=()

# publish_immutable <local-file> <content-type>
#
# Uploads one versioned artifact plus the `.sha256` sidecar that keeps it
# verifiable after this publish stops being the newest one, and records the line
# in the SHA256SUMS for this run. The sidecar goes up in the same step as the
# artifact and under the same `set -e`, so an artifact can never end up in the
# bucket without the checksum that attests to it.
publish_immutable() {
  local f="$1" ct="$2" base
  base="$(basename "$f")"

  put "$f" "releases/$base" "$ct" "$IMMUTABLE"

  sha256_line_of "$f" > "$WORK/$base.sha256"
  put "$WORK/$base.sha256" "releases/$base.sha256" text/plain "$IMMUTABLE"

  cat "$WORK/$base.sha256" >> "$WORK/SHA256SUMS"
  PUBLISHED+=("$base")
  PUBLISHED_PATHS+=("$f")
}

# ---------------------------------------------------------------------------
# Phase 1: immutable, versioned objects.
#
# Every key written here is new, so nothing an installer can already be
# pointing at changes. With `set -e` a failure anywhere in this phase aborts
# before a single rolling pointer moves, leaving the previous publish fully
# intact rather than a mix of old and new. That is why the rolling pointers
# below are deliberately NOT written next to their immutable counterparts.
# ---------------------------------------------------------------------------
if [ -n "$TARBALL" ]; then
  echo "Linux tarball: $TARBALL"
  publish_immutable "$TARBALL" application/gzip
else
  echo "::warning::no Linux tarball found in $ARTIFACT_DIR"
fi

if [ -n "$MSI" ]; then
  echo "Windows MSI: $MSI"
  publish_immutable "$MSI" application/x-msi
else
  echo "::warning::no MSI found in $ARTIFACT_DIR"
fi

if [ -n "$VSIX" ]; then
  echo "VS Code extension: $VSIX"
  publish_immutable "$VSIX" application/octet-stream
fi

if [ "${#PUBLISHED[@]}" -eq 0 ]; then
  echo "::error::nothing was published - refusing to overwrite SHA256SUMS/status.json"
  exit 1
fi

# ---------------------------------------------------------------------------
# Phase 2: rolling pointers and metadata.
#
# Only reached once every immutable upload above succeeded, so "latest",
# SHA256SUMS and status.json always describe the same publish. A failure here
# can still leave the Linux pointer ahead of the Windows one, so the pointers
# go first and the metadata that claims the publish is complete goes last.
# ---------------------------------------------------------------------------
echo "All immutable objects uploaded; updating rolling pointers..."

if [ -n "$TARBALL" ]; then
  put "$TARBALL" "releases/wfl-latest-linux-x86_64.tar.gz" application/gzip "$ROLLING"
fi

if [ -n "$MSI" ]; then
  put "$MSI" "releases/wfl-latest-windows-x86_64.msi" application/x-msi "$ROLLING"
fi

put "$WORK/SHA256SUMS" "releases/SHA256SUMS" text/plain "$ROLLING"

# Serialized with jq, not a here-doc: VERSION, BRANCH and the artifact names are
# interpolated values, and a quote or newline in any of them would silently
# publish a status.json that no consumer can parse. jq escapes them.
jq -n \
  --arg sha      "$COMMIT_SHA" \
  --arg version  "$VERSION" \
  --arg message  "built and published ${PUBLISHED[*]}" \
  --arg branch   "$BRANCH" \
  --arg finished "$(date -u +%Y-%m-%dT%H:%M:%S+00:00)" \
  '{
     result:   "success",
     sha:      $sha,
     version:  $version,
     message:  $message,
     branch:   $branch,
     finished: $finished,
     host:     "github-actions/blacksmith"
   }' > "$WORK/status.json"
put "$WORK/status.json" "status.json" application/json "$ROLLING"

# Prove the CDN actually serves what we just uploaded. The bucket was private
# until this pipeline existed, so a silent ACL regression would break every
# install while the workflow still reported success.
echo "Verifying public readability via CDN..."

# For the artifacts, a 200 is not enough - it proves a key exists, not that the
# bytes an installer downloads are the ones we built. Pull each immutable object
# back through the CDN and compare its SHA-256 against the local file.
for f in "${PUBLISHED_PATHS[@]}"; do
  key="releases/$(basename "$f")"
  want="$(sha256_of "$f")"
  if ! curl -fsS --max-time 300 -o "$WORK/verify.bin" "${CDN}/${key}"; then
    echo "::error::could not fetch ${CDN}/${key} - the object is not publicly readable"
    exit 1
  fi
  got="$(sha256_of "$WORK/verify.bin")"
  rm -f "$WORK/verify.bin"
  if [ "$want" != "$got" ]; then
    echo "::error::${CDN}/${key} does not match the artifact we uploaded (want ${want}, got ${got})"
    exit 1
  fi
  echo "  ok ${CDN}/${key} sha256=${want}"

  # The sidecar is the whole point of pinning a build, so an unreadable or
  # mismatched one is a failed publish, not a warning. Both objects are
  # immutable, so unlike the rolling keys below there is no cache window in
  # which a correct publish could legitimately serve something else.
  sidecar="${key}.sha256"
  if ! curl -fsS --max-time 30 -o "$WORK/verify.sha256" "${CDN}/${sidecar}"; then
    echo "::error::could not fetch ${CDN}/${sidecar} - the checksum sidecar is not publicly readable"
    exit 1
  fi
  if [ "$(cat "$WORK/verify.sha256")" != "$(sha256_line_of "$f")" ]; then
    echo "::error::${CDN}/${sidecar} does not describe the artifact we uploaded"
    exit 1
  fi
  rm -f "$WORK/verify.sha256"
  echo "  ok ${CDN}/${sidecar}"
done

# The rolling keys get a byte check only on their status code: they carry
# max-age=60, so the CDN may legitimately still be serving the previous publish
# for up to a minute and a hash comparison here would be flaky by design.
for key in "releases/SHA256SUMS" "status.json"; do
  code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 30 "${CDN}/${key}")"
  if [ "$code" != "200" ]; then
    echo "::error::${CDN}/${key} returned HTTP ${code} - objects are not public-read"
    exit 1
  fi
  echo "  200 ${CDN}/${key}"
done

echo "Published ${#PUBLISHED[@]} artifact(s) to s3://${BUCKET}/releases/"
