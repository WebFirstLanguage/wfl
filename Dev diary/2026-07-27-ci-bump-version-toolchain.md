# 2026-07-27 — main CI red: `Bump Version` had no Rust toolchain

## Symptom

Every push to `main` since 2026-07-26 produced a red **CI** run, while all
seven build/test jobs went green. Three consecutive runs failed identically:

| Run | Commit | Failing job |
|---|---|---|
| 30205164652 | `ed704e6a` (#646, Blacksmith migration) | Bump Version |
| 30208231213 | `7937a53b` (#649) | Bump Version |
| 30233263720 | `f903e8e0` (#650) | Bump Version |
| 30237670286 | `48c46423` (#652) | Bump Version |

The job log:

```
error: rustc 1.92.0 is not supported by the following packages:
  sqlx@0.9.0 requires rustc 1.94.0
  ...
  wfl@26.7.53 requires rustc 1.94
Error: locked fuzz check failed after bump; refusing to stage a broken fuzz/Cargo.lock
```

## Root cause

`bump-version` in `.github/workflows/ci.yml` runs
`python scripts/bump_version.py --update-all`. That script shells out to Cargo
three times — `cargo update --package wfl` for the root lock, the same for
`fuzz/Cargo.lock`, and finally `cargo check --locked --manifest-path
fuzz/Cargo.toml` as a guard so a bump can never stage a fuzz lock that the
`fuzz-check` gate would then reject.

The job never installed a toolchain. Every *other* Cargo-running job in the repo
uses `dtolnay/rust-toolchain@stable`; this one silently inherited whatever
`rustc` the runner image happened to preinstall. That worked on GitHub-hosted
images and stopped working the moment CI moved to Blacksmith runners (#646),
whose `ubuntu-2404` image ships rustc 1.92.0 — below our
`rust-version = "1.94"` (raised by the `sqlx` 0.9 dependency). The locked fuzz
check failed, the script refused to stage, and the job exited 1.

Two things made this hard to see at a glance:

- The Cargo dependency is **invisible in the workflow file** — it hides behind a
  Python script, so a reader scanning for `cargo` in `ci.yml` finds nothing in
  this job.
- The failure is a *consequence* of the migration PR but appears in a job that
  PR never touched beyond the `runs-on:` label.

Because the bump never landed, `main` has been stuck at version **26.7.52** and
no `v*` tags were pushed for those four commits.

## Fix

Give the job the toolchain it always needed, before the bump step:

- `.github/workflows/ci.yml` — added `dtolnay/rust-toolchain@stable`, plus a
  restore-only `Swatinem/rust-cache` sharing `fuzz-check`'s `fuzz-check-cache`
  key so the locked fuzz check reuses that job's build instead of compiling the
  workspace a second time on every push to `main`.
- `.github/workflows/versioning.yml` — the manual-dispatch `bump-version` job
  had the identical latent defect (it would have failed the same way the next
  time anyone triggered it). Same one-line fix.

Deliberately *not* changed: the `cargo check --locked` guard inside
`bump_version.py`. It is doing exactly its job — it caught a broken environment
and refused to commit a lockfile it could not verify. Weakening it to "skip when
cargo is unhappy" would trade a loud failure for a silently stale
`fuzz/Cargo.lock`.

## Testing (Logbie Testing Policy)

- **Risk class:** R1 — build/release tooling, no runtime behavior change.
- **Red → Green:** `tests/workflow_rust_toolchain_test.rs` was committed
  test-only in `2ab115d` and failed there for the intended reason:

  ```
  no toolchain step: ["ci.yml:bump-version", "versioning.yml:bump-version"]
  ```

  It passes on the fix commit. The test scans every job in every workflow,
  resolves the Python-script indirection through `CARGO_INVOKING_SCRIPTS`, and
  asserts a toolchain marker appears *before* the first Cargo use. A companion
  test re-derives the script list from `scripts/*.py` so the indirection list
  cannot go stale, and a third pins the scanner's own behavior (comments
  mentioning `cargo` must not count).
- **Real boundary:** `cargo check --locked --manifest-path fuzz/Cargo.toml` —
  the exact command that failed in CI — was run locally on rustc 1.94.1 and
  passes, confirming an MSRV-satisfying toolchain is the whole of the fix.
- **Residual risk:** the guard is a line scanner, not a YAML parse (the repo has
  no YAML dependency), so it assumes job ids are the only two-space-indented
  keys under `jobs:` — true for all current workflows and asserted by the
  scanner self-test. It also cannot see Cargo invoked from a shell script or a
  composite action; only `scripts/*.py` indirection is resolved.
