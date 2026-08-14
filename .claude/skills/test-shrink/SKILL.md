---
name: test-shrink
description: Run the WFL test-suite shrink loop — an iterative, subagent-driven optimization pass that reduces the size of the Rust unit/integration tests and the debug data they generate, while keeping every test and every TestPrograms/ program passing. Use this whenever the maintainer asks to shrink, slim, optimize, or reduce the tests, test output, test debug data, test bloat, or test disk usage, or says things like "run the test shrink loop", "make the tests smaller", "the tests generate too much debug data", or "optimize the unit tests". This is an occasional maintenance pass, not part of feature work.
---

# WFL Test Shrink Loop

An iterative optimization pass over the WFL test suite. The goal is to make the
tests **smaller** — less debug data generated, fewer bytes on disk, less
duplicated harness code, faster runs — **without losing one bit of what they
verify**. Coverage is the product here; size is the cost. This loop only ever
reduces cost.

The loop is: **measure → research → shrink one candidate → review → verify →
keep or revert → repeat** until gains dry up. Subagents do the heavy lifting:
`research-agent` finds candidates, `code-monkey` implements them one at a time,
`code-reviewer` guards coverage before anything is kept.

## Hard rules (read before anything else)

These come from the binding Logbie Testing Policy (root `testing.md`) and
`GOVERNANCE.md`. A shrink that violates one of these is a regression, not an
optimization — revert it.

1. **Never weaken what a test verifies.** No deleted assertions, no relaxed
   tolerances, no `#[ignore]`, no removed negative/failure-path checks. Two
   tests may be merged only when the merged test provably makes every
   assertion of both — the code-reviewer must confirm this from the diff.
2. **Never delete a test file** unless it is demonstrably a strict duplicate
   of another (same inputs, same assertions) and the reviewer confirms it.
   When in doubt, keep it and shrink something else.
3. **All gates stay green.** `cargo fmt --all -- --check`,
   `cargo clippy --all-targets --all-features -- -D warnings`,
   `cargo test --all`, and the full `TestPrograms/` integration run must pass
   after every accepted change. A change that can't be verified is reverted,
   not "probably fine".
4. **Backward compatibility is out of scope.** This loop touches `tests/`,
   `tests/common/`, `tests/fixtures/`, and test-output plumbing. It does not
   change `src/` behavior. If a real product improvement surfaces (e.g. a
   runtime flag to silence trace output), record it as a **proposal** in the
   final report instead of implementing it here.
5. **Maintainer-decision items are flagged, not applied.** `[profile.release] debug = true`
   is deliberate (release backtraces) — never change the release profile. New
   dev-dependencies (e.g. snapshot crates), CI workflow files, linker/tooling
   config, and `.repo-hygiene.toml` also go in the report's "Proposals for
   Brad" section instead of being applied. **Test-profile tuning
   (`[profile.test]`) is an ordinary candidate** — see the playbook's rules,
   including the readable-panic check. One carve-out: when an accepted
   candidate renames or moves test targets (e.g. suite consolidation), the
   **mechanical** updates to references to those target names — in
   `testing.md`, `scripts/`, and CI workflow files — are part of that same
   candidate, because the gates cannot stay green without them. Substantive
   CI changes (new jobs, runners, tools) remain proposals.
6. **Hygiene:** all measurement scratch and reports go under
   `target/reports/test-shrink/` or the session temp dir — never into the tree.

## Phase 0 — Preflight

1. **Disk check** (per CLAUDE.md): a full build needs ~30 GB of `target/`.
   ```bash
   [ "$(df -BG --output=avail . | tail -1 | tr -dc '0-9')" -lt 30 ] && cargo clean
   ```
