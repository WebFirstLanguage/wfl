# Repository Hygiene and Layout Design

**Status:** Approved design
**Owner:** WFL Maintainers
**Approved:** 2026-07-28
**Last reviewed:** 2026-07-28
**Normative:** No. The implemented root `REPOSITORY_HYGIENE.md` will be binding.

## Summary

WFL will adopt a binding repository-hygiene policy, a purpose-based top-level
layout, a full migration of legacy material, and blocking CI enforcement.
Existing Git history will not be rewritten.

The design separates four kinds of written material:

- `Docs/` contains maintained, published user and contributor documentation.
- `Engineering/` contains active designs, plans, evidence, and engineering
  reports.
- `History/` contains chronological, non-normative project history.
- `Archive/` contains retained but inactive material indexed by an archive
  manifest.

The root policy will define where every class of repository content belongs,
what may be tracked, what must remain ephemeral, how exceptions work, and how
CI proves compliance.

## Authority and policy files

- Create root `REPOSITORY_HYGIENE.md`, titled **Repository Hygiene and Layout
  Policy**.
- Incorporate it into `GOVERNANCE.md` §3 as a binding technical policy.
- Reference it from `CONTRIBUTING.md`, `README.md`, `Docs/README.md`,
  `Docs/contributing/index.md`, `AGENTS.md`, and `CLAUDE.md`.
- Create root `.repo-hygiene.toml` as the machine-readable enforcement profile.
  It records the root allowlist, approved output roots, tracked generated-file
  exceptions, permitted binary assets, size exceptions, and experiment
  metadata requirements. The prose policy remains authoritative.
- `AGENTS.md` is the canonical source of shared agent instructions.
  `CLAUDE.md`, `.cursor/`, `.jules/`, and similar tool-specific files may contain
  only discovery adapters and genuine tool-specific deltas. They may not
  redefine governance, testing, compatibility, or placement policy.

## Canonical repository layout

```text
/
├── Archive/                 # retained, inactive, non-normative material
├── Docs/                    # maintained published documentation
├── Engineering/             # active designs, plans, evidence, reports
├── History/                 # chronological project history
├── benches/                 # Criterion benchmarks
├── examples/                # polished, supported WFL examples
├── experiments/             # owned, time-bounded non-gating prototypes
├── fuzz/                    # standalone cargo-fuzz workspace
├── icons/                   # referenced source artwork
├── scripts/                 # maintained automation and workflow entry points
├── src/                     # compiler and runtime
├── TestPrograms/            # executable WFL end-to-end programs
├── tests/                   # Rust tests and consumed fixtures
├── vscode-extension/        # VS Code extension component
├── wfl-lsp/                 # language-server component
├── wix/                     # cargo-wix conventional component
└── root policy, manifest, lock, and tool-required configuration files
```

Stable product components remain recognizable at the root. The migration does
not introduce a component-monorepo reshuffle.

### Placement rules

| Content | Canonical home |
|---|---|
| Compiler/runtime source | `src/` |
| Rust integration/unit tests | `tests/` |
| Fixtures consumed by Rust tests | `tests/fixtures/<feature>/` |
| Executable WFL end-to-end programs | `TestPrograms/<feature>/` |
| Polished user-facing programs | `examples/<topic>/` |
| Non-gating prototypes | `experiments/<topic>/` |
| Benchmarks and fuzz targets | `benches/`, `fuzz/` |
| CI/release/contributor automation | `scripts/` |
| Component-local usage documentation | Beside the component as `README.md` |
| Maintained user/contributor docs | `Docs/` |
| Active designs and architecture records | `Engineering/designs/` |
| Active execution plans and roadmaps | `Engineering/plans/` |
| Test/release evidence | `Engineering/evidence/` |
| Current generated engineering summaries | `Engineering/reports/` only when a tracked exception is approved |
| Dev Diary entries | `History/dev-diary/<year>/` |
| Retired plans, reports, reviews, and notes | `Archive/<kind>/` |
| Builds, test output, reports, packages | `target/` or an OS temporary directory |
| CI-delivered reports/packages | CI artifacts or GitHub Releases |

### Tests, examples, and experiments

- A file belongs in `TestPrograms/` only when the gated runner executes it and
  failure produces a nonzero result. Displaying `PASS` or `FAIL` is not an
  assertion.
- Intentionally invalid programs belong under `tests/fixtures/diagnostics/` and
  require a test that asserts the expected diagnostic and exit status.
