# Repository Hygiene and Layout Policy

**Status:** Binding technical policy (incorporated by `GOVERNANCE.md` §3)
**Owner:** WFL Maintainers (primary: Brad, Logbie LLC)
**Approved design:** `Engineering/designs/2026-07-28-repository-hygiene-design.md`
**Machine-readable profile:** `.repo-hygiene.toml`
**Enforcement:** `scripts/check_repo_hygiene.py` (blocking `repo-hygiene` CI job)

This policy defines where every class of repository content belongs, what may
be tracked, what must remain ephemeral, how exceptions work, and how CI proves
compliance. The prose here is authoritative; `.repo-hygiene.toml` records the
concrete allowlists and approved exceptions the checker enforces. Existing Git
history is never rewritten for hygiene reasons.

## 1. Four kinds of written material

| Directory | Contents | Normative? |
|---|---|---|
| `Docs/` | Maintained, published user and contributor documentation — what ships **now** | Yes |
| `Engineering/` | Active designs (`designs/`), execution plans (`plans/`), test/release evidence (`evidence/`), component records (`components/`) | Yes, while active |
| `History/` | Chronological project history, including the Dev Diary (`History/dev-diary/<year>/`) | No |
| `Archive/` | Retained but inactive material, indexed by `Archive/manifest.json` | No |

`Archive/` and `History/` content is never silently rewritten; corrections use
a dated addendum. Current behavior and policy always come from root policy,
maintained docs, code, and tests — never from history or archive.

## 2. Canonical layout and placement rules

Stable product components stay recognizable at the root: `src/`, `tests/`,
`TestPrograms/`, `examples/`, `experiments/`, `benches/`, `fuzz/`, `scripts/`,
`icons/`, `vscode-extension/`, `wfl-lsp/`, `wix/`, plus the four material
directories above and root policy/manifest/configuration files. The root
allowlist in `.repo-hygiene.toml` is exhaustive; adding a root entry requires
updating it (and passing review of why the content belongs at the root).

| Content | Canonical home |
|---|---|
| Compiler/runtime source | `src/` |
| Rust integration/unit tests | `tests/` |
| Fixtures consumed by Rust tests | `tests/fixtures/<feature>/` |
| Executable WFL end-to-end programs | `TestPrograms/<feature>/` (or top level) |
| Polished user-facing programs | `examples/<topic>/` |
| Non-gating prototypes | `experiments/<topic>/` |
| Benchmarks and fuzz targets | `benches/`, `fuzz/` |
| CI/release/contributor automation | `scripts/` |
| Component-local usage documentation | Beside the component as `README.md` |
| Maintained user/contributor docs | `Docs/` (contributor material: `Docs/contributing/`) |
| Active designs and architecture records | `Engineering/designs/` |
| Active execution plans and roadmaps | `Engineering/plans/` |
| Test/release evidence | `Engineering/evidence/` |
| Dev Diary entries | `History/dev-diary/<year>/` |
| Retired plans, reports, reviews, notes | `Archive/<kind>/` |
| Builds, test output, reports, packages | `target/` or an OS temporary directory |
| CI-delivered reports/packages | CI artifacts or GitHub Releases |

### Tests, examples, and experiments

- A file belongs in `TestPrograms/` only when the gated runner executes it and
  failure produces a nonzero exit. Displaying `PASS`/`FAIL` text is not an
  assertion.
- Intentionally invalid programs belong under `tests/fixtures/diagnostics/`
  with a Rust test asserting the expected diagnostic and exit status.
- `examples/` contains current, documented, validated programs — not scratch
  reproducers.
- Every `experiments/<topic>/` directory must contain a `README.md` with
  `Owner`, `Status`, `Issue`, `Created`, `Review-by`, and `Exit criteria`
  fields. An experiment past its `Review-by` date must be promoted, archived,
  or removed — the checker fails on expiry.
- GitHub Issues are the live backlog. Markdown TODO inventories and completed
  phase plans are not substitutes for issues.

## 3. What may be tracked

- Source code, manifests, binding policies, maintained documentation, and
  authoritative configuration.
- Reproducibility locks used by supported workflows: root `Cargo.lock`,
  `fuzz/Cargo.lock`, `vscode-extension/package-lock.json`.
- Source assets referenced by the product or packaging (e.g. `icons/wfl.png`),
  declared in the profile's `[binaries]` allowlist.
- Fixtures, expected outputs, and snapshots that a named automated test
  consumes.
- A generated file **only** when `.repo-hygiene.toml` records its generator,
  consumer, owner, justification, drift-check command, and deterministic
  regeneration rule (a `[[generated]]` entry).
- Vendored material only by explicit Maintainer approval with license,
  upstream source, immutable hash, update procedure, and owner recorded.

## 4. What must not be tracked

- Build directories, executables, object files, release archives, installers,
  or extension packages.
- Logs, caches, debug output, compiler dumps (`*.ast.txt`, `*.lex.txt`), raw
  coverage output, temporary files, mutable validation caches, or
  timestamp-only reports.
