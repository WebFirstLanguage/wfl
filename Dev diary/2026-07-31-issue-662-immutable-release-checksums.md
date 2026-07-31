# 2026-07-31 — Pinned releases stay verifiable: immutable `.sha256` sidecars (#662)

## What changed

Every versioned artifact published to Spaces now gets its own checksum file,
written once and never rewritten:

```
releases/wfl-<version>-linux-x86_64-<sha>.tar.gz
releases/wfl-<version>-linux-x86_64-<sha>.tar.gz.sha256   <- new
releases/wfl-<version>.msi
releases/wfl-<version>.msi.sha256                         <- new
releases/vscode-wfl-<version>.vsix
releases/vscode-wfl-<version>.vsix.sha256                 <- new
```

Three parts:

1. **`publish_spaces.sh`** writes the sidecar in the same step as the artifact it
   describes, and verifies both back through the CDN before the publish is
   allowed to complete.
2. **`backfill_spaces_checksums.sh`** (new) writes the sidecars for everything
   already in the bucket. It runs after every nightly publish and on demand via
   the new **Backfill Release Checksums** workflow.
3. **`test_publish_spaces.sh`** (new) tests both, in CI, on every PR.

`releases/SHA256SUMS` is unchanged: same name, same location, same contents, same
rolling cache headers. Nothing that reads it today has to change.

## The bug

`SHA256SUMS` is rewritten from scratch on every publish, so it only ever
describes the newest build. The artifacts it describes are immutable and stay
served indefinitely — their attestation does not.

The consequence is that pinning a version and verifying it were mutually
exclusive. A downstream GitLab pipeline pinned `26.7.57 / 2d74737`, verified
against `SHA256SUMS`, and was green for two days; when 26.7.59 published on
2026-07-30 the pipeline went red with "sha256 … is not in the published
SHA256SUMS". Nothing was wrong with the pinned build. It just stopped being
provable.

The failure mode is worse than a plain outage because of how it reads. The
consumer sees "this checksum is not published", which is what a *tampered
download* looks like. The one signal we give people for detecting corruption
fires, routinely, on correct downloads — which is exactly how a check gets
switched off.

The header comment in `publish_spaces.sh` said `checksums for this publish`, so
the behaviour was intentional. The name and location were not: `SHA256SUMS`
sitting in `releases/` reads as "the manifest for the releases directory", and
that is how it was reasonably used.

## Why sidecars rather than a cumulative SHA256SUMS

Making `SHA256SUMS` cumulative would fix existing consumers with no client
change, which is genuinely attractive. It was rejected on two grounds:

- **It reintroduces a mutable object into the trust path.** A pinned consumer
  would still be fetching a file that every subsequent publish rewrites. Correct
  behaviour would then depend on every future publish preserving history —
  one bad merge, one restored-from-backup object, and pinned verification breaks
  again with the same confusing symptom.
- **It requires read-modify-write on a shared key.** Two publishes overlapping
  (a scheduled nightly and a manual dispatch, which this repo permits) can
  interleave and drop entries. There is no compare-and-swap here to prevent it.

