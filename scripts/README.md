# WFL Scripts

This directory contains utility scripts for WFL development, testing, and maintenance.

## Testing Scripts

### `run_integration_tests.ps1` / `.sh`
Runs the WFL integration test suite against all test programs in `TestPrograms/`.

**Requirements:**
- Release build of WFL (`cargo build --release`)

**Usage:**
```powershell
# PowerShell
.\scripts\run_integration_tests.ps1

# Bash
./scripts/run_integration_tests.sh
```

### `run_web_tests.ps1` / `.sh`
Runs WFL web server tests.

**Requirements:**
- Release build of WFL

**Usage:**
```powershell
# PowerShell
.\scripts\run_web_tests.ps1

# Bash
./scripts/run_web_tests.sh
```

### `validate_docs_examples.py`
Validates all code examples in the documentation using the WFL compiler and LSP tools.

**Requirements:**
- Python 3.x
- WFL release build

**Usage:**
```bash
python scripts/validate_docs_examples.py
```

## Configuration Scripts

### `init_config.ps1`
Interactive script to create `.wflcfg` configuration files.

**Usage:**
```powershell
.\scripts\init_config.ps1
```

### `configure_lsp.ps1`
Sets up the WFL Language Server Protocol (LSP) for IDE integration.

**Usage:**
```powershell
.\scripts\configure_lsp.ps1
```

## IDE Integration Scripts

### `install_vscode_extension.ps1`
Installs the WFL VS Code extension for development.

**Usage:**
```powershell
.\scripts\install_vscode_extension.ps1
```

## Maintenance Scripts

### `update_security_doc.ps1` / `.sh`
**Automatically updates `SECURITY.md` with current version information from `Cargo.toml`.**

This script:
- Extracts the current version from `Cargo.toml`
- Updates the supported versions table (current, limited support, no support)
- Updates the "Last Updated" date
- Updates the copyright year

**Usage:**
```powershell
# PowerShell
.\scripts\update_security_doc.ps1

# Bash
./scripts/update_security_doc.sh
```

**Automation:**
This script is automatically run monthly via GitHub Actions (`.github/workflows/update-security-doc.yml`), which creates a PR with the updates. You can also run it manually when the version changes.

**When to use:**
- After bumping version in `Cargo.toml`
- At the start of each month (automated)
- Before releases to ensure documentation is current

### `bump_version.py`
Computes and writes WFL versions using calendar-based versioning (YY.M.BUILD).
Nightly numbers come from published `vYY.M.N` tags for the current UTC month
(`--from-tags`), with the committed `.build_meta.json` as a same-month floor.
`--set-version` writes that version into every mirror without committing.

**Usage:**
```bash
# Next version from tags (stdout only)
python scripts/bump_version.py --from-tags --print

# Write an exact version into all mirrors, do not commit
python scripts/bump_version.py --set-version 26.10.1 --update-all --skip-git
```

### `sync-branch.ps1`
Utility script for branch synchronization.

**Usage:**
```powershell
.\scripts\sync-branch.ps1
```

## Release Publishing Scripts

These run from `.github/workflows/nightly.yml` against the DigitalOcean Spaces
bucket that backs <https://wfl.nyc3.cdn.digitaloceanspaces.com>, the canonical
download location. Both need `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` for
Spaces (`SPACES_BUCKET`, `SPACES_ENDPOINT` override the defaults) and `jq`.

### `publish_spaces.sh`
Uploads one nightly's artifacts. Immutable versioned objects go up first, each
with an immutable `<artifact>.sha256` sidecar; the rolling `latest` pointers,
`SHA256SUMS` and `status.json` follow only once every one of them succeeded, so a
partial publish is never observable as a release and anything it left behind is
unreferenced. Every object is then read back through the CDN and compared byte
for byte against what was uploaded.

Versioned keys are treated as write-once: re-publishing identical bytes is a
no-op, and a build whose bytes differ from what is already published under the
same key **skips that artifact** (success) rather than replacing it or failing
the nightly. Rolling `latest` pointers move only for artifacts this run
accepted. Re-running a publish that failed partway is therefore still the
supported way to finish it, and a same-version MSI rebuild no longer turns
Nightly Build red.

**Usage:**
```bash
./scripts/publish_spaces.sh <artifact-dir> <version> <short-sha> <commit-sha> <branch>
```

