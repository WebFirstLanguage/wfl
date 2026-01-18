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
Bumps the WFL version using calendar-based versioning (YY.MM.BUILD).

**Usage:**
```bash
python scripts/bump_version.py --update-all
```

### `sync-branch.ps1`
Utility script for branch synchronization.

**Usage:**
```powershell
.\scripts\sync-branch.ps1
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

1. **Bump version**: `python scripts/bump_version.py --update-all`
2. **Update security docs**: `.\scripts\update_security_doc.ps1`
3. **Run all tests** (see above)
4. **Commit and tag**: Follow conventional commits format
5. **Push**: Changes and tags to remote

## Script Conventions

- **Cross-platform**: Most scripts have both `.ps1` (PowerShell) and `.sh` (Bash) versions
- **Exit codes**: Scripts exit with non-zero code on failure
- **Output**: Color-coded output for success/warning/error
- **Safety**: All scripts use `-ErrorActionPreference "Stop"` (PowerShell) or `set -e` (Bash)

## Automation

Several scripts are integrated with GitHub Actions:

- **CI/CD** (`.github/workflows/ci.yml`): Runs tests and version bumping
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
- [Docs/development/](../Docs/development/) - Development documentation
