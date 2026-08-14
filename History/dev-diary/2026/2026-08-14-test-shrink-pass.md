# 2026-08-14 — first test-shrink pass

First real run of the `test-shrink` skill. Three candidates landed; the headline
is that the Rust test suite went from **impossible to build** in a 30 GB
environment to building and passing in full.

## Metrics — before / after

| Metric | Before | After | Delta |
|---|---|---|---|
| `cargo test --all` | **failed to link**, 0 tests ran | 2096 passed, 0 failed | suite is runnable |
| All test binaries | 25.13 GB (156, incomplete) | **11.22 GB** (180, complete) | −55 % on a larger set |
| Mean per test binary | 165 MB | **63.9 MB** | −61 % |
| `target/` | 27 G | 17 G | −37 % |
| Test source lines | 42,940 | **42,629** | −311 |
| `tests/common/mod.rs` | 21 lines (1 helper) | 216 lines (14 shared helpers) | shared harness now real |
| Integration (`TestPrograms/`) | not reachable | 136 passed, 0 failed, 24 skipped | — |
| Full-suite wall time | n/a | 106–139 s | — |

The binary-size comparison is conservative: the "before" 25.13 GB was an
*incomplete* set, because the build died partway. The complete 180-target set at
the old mean would have been ~29.7 GB, against 11.22 GB now.

## The baseline was red, and that was the finding

`cargo test --all` exited 101 without running a single test:

```
error: linking with `cc` failed: exit status: 1
  = note: collect2: fatal error: ld terminated with signal 7 [Bus error]
```

`ld ... signal 7` is the ENOSPC-during-link signature CLAUDE.md already warns
about. Disk hit 0 G mid-build. 143 top-level `tests/*.rs` files each statically
link the whole compiler with full debug info; at ~165 MB apiece that is more
than the entire disk allowance. The base was not broken — the suite simply could
not be built.

## Candidates accepted

**C1 — `debug = "line-tables-only"` on the dev and test profiles** (`6f4c686`)

Test binary 199.5 MB → 56.1 MB; `libwfl-*.rlib` 291.0 MB → 85.5 MB. This is what
made every later step possible.

Two things worth recording, because both contradict the skill's own playbook:

- **`debug = 1` is not an alias for `line-tables-only`.** Numeric levels are
  0 = none, 1 = limited, 2 = full; `line-tables-only` is a separate named level,
  lighter than 1. The playbook's inline comment (`debug = 1 # line-tables-only`)
  is wrong and should be corrected.
- **`[profile.test]` alone is exactly right — and the first version of this
  change got that wrong.** It originally set `[profile.dev]` *and*
  `[profile.test]`, on the belief that integration-test binaries link the
  library as a *dev*-profile dependency. That belief is false, and PR #694
  review (Codex) caught it. Under `cargo test` the **entire** build graph — the
  local library, the `wfl` binary, and every external dependency — is built with
  the `test` profile. Verified with a throwaway crate built with deliberately
  different values (`[profile.dev] debug = 0`, `[profile.test] debug = 2`): the
  plain `--crate-type lib` rlib *and* an external path dependency both compiled
  with `-C debuginfo=2`, i.e. the test profile, never dev's 0. Confirmed against
  the real repo too — dropping the `[profile.dev]` override left the test binary
  at 56.1 MB and `libwfl-*.rlib` at 85.5 MB, byte-identical to before. So the
  dev override bought nothing for tests and only cost contributors variable and
  type DWARF in ordinary `cargo build` / `cargo run` binaries. It was removed;
  `[profile.dev]` stays at full debug info.

No `strip` was added — it would remove the very line tables the setting keeps.
`[profile.release]` was not touched; its `debug = true` stays deliberate.

Readable-panic check (mandatory for a debug-info reduction) passed — a
deliberately broken assertion still reports its location:

```
thread 'test_explicit_token_stream_analysis' panicked at
tests/colon_consumption_test.rs:69:5
...
4: colon_consumption_test::test_explicit_token_stream_analysis
           at ./tests/colon_consumption_test.rs:69:5
```

**C2 — hoist in-memory WFL drivers into `tests/common/`** (`12f727b`)
12 files, net −229 lines. Three driver shapes (`run_wfl`, `run_wfl_code`,
`run_wfl_ok`) plus shared accessors.
`http_connection_reuse_retry_test.rs` was deliberately left alone: its `get_text`
matches `Option<Value>` directly instead of routing through a panicking
`get_var`, so the missing-variable path genuinely differs.

**C3 — hoist the wfl-binary spawn harness into `tests/common/`** (`d1b8aef`)
13 files, net −82 lines. The real CLI boundary is preserved everywhere.

The trap here: the two original `wfl_exe()` variants resolve to **different
binaries** — `env!("CARGO_BIN_EXE_wfl")` is the profile-matched test binary,
while the hardcoded path is the separately built `target/release/wfl`. They were
kept as two helpers (`wfl_exe` / `wfl_release_exe`) and every call site retained
the binary it had. Unifying them would have silently changed what several tests
actually exercise.

For C2 and C3 the coverage evidence was a before/after `--list` test-name
inventory per target, diffed line by line: byte-identical every time, and the
full-suite count held at 2096 throughout.

## Rejected / not pursued

- **No test merges.** The redundancy hunt read every same-named family
  (`concurrent_*`, `crypto_*`/`wflhash_*`, `database_*`, `web_server_tls*`,
  `*_backcompat_*`, `*return_type*`) and found **no pair** where one file's
  assertions are provably a subset of another's. The naming discipline tracks
  real architectural seams — pipeline stage, real-binary vs in-process boundary.
  Shared test *names* across `web_server_tls_test` and
  `web_server_tls_parser_test` are deliberate dual-layer coverage, which the
  testing policy explicitly wants.