- `examples/` contains current, documented, validated programs. Examples may
  not be scratch reproducers or unbounded test servers presented as tests.
- Every `experiments/<topic>/` directory must contain a `README.md` with owner,
  status, linked issue, creation date, review date, and measurable exit
  criteria. An experiment that misses its review date must be promoted,
  archived, or removed.
- GitHub Issues are the live backlog. Markdown TODO inventories and completed
  phase plans are not substitutes for issues.

## What may be tracked

The repository may track:

- Source code, manifests, binding policies, maintained documentation, and
  authoritative configuration.
- Reproducibility locks used by supported workflows: root `Cargo.lock`,
  `fuzz/Cargo.lock`, and `vscode-extension/package-lock.json`.
- Source assets referenced by the product or packaging, such as
  `icons/wfl.png`.
- Fixtures, expected outputs, and snapshots that a named automated test
  consumes.
- A generated file only when `.repo-hygiene.toml` records its generator,
  consumer, owner, justification, drift-check command, and deterministic
  regeneration rule.
- Vendored material only by explicit Maintainer approval with license,
  upstream source, immutable hash, update procedure, and owner recorded.

## What must not be tracked

The repository must not track:

- Build directories, executables, object files, release archives, installers,
  or extension packages.
- Logs, caches, debug output, compiler dumps, raw coverage output, temporary
  files, mutable validation caches, or timestamp-only reports.
- Merge/recovery remnants such as `*.orig` and `*.rej`.
- Local permissions, local agent settings, editor state, credentials, secrets,
  machine-specific configuration, or personal absolute paths.
- Test-created files outside the approved `target/test-artifacts/` or
  harness-provided temporary directory.
- Alternate source copies, disconnected crates, or “will replace this later”
  implementation files.
- Stale status reports or TODO documents presented as current project state.
- Generated content without a declared consumer and deterministic drift check.

Ignoring an artifact is necessary but not sufficient. Tests and tools must
write it to an approved output root rather than littering source directories.

## Output and generated-file policy

- Tests write under a harness-created temporary directory or
  `target/test-artifacts/<suite>/`; the harness removes the output on success
  and failure.
- Developer and validation reports write to `target/reports/<tool>/`.
- Packaging stages exact release inputs under `target/package/<platform>/`.
- CI uploads reports and packages with an explicit retention period; it does
  not commit them.
- Golden fixtures use repository-relative normalized paths and stable content.
  Timestamps, usernames, home directories, random identifiers, and platform
  separators must be normalized unless they are the behavior under test.
- A tracked generated file is an exception, not a default. CI regenerates it
  and fails on drift.

## Single sources of truth

### Product version

- Root `Cargo.toml` `[package].version` is the sole WFL product-version
  authority.
- Runtime version reporting uses `env!("CARGO_PKG_VERSION")`; retire the
  hand-maintained `src/version.rs` constant.
- Retire `.build_meta.json`.
- Remove `[package.metadata.bundle].version`; the bundle inherits the Cargo
  package version. WiX receives the Cargo version through its build input.
  The npm manifest/lock versions are unavoidable format-specific mirrors, so
  the bump tool generates and checks them.
- `scripts/bump_version.py` updates the Cargo version and regenerates required
  mirrors and lockfiles atomically.
- VS Code `package.json`, both version fields in `package-lock.json`, VSIX
  contents, MSI ProductVersion, and release filenames must agree with the
  canonical version.
- `wfl-lsp` remains an independently SemVer-versioned component using
  `wfl-lsp/Cargo.toml`. It is not a WFL product-version mirror. Packaging records
  both versions, and the VS Code compatibility range must accept the bundled
  LSP version.

### Formatting and configuration

- Keep `rustfmt.toml` as the sole root Rust formatting file, set it to edition
  2024, and delete `.rustfmt.toml`.
- Tool-required configuration stays in its conventional location only when a
  current workflow consumes it.
- Configuration summaries in agent files and READMEs are non-authoritative
  pointers and are checked for obvious version/path drift.

## Documentation lifecycle

### Maintained documentation

- `Docs/` describes what ships now and remains part of normal documentation
  navigation and validation.
- Contributor material currently under `Docs/development/` moves to
  `Docs/contributing/`.
- Current technical contracts move to `Engineering/designs/`.
- Active phased work moves to `Engineering/plans/`.
- Completed, abandoned, superseded, deferred-without-owner, or candidate-only
  records move to `Archive/`.

