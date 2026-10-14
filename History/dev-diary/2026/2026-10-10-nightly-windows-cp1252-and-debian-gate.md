# 2026-10-10 — Nightly 38027033412: Windows cp1252 and the Debian 12 version gate

The first tag-based nightly after #782 computed `VERSION=26.10.1` correctly
and then published nothing. Both build jobs died on steps that PR had added
and that had never executed on `main`.

## Windows: `UnicodeEncodeError` on U+2713

`Build WFL for Windows` failed at **Set version for this build**.
`scripts/bump_version.py` printed `✓ Cargo.lock synchronized:` (U+2713).
The Blacksmith Windows 2025 console is cp1252, so `print` raised
`UnicodeEncodeError` at `update_cargo_lock` (line 378 in that revision).
The same mark is on the fuzz-lock success path and would have been next.

Fix is three independent layers:

1. Status lines are ASCII (`OK ... synchronized`).
2. `configure_stdio()` reconfigures stdout/stderr to UTF-8 at startup.
3. `nightly.yml` sets `PYTHONIOENCODING: utf-8` at workflow scope and again
   on both `--set-version` steps.

A subprocess test runs `--set-version --update-all --skip-git` with
`PYTHONIOENCODING=cp1252` and `PYTHONUTF8=0`.

Other scripts Windows workflows actually run:

- `validate_docs_examples.py` already wraps stdio in UTF-8 (and already
  runs on the Windows CI matrix).
- `check_repo_hygiene.py` prints ASCII only.
- PowerShell `Write-Host "✓ ..."` in `nightly.yml` / `ci.yml` is not
  Python and has already survived previous Windows nightlies.

## Linux: `$actual` leaked out of `sh -euc '...'`

`Build WFL for Linux (static musl)` built, smoked, and packaged 26.10.1,
then died at **Prove portability on Debian 12** with
`line 2: actual: unbound variable`. Docker never started.

PR #782 added a version assertion inside `debian:12-slim sh -euc '...'`.
The error line used `'$actual'`, which closed the single-quoted `-c`
script. The outer `set -u` bash then expanded an unbound `$actual` while
parsing the `docker run` command.

The inner script is now a quoted heredoc on `docker run --rm -i ... sh -eu`.
`$VERSION` / `$TARBALL` still come from container env. A workflow test
extracts that `run:` block and executes it under `bash -u` with a stub
`docker`; it reproduced the unbound-variable failure on the #782 text
and now requires the inner script to reach docker still containing
`./wfl --version` and `$actual`.

## Other #782 steps that had never run

Reviewed, no similar quoting or encoding bug:

- Host Linux smoke test assigns `$actual` in the same shell before using it.
- Windows installer `--version` check is PowerShell.
- `Tag the published version` uses `"refs/tags/$TAG"` / `"refs/tags/$TAG^{}"`
  in double quotes under `set -u` after `VERSION` is set.
- Artifact names already interpolated the computed version; they never ran
  because both build jobs failed first.

Risk class **R3** (release numbering / publish path). No ruleset bypasses,
PATs, or new secrets.