- Merge/recovery remnants (`*.orig`, `*.rej`).
- Local permissions, local agent settings (`settings.local.json`), editor
  state, credentials, secrets, machine-specific configuration, or personal
  absolute paths (documentation may use the generic placeholder users listed
  in the profile).
- Test-created files outside `target/test-artifacts/` or a harness-provided
  temporary directory.
- Alternate source copies, disconnected crates, or "will replace this later"
  implementation files.
- Stale status reports or TODO documents presented as current project state.
- Generated content without a declared consumer and deterministic drift check.

Ignoring an artifact is necessary but not sufficient: tests and tools must
write to an approved output root rather than littering source directories.

## 5. Output and generated-file policy

- Tests write under a harness-created temporary directory or
  `target/test-artifacts/<suite>/`; output is removed on success and failure.
- Developer and validation reports write to `target/reports/<tool>/`.
- Packaging stages exact release inputs under `target/package/<platform>/`.
- CI uploads reports and packages with explicit retention; it never commits
  them.
- Golden fixtures use repository-relative normalized paths and stable content.
  Timestamps, usernames, home directories, random identifiers, and platform
  separators are normalized unless they are the behavior under test.
- A tracked generated file is an exception, not a default; its `[[generated]]`
  entry names the drift check that CI runs.

## 6. Single sources of truth

### Product version

Root `Cargo.toml` `[package].version` is the sole WFL product-version
authority. `scripts/bump_version.py` updates it and regenerates every mirror
atomically; the checker's version-drift rule fails CI when any declared source
disagrees.

**Transitional state (explicitly not final):** `src/version.rs`,
`.build_meta.json`, and the `wix.toml` version line are still hand-tracked
mirrors consumed by the release pipeline, and the pinned
`vscode-extension/vscode-wfl-0.1.0.vsix` is still consumed by the MSI
optional-extension custom action. Each is declared as a `[[generated]]`
exception in the profile. Retiring them — runtime via
`env!("CARGO_PKG_VERSION")`, nightly via `Cargo.toml`, packaging via a VSIX
staged at `target/package/windows/` — is tracked follow-up work that requires
Windows packaging evidence (see the approved design §"Single sources of
truth" and the follow-up issue referenced there). Until then the drift rule
keeps every mirror in agreement.

`wfl-lsp` remains an independently SemVer-versioned component
(`wfl-lsp/Cargo.toml`); it is not a product-version mirror.

### Formatting and configuration

- `rustfmt.toml` is the sole Rust formatting configuration; a second tracked
  rustfmt file fails the checker.
- Tool-required configuration stays in its conventional location only while a
  current workflow consumes it.
- Configuration summaries in agent files and READMEs are non-authoritative
  pointers. `CLAUDE.md` is the canonical source of shared agent instructions;
  `AGENTS.md`, `.cursor/`, `.jules/`, and similar tool-specific files carry
  only discovery adapters and genuine tool-specific deltas, and may not
  redefine governance, testing, compatibility, or placement policy.

## 7. Archive rules

- Broadly retain human-authored material with historical, evidentiary, or
  design value; do not archive reproducible machine debris merely to avoid
  deleting it.
- Never archive secrets, local permissions, unnecessary personal data, or
  embargoed vulnerability information into the public repository.
- Preserve archived content byte-for-byte where practical; put corrections in
  metadata or a dated addendum.
- `Archive/README.md` states that archive content is non-normative and
  excluded from maintained documentation navigation.
- `Archive/manifest.json` indexes every retained file with: `path`,
  `original_path`, `source_commit`, `sha256`, `document_date`, `archived_at`,
  `kind`, `status_at_archive`, `reason`, `normative: false`, `superseded_by`,
  `related_issues_or_prs`, and `security_classification`. The checker verifies
  presence, required fields, and checksums.
- Exact duplicates may be deduplicated after matching hashes if the manifest
  retains every original path.

## 8. Enforcement

`scripts/check_repo_hygiene.py` (dependency-free, Python 3.11+ stdlib) has two
modes:

- **Static mode** (`--mode static`) fails on: root entries absent from the
  allowlist; forbidden tracked suffixes, names, or retired paths; undeclared
  binary or oversized files; personal absolute paths outside approved
  portability fixtures; incomplete generated-file exception metadata;
  missing/invalid archive manifest entries or checksum drift; missing
  experiment metadata or expired reviews; multiple rustfmt configurations; and
  product-version drift across the declared sources.
- **Working-tree mode** (`--mode working-tree`) runs after generators and test
  suites and fails on modified tracked files, or new untracked/ignored files
  outside the approved output roots.

CI runs static mode early as the blocking `repo-hygiene` job and working-tree
mode after the test suites; workflows that commit to the repository run the
checks immediately before pushing. The checker's own unit tests live in
`tests/tooling/`.

## 9. Exceptions

Every exception is Maintainer-approved and recorded in `.repo-hygiene.toml`
next to the rule it relaxes, with enough metadata for a stranger to understand
why it exists and how it is kept honest. Widening an allowlist to silence a
violation without that approval is itself a policy violation. When the reason
for an exception disappears, remove the exception in the same change.