2. **Green baseline.** Run the full gate once:
   `cargo test --all`, then `cargo build --release` and
   `./scripts/run_integration_tests.sh` (`.ps1` on Windows — wherever this
   skill says `.sh`, use the platform's variant). If anything is red **stop** — report
   the failure instead of optimizing on a broken base. Shrinking on red makes
   it impossible to tell whether a shrink broke something.
3. **Working branch.** Do this work on a dedicated branch (or the branch the
   session was given), one conventional commit per accepted candidate
   (`test: ...` or `refactor(tests): ...`), so any single shrink can be
   reverted later without unwinding the rest.

## Phase 1 — Measure the baseline

"Small" must be a number or the loop can't tell whether it's winning. Capture
these into `target/reports/test-shrink/baseline.md` (and re-capture the same
way at the end):

| Metric | How |
|---|---|
| Test source size | `find tests -name '*.rs' -print0 \| xargs -0 wc -l \| tail -1`; `du -sh tests/` (not a bare `tests/**/*.rs` glob — without globstar it silently skips the top-level files, which is most of the suite) |
| Fixture size | `du -sh tests/fixtures/` |
| Debug/output volume | `cargo test --all 2>&1 \| wc -c` (bytes the suite prints) |
| On-disk artifacts | `du -sh target/test-artifacts/` before vs after a run; list any stray files a run drops elsewhere (hygiene violations — fix on sight) |
| Wall time | time of `cargo test --all` and of the integration script |
| Test binary footprint | `du -sh target/debug/deps/` and count of test binaries — with ~146 separate files under `tests/`, each is its own binary statically linking the whole compiler with debug info; this is usually where most of the "debug data" lives. **Compare like with like:** Cargo never deletes a removed target's old binaries, so a dirty `deps/` diff shows no saving (or growth) after consolidation — measure from equivalent clean states, or inventory only the current target graph's artifacts (`cargo test --no-run --message-format=json` lists them) |

Not every metric must improve every run — but no metric may get *worse* in an
accepted change without an explicit reason in its commit message.

## Phase 2 — Research (parallel subagents)

**First, read `references/rust-test-optimization.md`** — the distilled
playbook of Rust compiler-testing research (single-binary consolidation,
`check` drivers, profile tuning, snapshot testing, rustc hygiene rules) with
WFL-specific migration notes and a suggested priority order. Hand each
research agent the section for its hunting ground; treat the playbook's claims
as hypotheses to verify and measure, not facts.

Spawn **research-agent** subagents in parallel, one per hunting ground, each
returning a ranked candidate list (what, where, estimated saving, risk,
suggested approach). Hunting grounds that historically pay off in this repo:

- **Binary proliferation (playbook §1):** every `.rs` file directly under
  `tests/` becomes its own statically-linked test binary. Consolidating into a
  single suite binary in incremental batches is usually the biggest disk and
  link-time win available.
- **Noise:** `println!`/`eprintln!`/`dbg!` and captured `exec_trace!` output in
  test code; tests that run the binary with verbose flags they don't assert on;
  overly chatty failure messages built eagerly (`format!` on the hot path).
- **Duplication (playbook §3):** the same spawn-server / run-wfl-file /
  temp-dir harness re-implemented across files — hoist into `tests/common/`
  as `check`-style drivers. With ~146 test files this is usually the biggest
  source-line win.
- **Redundancy:** tests that assert a strict subset of another test's
  assertions on the same inputs (merge, carefully — rule 1).
- **Fixtures (playbook §5):** oversized `.wfl` fixtures where a minimal
  program exercises the same path; generated fixtures that could be built in
  the test instead of checked in; non-descriptive issue-number test names.
- **Artifacts:** test output written outside `target/test-artifacts/<suite>/`
  or temp dirs; artifacts never cleaned up; debug dumps written even on pass.
- **Profiles (playbook §2):** `[profile.test]` debug-info and opt-level
  tuning — ordinary candidate, but never the release profile, and always with
  the readable-panic check.
- **Rust-level:** anything beyond the playbook that a current-best-practices
  search turns up for shrinking Rust test debug data (e.g. capturing output
  instead of printing, lazy `assert!` messages, splitting mega-tests). The
  research agent may use web search, and should verify the playbook's claims
  against current docs while it's there.

Merge the lists, de-duplicate, and order the worklist **highest saving × lowest
risk first**. Cap a single run's worklist at ~10 candidates — this skill runs
occasionally; leftover candidates go in the report for next time.

## Phase 3 — The shrink loop (serial, one candidate at a time)

For each candidate, in order:

1. **Implement — spawn `code-monkey`** with a tight brief: the exact files,
   the exact change, the hard rules above verbatim, and the targeted test
   command for the affected area (e.g. `cargo test --test <name>`). One
   candidate per agent; small focused diffs are what make step 3 meaningful.
2. **Review — spawn `code-reviewer`** on the complete change: run
   `git add -A` first, then have it review `git status --short` plus
   `git diff --cached HEAD` — a plain `git diff` omits newly created files
   and staged-only changes (e.g. a new shared helper, or `git mv`s during
   consolidation), letting a KEEP through without the reviewer ever seeing
   the code that decides coverage. Its one non-negotiable question: *does
   the suite still verify everything it verified before this diff?* It
   answers KEEP / REVISE / REJECT with evidence. REVISE goes back to a
   code-monkey once; REJECT means revert now.
3. **Verify.** Targeted tests first (fast feedback), then before committing:
   `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`,
   `cargo test --all`. If the change touched anything the end-to-end programs
   exercise (fixtures, harness, binary invocation), also rerun
   `./scripts/run_integration_tests.sh`.
4. **Keep or revert.** Green → commit (one candidate, one commit, message
   states the measured saving). Anything red or reviewer-rejected → restore
   tracked files (`git restore --staged --worktree -- .`) **and** delete any
   files the attempt created with `git clean -fd` scoped to the paths the
   candidate touched (e.g. `git clean -fd -- tests/`) — `git restore` alone
   leaves new untracked files behind, silently polluting the next attempt.
   Never carry a broken candidate forward while starting the next one.

Stop the loop when: the worklist is empty, **or** the last 3 candidates were
all rejected/reverted, **or** remaining candidates are estimated under ~1% of
any metric. Diminishing returns are the expected exit — this skill gets run
again another day.

## Phase 4 — Wrap up

1. Re-run the **full** gate one final time, including
   `./scripts/run_integration_tests.sh` — every TestPrograms/ program must
   pass. This is the promise the skill makes.
2. Re-measure Phase 1's metrics; write the before/after table.
3. `python3 scripts/check_repo_hygiene.py --mode static` (structural changes
   were made).
4. Add a Dev Diary entry `History/dev-diary/<year>/<date>-test-shrink-pass.md`
   summarizing: metrics before/after, candidates accepted (with commits),
   candidates rejected and why, and the **Proposals for Brad** list
   (maintainer-decision items from rule 5 and unimplemented `src/` ideas).
5. Commit the diary entry, push the branch, and report the before/after
   numbers as the headline.

## Subagent quick reference

| Agent | Role | Mode |
|---|---|---|
| `research-agent` | find & rank shrink candidates; may web-search Rust practices | parallel, read-only |
| `code-monkey` | implement exactly one candidate, run targeted tests | serial |
| `code-reviewer` | guard coverage; KEEP / REVISE / REJECT with evidence | serial, read-only |

Give every subagent the hard-rules block verbatim in its prompt — subagents do
not see this file unless told, and the rules are the contract that keeps this
loop safe to run unattended.
