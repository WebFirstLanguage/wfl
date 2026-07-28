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
#   releases/wfl-latest-linux-x86_64.tar.gz            rolling pointer
#   releases/SHA256SUMS                                checksums for this publish
#   status.json                                        last-publish record
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

# AWS CLI v2.23+ sends CRC32 integrity headers by default. DigitalOcean Spaces
# rejects them with a 400, so every upload fails with an opaque error unless
# these are relaxed. This is the single most common reason "aws s3 cp works
# against S3 but not Spaces".
export AWS_REQUEST_CHECKSUM_CALCULATION="${AWS_REQUEST_CHECKSUM_CALCULATION:-when_required}"
export AWS_RESPONSE_CHECKSUM_VALIDATION="${AWS_RESPONSE_CHECKSUM_VALIDATION:-when_required}"
export AWS_DEFAULT_REGION="${AWS_DEFAULT_REGION:-nyc3}"

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

find_one() { find "$ARTIFACT_DIR" -type f -name "$1" | head -1; }

TARBALL="$(find_one 'wfl-*-linux-x86_64-*.tar.gz')"
MSI="$(find_one '*.msi')"
VSIX="$(find_one '*.vsix')"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
: > "$WORK/SHA256SUMS"

PUBLISHED=()

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
  put "$TARBALL" "releases/$(basename "$TARBALL")" application/gzip "$IMMUTABLE"
  ( cd "$(dirname "$TARBALL")" && sha256sum "$(basename "$TARBALL")" ) >> "$WORK/SHA256SUMS"
  PUBLISHED+=("$(basename "$TARBALL")")
else
  echo "::warning::no Linux tarball found in $ARTIFACT_DIR"
fi

if [ -n "$MSI" ]; then
  echo "Windows MSI: $MSI"
  put "$MSI" "releases/$(basename "$MSI")" application/x-msi "$IMMUTABLE"
  ( cd "$(dirname "$MSI")" && sha256sum "$(basename "$MSI")" ) >> "$WORK/SHA256SUMS"
  PUBLISHED+=("$(basename "$MSI")")
else
  echo "::warning::no MSI found in $ARTIFACT_DIR"
fi

if [ -n "$VSIX" ]; then
  echo "VS Code extension: $VSIX"
  put "$VSIX" "releases/$(basename "$VSIX")" application/octet-stream "$IMMUTABLE"
  ( cd "$(dirname "$VSIX")" && sha256sum "$(basename "$VSIX")" ) >> "$WORK/SHA256SUMS"
  PUBLISHED+=("$(basename "$VSIX")")
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

cat > "$WORK/status.json" <<JSON
{
  "result":   "success",
  "sha":      "${COMMIT_SHA}",
  "version":  "${VERSION}",
  "message":  "built and published ${PUBLISHED[*]}",
  "branch":   "${BRANCH}",
  "finished": "$(date -u +%Y-%m-%dT%H:%M:%S+00:00)",
  "host":     "github-actions/blacksmith"
}
JSON
put "$WORK/status.json" "status.json" application/json "$ROLLING"

# Prove the CDN actually serves what we just uploaded. The bucket was private
# until this pipeline existed, so a silent ACL regression would break every
# install while the workflow still reported success.
CDN="https://${BUCKET}.nyc3.cdn.digitaloceanspaces.com"
echo "Verifying public readability via CDN..."
for key in "releases/SHA256SUMS" "status.json"; do
  code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 30 "${CDN}/${key}")"
  if [ "$code" != "200" ]; then
    echo "::error::${CDN}/${key} returned HTTP ${code} - objects are not public-read"
    exit 1
  fi
  echo "  200 ${CDN}/${key}"
done

echo "Published ${#PUBLISHED[@]} artifact(s) to s3://${BUCKET}/releases/"