A sidecar has neither property. It is written exactly once, next to the object it
describes, with the object's own lifetime and cache policy — `max-age=31536000,
immutable`, the same header as the artifact. Nothing rewrites it, so nothing can
race on it, and its correctness does not depend on the behaviour of any later
publish. It also composes with what people already do: `sha256sum -c
wfl-….tar.gz.sha256` verifies in place and exits non-zero on mismatch.

The sidecar carries sha256sum's own `<hash>  <name>` line rather than a bare
hash, so `sha256sum -c` works directly and the name is bound into the
attestation.

Rolling keys (`wfl-latest-*`, `SHA256SUMS`, `status.json`) deliberately get **no**
sidecar. A checksum published beside a key whose bytes change would be wrong the
next time it changed — the same bug in a new place, and one that would look
authoritative because of where it sat.

## Risk class and verification (R3)

**R3: backward compatibility and integrity/verification tooling.** This is the
publish path for every artifact users install, and the object it produces is the
one they check downloads against.

| Acceptance criterion | Gate that proves it | Layer |
|---|---|---|
| Every versioned artifact gets a sidecar holding its real hash | `test_publish_spaces.sh`: runs the real script, compares each sidecar against `sha256sum` of the artifact | Integration, in-CI |
| Sidecars are cached as immutable and served as text | Same test, asserting the recorded `--cache-control` / `--content-type` per key | Integration, in-CI |
| Rolling keys get no sidecar | Same test, asserting absence for `wfl-latest-*`, `SHA256SUMS`, `status.json` | Integration, in-CI |
| An artifact can never be published without its checksum | Sidecar upload is in phase 1 under `set -e`; test injects a sidecar-upload failure and asserts non-zero exit with rolling pointers and `status.json` unwritten | Integration, in-CI (negative) |
| Existing `SHA256SUMS` consumers are unaffected | Same test: still published, still lists all three artifacts, still `max-age=60` | Integration, in-CI |
| The CDN actually serves the sidecar we uploaded | `publish_spaces.sh` fetches each sidecar back and compares content; publish fails otherwise | E2E, against the real bucket, every nightly |
| The backfill hashes the bucket's bytes, not CI's | `test_publish_spaces.sh`: seeds objects, asserts each backfilled sidecar matches the stored object | Integration, in-CI |
| The backfill never rewrites a published checksum | Same test: a pre-existing sidecar is byte-identical afterwards and recorded zero uploads | Integration, in-CI (negative) |
| The backfill is safe to run repeatedly | Same test: second run performs zero uploads | Integration, in-CI |

**Red evidence.** `987f3a7` is a test-only commit, an ancestor of the Green
implementation: 13 failing / 15 passing, failing for the intended reasons — no
`.sha256` key for any artifact, a sidecar-upload failure not aborting the
publish, and no backfill script to run (127). The implementation commit takes it
to 43 passing / 0 failing.

**Layers executed.** `bash -n` and `shellcheck` on all three scripts,
`test_publish_spaces.sh` (43 assertions, 0 failures), `actionlint` on the three
changed/added workflows (clean; the remaining findings in `ci.yml` and
`nightly.yml` are pre-existing and in untouched steps).

The documented consumer flow was also checked against the live bucket: the
published `wfl-26.7.59-linux-x86_64-579eb80.tar.gz` was downloaded through the
CDN, and a sidecar in the exact format `publish_spaces.sh` writes was verified
with `sha256sum -c` → `OK`. That is the format and the command the install docs
now tell people to use.

`scripts/validate_docs_examples.py` is not applicable here — the documentation
change adds shell snippets, not WFL examples, and no `TestPrograms/` example was
touched. The Rust suites are likewise untouched by this change and run as usual
in `ci.yml`.

**Residual risk.**

- The publish path cannot be fully exercised without live Spaces credentials, so
  the stubbed tests plus the script's own CDN verification are what stand behind
  it until the first real nightly. Same posture as the original publishing
  change.
- The backfill has to be run once against the live bucket to repair the
  artifacts published before this change. Until it runs, those artifacts remain
  in the state #662 describes.
- A checksum served from the same host as the artifact demonstrates transport
  integrity, not authenticity. Signing remains a tracked Operations follow-up,
  and the install docs now say so plainly rather than implying more than the
  file provides.

## Testing a script that only ever runs in production

`publish_spaces.sh` had no automated tests, for an understandable reason: it only
runs in the nightly job, against a live bucket, with credentials no PR has. A
mistake in it is invisible until it has already published — or failed to publish
— a real release. That is precisely how #662 shipped.

`test_publish_spaces.sh` closes that gap by stubbing the one boundary a test must
not cross. The stub is a *recording fake with a real object store behind it*: a
directory that uploads write into and later reads (including the CDN
verification) read out of, plus a log of every key with its content-type and
cache-control. So the assertions are about bytes and headers that actually moved,
not about "the script called `aws`". Everything else is real — the real scripts,
real `sha256sum`, real `jq`.

Per the testing policy this is the right layer for it: the real Spaces boundary
is verified where it exists, by the publish job's own CDN round-trip on every
release.

## For consumers

Pinned verification, which is what anyone deploying WFL actually needs:

```bash
BASE=https://wfl.nyc3.cdn.digitaloceanspaces.com/releases
TARBALL=wfl-26.7.59-linux-x86_64-579eb80.tar.gz

curl -fLO "$BASE/$TARBALL"
curl -fLO "$BASE/$TARBALL.sha256"
sha256sum -c "$TARBALL.sha256"
```

`SHA256SUMS` keeps its existing meaning for anyone tracking `latest`, and the
install docs now state which file is for which case instead of leaving it to be
inferred from the name.