### `backfill_spaces_checksums.sh`
Creates the `<artifact>.sha256` sidecars for artifacts published before those
existed. Idempotent and additive: it only ever creates missing `*.sha256` keys,
never rewrites one, and never touches an artifact, a rolling pointer,
`SHA256SUMS`, or `status.json`. Runs after every nightly publish, and on demand
via the **Backfill Release Checksums** workflow.

**Usage:**
```bash
./scripts/backfill_spaces_checksums.sh --dry-run   # report what is missing
./scripts/backfill_spaces_checksums.sh             # repair it
```

### `test_publish_spaces.sh`
Tests both of the above by running them against a recording stub of the AWS CLI,
asserting the keys, bytes and cache headers they write. No credentials needed;
runs in CI as the **Release Script Tests** job.

**Usage:**
```bash
./scripts/test_publish_spaces.sh
```

## Development Workflow

### Typical Development Cycle

1. **Make changes** to WFL source code
2. **Build**: `cargo build --release`
3. **Test**:
   - Unit tests: `cargo test`
   - Integration: `.\scripts\run_integration_tests.ps1`
   - Web tests: `.\scripts\run_web_tests.ps1`
4. **Validate docs**: `python scripts/validate_docs_examples.py`
5. **Format**: `cargo fmt --all`
6. **Lint**: `cargo clippy --all-targets --all-features -- -D warnings`

### Version Release Workflow

Nightly builds compute the next YY.M.BUILD from `v*` tags, write it into the
tree at build time, and push the `vX.Y.Z` tag only after a successful
publish. CI does not push version commits to `main` or `dev`.

1. **Preview the next version**: `python scripts/bump_version.py --from-tags --print`
2. **Update security docs** if needed: `.\scripts\update_security_doc.ps1`
3. **Run all tests** (see above)
4. **Commit** feature work with conventional commits; do not hand-bump
   `.build_meta.json` unless setting a same-month floor
5. **Push** the feature branch; nightly tags the published version

## Script Conventions

- **Cross-platform**: Most scripts have both `.ps1` (PowerShell) and `.sh` (Bash) versions
- **Exit codes**: Scripts exit with non-zero code on failure
- **Output**: Color-coded output for success/warning/error
- **Safety**: All scripts use `-ErrorActionPreference "Stop"` (PowerShell) or `set -e` (Bash)

## Automation

Several scripts are integrated with GitHub Actions:

- **CI/CD** (`.github/workflows/ci.yml`): Runs tests; does not push version commits
- **Auto-format** (`.github/workflows/auto-fmt.yml`): Format checking
- **Security Doc Updates** (`.github/workflows/update-security-doc.yml`): Monthly SECURITY.md updates
- **Nightly** (`.github/workflows/nightly.yml`): Nightly builds and tests

## Contributing

When adding new scripts:

1. **Create both `.ps1` and `.sh` versions** when possible for cross-platform support
2. **Document** the script in this README
3. **Use consistent naming**: `snake_case` for script names
4. **Add error handling**: Exit on errors, provide clear error messages
5. **Test on multiple platforms**: Windows (PowerShell), Linux (Bash), macOS (Bash)
6. **Make bash scripts executable**: `chmod +x scripts/your_script.sh`

## See Also

- [CLAUDE.md](../CLAUDE.md) - Development guidelines
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guidelines
- [Docs/contributing/](../Docs/contributing/) - Development documentation

## Repository Hygiene

### `check_repo_hygiene.py`
Enforcement arm of the root `REPOSITORY_HYGIENE.md` policy (profile:
`.repo-hygiene.toml`). Dependency-free, Python 3.11+.

**Usage:**
```bash
python3 scripts/check_repo_hygiene.py --mode static        # tracked-tree rules
python3 scripts/check_repo_hygiene.py --mode working-tree  # post-suite cleanliness
```

Unit tests: `python3 -m unittest discover -s tests/tooling`.

## Packaging

### `build_windows_installer.ps1`
Canonical Windows MSI entry point (moved from root `build_msi.ps1`). Builds
the release binary if needed, syncs the wix version, and runs cargo-wix.
`-BumpVersion` increments the build number first; `-OutputDir <dir>` overrides
the MSI output location.

## Metrics and Docs Tooling

### `metrics/generate_rust_loc_report.py`
Rust line-of-code report generator (moved from `Tools/rust_loc_counter.py`).
Write output under `target/reports/` — reports are not tracked.

### `docs/combine_markdown.py`
Combines `Docs/` markdown files into a single document (moved from
`Tools/wfl_md_combiner.py`). Output belongs under `target/reports/`.