### History

`Dev diary/YYYY-MM-DD-*.md` moves to
`History/dev-diary/<year>/YYYY-MM-DD-*.md`.

`History/dev-diary/README.md` states that entries are chronological,
non-normative accounts. Entries are not silently rewritten; corrections use a
dated addendum. Current behavior and policy come from root policy, maintained
docs, code, and tests.

### Archive

- Broadly retain human-authored material that has historical, evidentiary, or
  design value.
- Do not archive reproducible machine debris merely to avoid deleting it.
- Do not archive secrets, local permissions, unnecessary personal data, or
  embargoed vulnerability information into the public repository.
- Preserve archived content byte-for-byte where practical. Put corrections and
  current status in metadata or a new addendum.
- `Archive/README.md` prominently states that archive content is non-normative
  and excluded from maintained documentation navigation.
- `Archive/manifest.json` indexes every retained file with:
  `path`, `original_path`, `source_commit`, `sha256`, `document_date`,
  `archived_at`, `kind`, `status_at_archive`, `reason`, `normative: false`,
  `superseded_by`, `related_issues_or_prs`, and `security_classification`.
- Exact duplicates may be deduplicated after matching hashes if the manifest
  retains every original path.

## Initial migration

### Delete reproducible or unsafe debris

Delete from the tracked tree:

- `compare_search_full_clean`.
- All tracked `*.ast.txt` and `*.lex.txt` dumps.
- `docs_code_blocks_report.json`, `validation_report.json`, and the mutable docs
  validation cache.
- `src/linter/mod.rs.orig`, `src/linter/mod.rs.rej`, the raw Clippy log, and
  runtime debug reports.
- Generated `test_rust_files/` content and `google_index.html`.
- `Nexus/.claude/settings.local.json`.
- The stale tracked VSIX after all packaging consumers use the freshly staged
  package.
- Dead `crates/wfl_core/`.
- Duplicate/conflicting formatting and obsolete version-mirror configuration
  after their consumers are migrated.

Delete ignored debris already produced at the root, then change its producers
so a clean checkout remains clean after every gated suite.

### Reclassify legacy WFL programs

Apply these exact dispositions:

| Current path | Disposition |
|---|---|
| `blog_server.wfl` | Move to `examples/web/blog_server.wfl`; polish and validate. |
| `blog_server_minimal.wfl` | Archive under `Archive/legacy-programs/root/`. |
| `debug_split.wfl` | Archive under `Archive/legacy-programs/root/`; current split coverage supersedes it. |
| `test_container.wfl` | Move to `tests/fixtures/modules/person_container.wfl`. |
| `test_include.wfl` | Convert to asserted `TestPrograms/modules/include_container.wfl`. |
| `test_load_module.wfl` | Convert to asserted `TestPrograms/modules/load_container.wfl`. |
| `test_fix_verification.wfl` | Move to `tests/fixtures/modules/export_and_top_level_return.wfl`. |
| `test_main_fix.wfl` | Convert to asserted `TestPrograms/modules/include_top_level_return.wfl`. |
| `test_export_validation.wfl` | Move to `tests/fixtures/diagnostics/export_non_constant.wfl` and add a diagnostic/exit-status test. |
| `Webserver_test/server.wfl` | Move to `examples/web/html_server.wfl`, remove personalized wording, and validate. |
| `LabsTest/email.wfl` | Remove from the current tree; do not duplicate its personal identifier in `Archive/`. |
| `Nexus/nexus.wfl` | Move to `experiments/nexus/nexus.wfl` with required experiment metadata. |
| `Nexus/test_action_syntax.wfl` | Convert to asserted `TestPrograms/nexus/action_syntax.wfl`. |
| `Nexus/test_concat.wfl` | Convert to asserted `TestPrograms/nexus/numeric_concat.wfl`. |
| `Nexus/test_factorial_inline.wfl` | Convert to asserted `TestPrograms/nexus/factorial_inline_precedence.wfl`. |
| `Nexus/test_factorial_parens.wfl` | Convert to asserted `TestPrograms/nexus/factorial_parenthesized_precedence.wfl`. |
| `Nexus/test_inline_even.wfl` | Convert to asserted `TestPrograms/nexus/even_inline_precedence.wfl`. |
| `Nexus/test_nested_loops.wfl` | Convert to asserted `TestPrograms/nexus/nested_loop_control.wfl`. |
| `Nexus/test_simple_call.wfl` | Convert to asserted `TestPrograms/nexus/action_value_call.wfl`. |
| `Nexus/test_skip_loop.wfl` | Convert to asserted `TestPrograms/nexus/loop_skip.wfl`. |
| `Nexus/test_zero_concat.wfl` | Convert to asserted `TestPrograms/nexus/concat_zero_regression.wfl`. |
| Remaining `Nexus/*.wfl` partial/minimal probes | Archive under `Archive/legacy-programs/nexus/`. |
| `syntax_test/pattern.wfl` | Move to `experiments/syntax/pattern_kitchen_sink.wfl` with required experiment metadata. |
| `syntax_test/test_constant_immutability.wfl` | Move to `tests/fixtures/diagnostics/constant_immutability.wfl` and add a diagnostic test. |
| `syntax_test/test_error_messages.wfl` | Move to `tests/fixtures/diagnostics/constant_error_precedence.wfl` and add a diagnostic test. |
| `syntax_test/test_deprecated_constant.wfl` | Convert to asserted `TestPrograms/constants/deprecated_syntax_compat.wfl`. |
| `syntax_test/test_new_constant.wfl` | Convert to asserted `TestPrograms/constants/current_syntax.wfl`. |
| `syntax_test/test_variables_vs_constants.wfl` | Convert to asserted `TestPrograms/constants/mutable_and_constant_variables.wfl`. |
| Other human-authored `syntax_test/` probes/notes | Archive under `Archive/legacy-programs/syntax/`. |
| `test_rust_files/` and all legacy dumps/logs | Delete as generated output. |

