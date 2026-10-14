# 2026-10-09 — Nightly versions from v-tags, no post-merge push to main

The LOG-16 ruleset (created 2026-09-26, no bypass actors) requires pull
requests and 12 status checks on `main` and `dev`. The post-merge
`bump-version` job in `ci.yml` could no longer push its commit: run
37889671264 failed with GH013 ("Changes must be made through a pull
request" plus 12 expected checks). Every `main` CI run went red and
`.build_meta.json` stayed at 26.9.19, so nightly-2026-09-26 and
nightly-2026-10-09 both shipped that number.

The running YY.M.BUILD count is kept, but it is now numbered from git
tags (tags are not covered by the ruleset). `scripts/bump_version.py
--from-tags` reads `vYY.M.N` via `git ls-remote --tags`. For the current
UTC month the next version is max N + 1, or 1 if none exist. The
committed `.build_meta.json` is a same-month floor only — an older-month
floor such as 26.9.19 must not turn 26.10 into .20. A build just after
midnight UTC on the 1st uses the new month, matching `nightly-YYYY-MM-DD`.

`--set-version` writes every version mirror at build time without
committing. Nightly computes the version once in `check-for-changes`
(`inputs.version_override` still wins) and both Windows and Linux run
`--set-version` before compile. `--version` on the shipped `wfl` binary
must match that number. After Spaces publish succeeds, the workflow
pushes `vX.Y.Z` at the built commit (`contents: write`, `GITHUB_TOKEN`).
A same-commit rerun reuses the existing v-tag instead of burning a new
number. The `nightly-YYYY-MM-DD` tag behaviour is unchanged.

CI no longer pushes version commits to protected branches. The
`bump-version` job, `scripts/push_version_bump.sh`, and its test are
gone. `versioning.yml` is a dry-run that only prints the next tag-based
version. The 12 required check job names are unchanged.

Risk class is R3: release numbering, tag lifecycle, and the protected
branch push path. Tests cover month/year rollover, numeric sort of
single- vs double-digit months and build numbers, same-month vs
older-month floor, HEAD tag reuse, `--set-version` writes without a
commit, and workflow policy (no push to `main`/`dev`, set-version before
compile, v-tag after publish).