- **Print/noise removal was demoted.** `cargo test` captures stdout and stderr
  for passing tests and discards them, so `println!` in test bodies costs
  ~0 bytes on a green run. The suite prints 181 KB total. Not worth a candidate.

## Discovered defects — not fixed here, but real

1. **Flaky test: `interpreter::process_tests::test_capture_process_output`**
   (`src/interpreter/mod.rs:19109`). Spawns `echo`, sleeps a fixed 200 ms, then
   asserts the output was captured. It failed once during a loaded full-suite
   run and passes 5/5 in isolation. Fixed-sleep synchronisation under load. By
   §8.2 of the testing policy a flaky required test is a failing test. It lives
   in `src/`, so it was out of scope for this pass.
2. **`cargo test --all` is not self-sufficient.** `tests/binary_io_test.rs`
   hardcodes `target/release/wfl`, so without a prior `cargo build --release`
   six tests fail with an opaque
   `Os { code: 2, kind: NotFound }` rather than a clear "run cargo build
   --release first" message.
   *Partly addressed during PR #694 review:* both Devin and Copilot
   independently flagged that hoisting `wfl_release_exe()` into the shared
   harness made this trap easier to reach, since a future author picking
   `run_file_status` silently gets the release binary. The shared helper now
   anchors the path to `CARGO_MANIFEST_DIR` (so it no longer depends on the test
   process's working directory) and asserts the binary exists with an actionable
   "Run `cargo build --release` first" message. `binary_io_test.rs` still has its
   own copy of the hardcoded path and is unfixed — see issue #692.
3. **30 file-I/O tests can drop files into the repo root on failure.**
   `tests/file_io_{performance,concurrent,error_handling,execution}_test.rs` call
   `cleanup_test_files()` as a plain trailing statement with no `Drop` guard, so
   a failing assertion skips cleanup — turning one red test into a second,
   confusing hygiene-job failure. One sibling test in the same file already does
   it correctly with `tempfile::tempdir()`.
4. **`TestPrograms` leaks.** `error_handling_comprehensive.wfl:249-251` creates
   `test_error_file.txt` and never deletes it (CI-SKIPped, so manual runs only);
   five gated programs delete only on the happy path. The fix pattern already
   exists in `file_io_comprehensive.wfl:10,103,202`.
5. **Superseded test binaries accumulate without bound.** Cargo never deletes a
   removed target's old executables. Repeated builds silently grew
   `target/debug/deps` from 180 to 330 files (11 GB → 22 GB) and exhausted the
   disk four times this session. Any measurement of `deps/` must prune to the
   current target graph first or it compares unlike with unlike.

## Proposals for Brad

Maintainer-decision items, deliberately not applied:

- **Binary consolidation (playbook §1) is still the big structural win.** The
  scouting is done and the pilot is fully specified: group I (crypto + stdlib) —
  `crypto_test`, `crypto_async_test`, `crypto_kdf_test`, `crypto_seal_test`,
  `sha256_hmac_test`, `wflhash_hardened_security_test`, `wflhash_security_test`,
  `password_hashing_test`, `random_functions_test`, `toml_test` (2,973 lines).
  Verified clean: no `mod common`/`mod test_helpers`/`#[path]`/`include!`/inner
  attributes/`extern crate`/`fn main`, and — tree-wide, not just in this batch —
  **zero** `set_var`, `remove_var`, `set_current_dir`, `static mut`,
  `lazy_static`, `OnceLock`, or `Once`. That last result matters: the usual
  danger of putting many tests in one process (shared global state) does not
  exist in this codebase, so consolidation is far safer here than usual.
- **`test_helpers.rs` is doing double duty** — it is simultaneously a standalone
  test target with 6 of its own tests and a `mod`-included helper for 9 other
  files, so it is compiled 10 times. Deserves its own candidate, and it must be
  resolved before migrating any of its 9 consumers.
- **`run_src()` has no timeout.** `phase1_correctness_regression_test.rs` has a
  better `run_files` with a wall-clock kill and pinned `WFL_GLOBAL_CONFIG_PATH`;
  the other call sites can hang the whole run. Behavioral change, so a separate
  candidate.
- **`opt-level` tuning** (`[profile.test] opt-level = 1`,
  `[profile.dev.package."*"] opt-level = 2`) was deliberately not bundled with
  C1. Plausible runtime win for interpreter-heavy tests; needs its own
  measurement.
- **Fix the playbook's `debug = 1` error** in
  `.claude/skills/test-shrink/references/rust-test-optimization.md:70`.
- **Orphaned fixture** `tests/fixtures/tooling/combiner/test_file_list.wfl` — no
  consumer anywhere. It was moved there by the 2026-07-28 hygiene migration for a
  test that was never written. "Finish or remove" is the combiner tool owner's
  call, not a silent deletion.
- **`constant_error_precedence.wfl` scenario 3** is a coverage *gap* dressed as
  bloat — the scenario exists but nothing pins its intent. Prefer adding an
  assertion over trimming it.
- Larger files worth a future internal-redundancy pass: `overload_test.rs`
  (2,131 lines), `typechecker_gradual_any_test.rs` (1,262),
  `write_web_postfix_test.rs` (1,155).

## Gates

Every accepted candidate passed all four, individually, before commit:
`cargo test --all` (2096 passed / 0 failed / 27 ignored, 159 suites),
`cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features
-D warnings`, and `./scripts/run_integration_tests.sh` (136 passed / 0 failed /
24 skipped). `python3 scripts/check_repo_hygiene.py --mode static` exits 0 and
the working tree is clean.