Before adding a converted micro-repro, search for equivalent asserted coverage. If an
equivalent gated test already exists, archive the legacy source instead of adding a
duplicate test. This is a deterministic deduplication rule, not permission to drop
coverage.

### Reclassify tools

- Remove `Tools/` after applying this split.
- Retire `Tools/launch_msi_build.py` after
  `scripts/build_windows_installer.ps1` incorporates its maintained options.
  Replace its tests with `tests/tooling/windows_installer_test.py`.
- Retire `Tools/wfl_config_checker.py`; the shipped `--configCheck` and
  `--configFix` CLI paths are authoritative.
- Move `Tools/rust_loc_counter.py` to
  `scripts/metrics/generate_rust_loc_report.py`.
- Keep the full WFL line counter as `examples/tools/rust_loc_counter.wfl`;
  archive `line_counter.wfl` and `rust_loc_counter_simple.wfl` as superseded
  examples.
- Move `Tools/wfl_md_combiner.py` to
  `scripts/docs/combine_markdown.py`.
- Move `Tools/wfl_combiner.wfl` to
  `examples/tools/combine_markdown.wfl`.
- Move consumed recursive/file-list inputs to
  `tests/fixtures/tooling/combiner/`.
- Move `Tools/rust_loc_counter_spec.md` to
  `Engineering/components/rust_loc_counter.md`.
- Move `Tools/msi_enhancement_tdd_plan.md` to
  `Archive/implementation-plans/windows-installer/`.
- Delete the raw Clippy output and lexer dump.
- Replace `Tools/README.md` with focused `scripts/README.md` and
  `examples/tools/README.md`.

### Reclassify written records

- Move the Dev Diary to `History/dev-diary/2026/`.
- Move active type-system and route designs to `Engineering/designs/`.
- Move the active concurrency plan to `Engineering/plans/`.
- Move completed Superpowers plans, stale documentation TODOs, generated
  snapshots, old root bug reports, component reports, and retired designs into
  categorized `Archive/` paths.
- Move `Docs/Archive/wflpkg/` to `Archive/retired-systems/wflpkg/`.
- Security-triage the already-public WFLHASH review files before archiving them;
  do not publish unresolved private vulnerability details.
- Promote unique durable lessons out of `.jules/bolt.md` into `History/`, then
  replace it with a thin Jules adapter to canonical root policy.
- Replace stale `.cursor` policy content with a thin adapter to canonical root
  policy.

### Packaging

- Keep `wix/` in its conventional root location and preserve WiX UpgradeCode
  and component identities.
- Retire root `wix.toml`; move any still-required package settings into
  `Cargo.toml` package metadata or `wix/main.wxs`, with characterization tests
  proving the effective cargo-wix inputs before removal.
