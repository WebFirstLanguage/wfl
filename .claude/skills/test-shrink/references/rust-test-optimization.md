# Rust test-suite optimization playbook (WFL profile)

Distilled from external research on compiler/interpreter test architecture in
Rust (matklad's "How to Test" / "Fast Rust Builds", the rustc dev-guide test
best practices, rust-analyzer's and pydantic-monty's suites), applied to this
repo. Read this during Phase 2 (research) and hand the relevant section to the
agent working a candidate. Treat the *claims* here as hypotheses: research
agents should verify current crate/Cargo behavior against live docs before an
implementation is briefed, and every saving still has to show up in Phase 1's
metrics to be kept.

## 1. Test-binary proliferation — likely the single biggest win

Cargo compiles **every `.rs` file directly under `tests/` as its own binary**,
each statically linking the full WFL compiler library with debug info. With
~146 files, that is ~146 near-identical copies of the compiler in
`target/debug/deps/` — this is where most of the "huge debug data" lives, and
why link time dominates test iteration.

**Fix: the single-binary integration pattern.** One entry point (e.g.
`tests/suite/main.rs`) declaring the former files as `mod` submodules:

```rust
// tests/suite/main.rs
mod lexer;
mod parser;
mod typechecker;
mod e2e;
```

One binary, one link, same tests. Migration notes for this repo:

- Migrate **incrementally in batches** (e.g. 10–20 files per candidate), each
  batch its own commit — moving all 146 at once makes review and bisection
  impossible. Cargo ignores subdirectories of `tests/` (except as modules), so
  moved files stop being independent targets the moment they're under
  `tests/suite/`.
- Fix collisions as they surface: duplicate `mod common` imports, duplicate
  helper names across files, `#[path]` attributes, per-file `#![...]` inner
  attributes that must move or become `#[allow]` on the module.
- `cargo test --test <old_name>` granularity is lost; the equivalent is
  `cargo test --test suite <module_name>::` filter strings. Update any docs or
  scripts that name individual test targets (`testing.md`, CI, this skill).
- The existing `tests/common/` sharing mechanism becomes a plain module of the
  suite — usually *simpler* after consolidation.
- Verify with `cargo test --all` after each batch: the same test count must
  run, and no test may silently vanish (compare `cargo test -- --list` counts
  before/after — that is the reviewer's coverage evidence).

Measured effects to expect: `target/debug/deps` shrinks dramatically, full-
suite link time collapses, and per-edit iteration gets faster. Capture
before/after `du -sh target/debug/deps` and suite wall time.

## 2. Profile tuning for test builds — in-scope, with care

`cargo test` builds in the dev profile: `opt-level = 0`, full debug info.
Full debug symbols across ~146 (or even 1) test binaries are most of the disk
cost; `opt-level = 0` makes interpreter-heavy tests slow at runtime.

Candidate settings (verify against current Cargo docs, then measure):

```toml
[profile.test]
opt-level = 1          # cheap runtime win for interpreter-heavy tests
debug = 1              # line-tables-only: keeps usable panics/backtraces
strip = "debuginfo"    # shrink test binaries on disk

# Optimize heavy dependencies harder while keeping WFL code fast to compile:
[profile.test.package."*"]  # or name specific heavy deps (tokio, sqlx, …)
opt-level = 2
```

Hard boundary: **`[profile.release] debug = true` is deliberate** (release
backtraces, per CLAUDE.md) — never change the release profile. Test/dev
profile changes are ordinary candidates *if* the full gate stays green and
panic output in a deliberately-failed test is still readable (check this
explicitly: break a test on purpose, read the output, restore it).

## 3. Kill duplicated harness code with `check`-style drivers

The pattern that keeps rust-analyzer's suite small: tests never touch compiler
internals or re-implement plumbing — they call one shared driver:

```rust
fn check_interpreter(source: &str, expected: &str) {
    // lex → parse → analyze → typecheck → interpret, all in memory
    assert_eq!(run_wfl_source(source), expected);
}
```

Why it shrinks the suite: hundreds of tests collapse to data (input string,
expected output), and internal API changes touch one driver instead of every
test. This also passes the "neural-network test": a suite asserting on
language-visible behavior survives any internal refactor.

For WFL: hunt for repeated spawn-the-binary / build-an-interpreter / tempdir
scaffolding across `tests/*.rs` and hoist it into `tests/common/` (or the
suite root module after consolidation) as `check_*` drivers. Prefer in-memory
drivers over spawning the release binary wherever the test's claim doesn't
require the real CLI boundary — but per the hard rules, a test whose *point*
is the real binary/socket/file must keep the real boundary.

## 4. Snapshot testing — proposal-grade, big payoff for diagnostics

For AST dumps, diagnostics, and other large expected outputs, snapshot crates
beat hand-maintained `assert_eq!` blocks:

- **`expect-test`** (rust-analyzer's): inline `expect![[...]]` snapshots,
  minimal dependency footprint, update via `UPDATE_EXPECT=1 cargo test`.
- **`insta`**: external `.snap` files + `cargo-insta review` TUI; better for
  multi-line diagnostic rendering.
- **`datatest-stable`**: generates a test per file in a directory — maps
  naturally onto a `.wfl`-corpus style suite.

Adding a dev-dependency changes the audited dependency tree, so treat adoption
as a **maintainer-decision proposal** (with measured line-count savings on a
worked example) unless Brad's invocation explicitly green-lights it.

If snapshots are adopted, the known gotchas — bake these into the driver from
day one:

- **Span volatility:** never snapshot raw spans/byte offsets; redact them
  (custom `Debug`, a sanitizer pass, or insta filters), or every whitespace
  edit invalidates the suite.
- **Nondeterminism:** normalize absolute paths to placeholders, strip ANSI
  escapes, force `\n` line endings — or snapshots fail across machines/CI.
- **Parametric collisions:** parameterized tests sharing one function need
  unique snapshot suffixes per case, or cases overwrite each other.

## 5. rustc-suite hygiene rules worth copying (cheap, low-risk)

- **Minimal test programs:** each fixture/test program should contain only
  what its assertion needs — shrink oversized fixtures to minimal repros.
- **Descriptive names over issue numbers:** `issue_12345.rs` →
  `parser_bare_trait_object_issue_12345.rs`; filterable and self-documenting.
- **Suppress unrelated noise at the source:** `#[allow(...)]`/`#[expect(...)]`
  for warnings a test doesn't assert on, so unrelated lint churn doesn't bloat
  captured output or break expectations.

## 6. Environment/tooling — proposals only

Worth proposing to Brad with estimates, but they change the dev/CI
environment, not the tests, so they are never applied by this loop:

- **Faster linkers** (`mold` on Linux, `lld` cross-platform) via
  `.cargo/config.toml` — large link-step speedups, machine-setup dependent.
- **`cargo-nextest`** — per-test process isolation, better scheduling, cleaner
  output; would change the documented test commands in `testing.md`/CI.

## Priority order for a typical pass

1. Binary consolidation batches (§1) — biggest disk + link-time win.
2. Harness dedup into `check` drivers (§3) — biggest source-line win.
3. Fixture minimization + naming + noise suppression (§5) — steady small wins.
4. Test-profile tuning (§2) — one candidate, big disk win, needs the
   readable-panic check.
5. Snapshot adoption + tooling (§4, §6) — written up as proposals.
