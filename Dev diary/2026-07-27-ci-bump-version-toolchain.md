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

```text
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

- **Acceptance criteria → tests** (all in `tests/workflow_rust_toolchain_test.rs`):

  | Acceptance criterion | Test |
  |---|---|
  | Every workflow job that runs Cargo — directly or via a `scripts/*.py` indirection — installs/selects a toolchain *before* the first Cargo use | `cargo_jobs_install_a_rust_toolchain_first` |
  | The Cargo-invoking script inventory (`CARGO_INVOKING_SCRIPTS`) cannot silently go stale as scripts change | `cargo_invoking_scripts_list_is_complete` |
  | The scanner ignores comment-only `cargo` mentions yet still finds real jobs | `scanner_ignores_comments_and_finds_jobs` |
  | Only genuine toolchain setup counts; `echo rust-toolchain` and `rustup target/component add` do not | `toolchain_markers_reject_incidental_mentions` |
  | Cargo-argv detection survives whitespace, quote-style, and multi-line reformatting of `subprocess.run([...])` | `script_cargo_detection_tolerates_whitespace_and_argv_forms` |
  | A toolchain installed *after* the first Cargo step is treated as out of order, not as satisfying the rule | `toolchain_after_cargo_is_out_of_order` |

- **Red → Green:** `tests/workflow_rust_toolchain_test.rs` was committed test-only
  in `2ab115d` (an ancestor of the fix commit) and failed there for the intended
  reason — the two toolchain-less jobs — before any `ci.yml`/`versioning.yml`
  edit existed:

  ```text
  no toolchain step: ["ci.yml:bump-version", "versioning.yml:bump-version"]
  ```

  It passes on the fix commit. The `toolchain_markers_reject_incidental_mentions`
  and `script_cargo_detection_...` guards were added after review to tighten the
  scanner so it matches real setup steps and Cargo argv forms rather than loose
  substrings.

  The tightened scanner was then re-checked against the original defect: with the
  `dtolnay/rust-toolchain@stable` step temporarily removed from `ci.yml`,
  `cargo_jobs_install_a_rust_toolchain_first` fails again with
  `no toolchain step: ["ci.yml:bump-version"]`. The hardening did not cost the
  guard its bite.

- **Validation evidence** (rustc 1.94.1, satisfies `rust-version = "1.94"`):

  ```text
  $ cargo fmt --all -- --check
  # clean, no diff

  $ cargo clippy --all-targets --all-features -- -D warnings
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 41s   # no warnings

  $ cargo test --test workflow_rust_toolchain_test
  test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```

  Full workspace suite: CI run 30240210815 on this branch — `Build, Test, Clippy`
  green, including its `Run Tests` step, alongside the integration, database,
  fuzz-compile, and WFL-program jobs on both Linux and Windows. (A local
  `cargo test --all` in the authoring container exhausted its disk allowance
  mid-link — a `Bus error` from the ~30 GB `target/` tree described in
  `CLAUDE.md`, not a test failure; CI runners carry the `Free disk space` step
  that the container lacks.)

- **Real boundary:** `cargo check --locked --manifest-path fuzz/Cargo.toml` —
  the exact command that failed in CI — was run locally on an MSRV-satisfying
  toolchain (rustc 1.94.1) and passes, confirming an adequate toolchain is the
  whole of the fix.
- **Residual risk:** the guard is a line scanner, not a YAML parse (the repo has
  no YAML dependency), so it assumes job ids are the only two-space-indented
  keys under `jobs:` — true for all current workflows and asserted by the
  scanner self-test. It also cannot see Cargo invoked from a shell script or a
  composite action; only `scripts/*.py` indirection is resolved.
