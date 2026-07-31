# 2026-07-31 — Repository Hygiene and Layout Policy implementation

Implemented the approved design
(`Engineering/designs/2026-07-28-repository-hygiene-design.md`): binding root
policy, machine-readable profile, dependency-free checker, the full initial
migration, and the blocking CI gate.

## What landed

- **Policy**: root `REPOSITORY_HYGIENE.md`, incorporated into `GOVERNANCE.md`
  as §3.8; referenced from `CONTRIBUTING.md`, `README.md`, `Docs/README.md`,
  `Docs/contributing/index.md`, `AGENTS.md`, `CLAUDE.md`. `.cursor` and
  `.jules` files reduced to thin adapters.
- **Enforcement**: `.repo-hygiene.toml` + `scripts/check_repo_hygiene.py`
  (static + working-tree modes) with 27 fixture-tree unit tests
  (`tests/tooling/`), written Red-first (32 tests failing at the Red commit),
  plus a blocking `repo-hygiene` CI job and post-suite working-tree checks.
- **Migration**: 118 static violations at the policy commit burned down to 0.
  Debris deleted (AST/lex dumps — 16 of which embedded a personal Windows home
  directory — an accidentally committed 3.9 MB ELF binary, merge remnants,
  generated reports, dead `crates/wfl_core`, a personal email fixture);
  legacy probes converted to 15 asserted `TestPrograms/{modules,constants,
  nexus}/` programs and 5 `tests/fixtures/` fixtures with
  `tests/diagnostics_fixtures_test.rs`; `Tools/` split into `scripts/`,
  `examples/tools/`, `Engineering/`, `Archive/`; Dev Diary moved to
  `History/dev-diary/2026/`; `Docs/development/` → `Docs/contributing/`;
  active designs/plans → `Engineering/`; 48 retained files indexed in
  `Archive/manifest.json` with sha256 checksums.

## Notable findings along the way

- The old `test_load_module.wfl` probe *expected* `load module` to share a
  container with the caller. It doesn't — load-module scope isolation is
  intended (the interpreter's own diagnostic says to use `include from`), so
  the conversion (`TestPrograms/modules/load_container.wfl`) pins the
  isolation as a negative assertion instead of enshrining the misconception.
- The `Nexus` "even check" probes assumed truncating division;
  `divided by` is exact, so `(n / 2) * 2 == n` is an identity. Pinned in
  `TestPrograms/nexus/even_inline_precedence.wfl` with `modulo` shown as the
  real parity test.
- The analyzer drops one of five `Cannot modify constant` reports when the
  mutations appear in sequence (the `add` form). Filed as **#671**; the
  diagnostics test pins `>= 4` until it's fixed.
- `vscode-extension/package-lock.json` had drifted to 26.7.46 while everything
  else said 26.7.59 — exactly the class of bug the version-drift rule now
  blocks. `bump_version.py` updates both lock version fields from now on.
- Running programs with a debug binary drops `wfl_exec.log` next to the
  program (`execution_logging` defaults on under `debug_assertions`); release
  CI is unaffected, and the working-tree gate will catch any regression.

## Testing (per `testing.md`)

- **Risk class**: R3 (release controls / CI gates touched).
- **Red evidence**: test-only commit `test: add failing repo-hygiene checker
  and installer placement tests` (32 tests, 30 failures + 2 errors, all for
  the intended reason), an ancestor of the Green commits.
- **Layers**: checker unit tests (fixture git trees, clean + per-violation),
  Rust diagnostics fixture tests, 15 asserted WFL end-to-end programs,
  static + working-tree checker runs against the real tree, `cargo fmt`,
  `clippy -D warnings`, `cargo test --all`, docs example validation.
- **Residual risk**: version/packaging consolidation deferred to **#670**
  (needs Windows MSI evidence; mirrors stay drift-checked meanwhile);
  `scripts/test_bump_version.py` mock harness was already broken before this
  change (MagicMock JSON serialization) — the new lock-update logic was
  verified functionally instead; Windows `run_integration_tests.ps1` recursion
  mirrored from the `.sh` runner but exercised only on Linux here.