- Move `build_msi.ps1` to `scripts/build_windows_installer.ps1`.
- Use that entry point for local and nightly packaging.
- Generate the VSIX during packaging and stage it at
  `target/package/windows/vscode-wfl.vsix`.
- Update WiX and extension-install consumers atomically before deleting the
  tracked VSIX.
- Verify that MSI ProductVersion, release filenames, the WFL CLI, and the
  embedded VSIX carry the canonical product version. Record the independently
  versioned bundled LSP and prove that the extension accepts it.

## Enforcement

Create a dependency-free `scripts/check_repo_hygiene.py` with two modes.

### Static mode

Static mode fails on:

- A root entry absent from the allowlist.
- Forbidden tracked suffixes, file magic, local settings, or generated
  artifacts.
- Undeclared binary or oversized files.
- Personal absolute paths in tracked content outside an explicitly approved
  portability fixture.
- Missing generated-file exception metadata or regeneration drift.
- Missing/invalid archive manifest entries or checksum drift.
- Missing experiment metadata or expired experiment reviews.
- Multiple Rust formatting configurations.
- Product-version drift across Cargo, npm, runtime, VSIX, MSI, and locks.
- Disconnected workspace directories or known retired paths.

### Working-tree mode

After generators and test suites, working-tree mode fails on:

- Modified tracked files.
- New untracked files outside approved output roots.
- New ignored files outside approved build, dependency, report, package, or
  test-artifact roots.
- Test or tool output written into source, documentation, fixture, or root
  locations.

### CI integration

- Add an early Linux/Windows `repo-hygiene` matrix job to
  `.github/workflows/ci.yml`.
- Make downstream merge and version-bump jobs depend on it.
- Run working-tree mode after Rust tests, WFL integration programs, web tests,
  docs validation, extension tests, and packaging tests.
- Every workflow that writes directly to the repository must run static and
  version checks immediately before its commit/push.
- Fix `vscode-extension/.gitignore`, recursive local-settings ignores, VS Code
  negations, and the case-insensitive `Icon?` rule.
- Keep the checker blocking only after the migration branch itself is fully
  compliant; main receives the policy, migration, and blocking gate together.

## Testing and rollout

The implementation is **R3** under `testing.md` because it changes release
controls and installer/extension packaging. It requires independent review,
failure-path coverage, complete presubmit, and recovery/upgrade evidence.

Use one reviewable migration branch and pull request with structured commits:

1. This R0 approved design record.
2. Failing checker, version-drift, archive-manifest, output-cleanliness, and
   packaging tests with auditable Red evidence.
3. Root policy, machine-readable profile, and checker implementation.
4. Artifact removal, path migration, generator/test-output fixes, and reference
   updates.
5. Version-source and packaging consolidation.
6. Blocking CI integration, maintained documentation updates, and final
   evidence record.

Required verification includes:

- Checker unit tests using clean and deliberately violating fixture trees.
- Root allowlist, magic-byte, personal-path, generated-exception, archive
  checksum, experiment-expiry, version-drift, and post-test cleanliness cases.
- `cargo metadata --no-deps` and locked root/fuzz dependency checks.
- `cargo fmt --all -- --check`.
- `cargo clippy --all-targets --all-features -- -D warnings`.
- `cargo test --all --verbose`.
- Release build plus official integration and web runners.
- Docs example validation and link/path checks.
- `npm ci`, extension compile/lint/tests, VSIX creation, and inspection of its
  embedded version.
- MSI build and file-table inspection plus clean install, optional-extension
  install, upgrade, uninstall, PATH/config behavior, and artifact-version
  checks on Windows.
- A final clean-tree check after every applicable suite.

## Acceptance criteria

- Every tracked path has one documented purpose and canonical home.
- The repository root contains only allowlisted policies, manifests,
  configuration, and stable purpose-based directories.
- No tracked build, dump, cache, log, package, local-settings, merge-remnant, or
  unconsumed report artifacts remain.
- All retained archive content is indexed and clearly non-normative.
- Current docs, active engineering records, history, and archive content are
  structurally distinct.
- One product-version source drives runtime and distributed artifacts without
  drift.
- All existing path references, includes, manifests, workflows, documentation
  links, and packaging inputs resolve after migration.
- Gated tests and generators leave the checkout clean outside approved output
  roots.
- The hygiene checker blocks regressions on Linux and Windows.
- Existing WFL language behavior and public programs remain backward
  compatible.
- Existing Git history, tags, forks, and commit identifiers remain unchanged.
