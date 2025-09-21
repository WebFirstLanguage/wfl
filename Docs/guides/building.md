# Building and Releasing WFL

This document describes how to build the WFL compiler from source, how the nightly build pipeline works, and how to cut a release.

## Building from Source

WFL is built using Rust and Cargo. To build from source:

1. Ensure you have Rust installed (see [rustup.rs](https://rustup.rs/) for installation instructions)
2. Clone the repository: `git clone https://github.com/WebFirstLanguage/wfl.git`
3. Navigate to the project directory: `cd wfl`
4. Build the project: `cargo build --release`
5. Run the compiler: `./target/release/wfl <file>`

## Nightly Builds

WFL has an automated nightly build pipeline that creates installers for Windows, Linux, and macOS.

### Schedule

The nightly builds run at **00:00 America/Chicago** (05:00 UTC during daylight-saving time; 06:00 UTC during standard time - November to March).

### Skip-if-Unchanged Logic

To save CI resources, the nightly build will skip if no changes have been made to the source code since the last successful build. This is determined by:

1. Comparing the current HEAD SHA with the last successful build SHA stored in the nightly release notes
2. If they match, the build is skipped
3. If they don't match, we check for file changes including:
   - Rust source files (.rs)
   - Cargo.toml
   - GitHub workflow files
   - Build scripts
4. Changes are detected using `git diff --name-status` to catch additions, deletions, and renames
5. Only if relevant changes are detected, the build proceeds

### Artifacts

The nightly build produces the following artifacts:

| OS | Artifact | Installation Location |
|----|----------|----------------------|
| Windows | `wfl-<version>.msi` + `wfl-Updater.exe` | `%ProgramFiles%\WFL\` |
| Linux | `wfl-<version>.tar.gz` and `.deb` | `/opt/wfl/` (deb), custom (tar.gz) |
| macOS | `wfl-<version>.pkg` | `/Applications/WFL.app/` |

### Version Numbering

Nightly builds use the format: `0.0.0-nightly.<YYYYMMDD>+<short-sha>`
   
For example: `0.0.0-nightly.20250420+fd1e218`

This format respects SemVer (prerelease & build-metadata segments) and sorts correctly in package managers.

### Manual Trigger

You can manually trigger a nightly build by:

1. Going to the GitHub repository
2. Navigating to Actions → Nightly Build
3. Clicking "Run workflow"
4. Optionally providing a specific SHA to compare against

## Cutting a Release

To cut a formal versioned release from a nightly:

1. Identify a stable nightly build that passes all tests
2. Update the version number in `Cargo.toml`
3. Create a release commit and tag: `git tag v0.x.y`
4. Push the tag: `git push origin v0.x.y`
5. The release workflow will automatically create a GitHub release with the appropriate artifacts

### Code Signing

Currently, installers are built without code signing. When code signing certificates are available:
   
1. Windows MSI: The EV code-signing certificate (PFX) will be added to GitHub Secrets
2. macOS pkg: The Developer ID Installer certificate (.p12) will be added to GitHub Secrets
3. The SIGNING_SKIP environment variable will be set to false in the workflow

## Enhanced MSI Installer

The Windows MSI installer has been enhanced to support optional component installation, providing users with flexibility in choosing which components to install.

### Available Components

The MSI installer includes the following components:

| Component | Description | Default | Required |
|-----------|-------------|---------|----------|
| **WFL Core** | Main WFL compiler and runtime | ✓ | Yes |
| **PATH Environment** | Add WFL to system PATH | ✓ | No |
| **LSP Server** | Language Server Protocol server for editor integration | ✗ | No |
| **VS Code Extension** | Visual Studio Code extension with syntax highlighting and IntelliSense | ✗ | No |

### Building Enhanced MSI

To build the MSI installer with component options:

```bash
# Build with all components (interactive selection)
python Tools/launch_msi_build.py --interactive

# Build with specific components
python Tools/launch_msi_build.py --include-lsp --include-vscode

# Build core only (default behavior)
python Tools/launch_msi_build.py

# Build with verbose output
python Tools/launch_msi_build.py --include-lsp --verbose
```

### Command Line Options

The enhanced MSI build script supports the following options:

```
Version Management:
  --bump-version        Increment the build number
  --version-override    Override version (format: YYYY.MM)

Build Options:
  --output-dir          Custom output directory for the MSI file
  --skip-tests          Skip running tests before building

Component Installation:
  --include-lsp         Include LSP server installation
  --include-vscode      Include VS Code extension installation
  --interactive         Use interactive mode to select components

Output Options:
  --verbose             Show detailed output
```

### Interactive Mode

Interactive mode provides a user-friendly way to select components:

```bash
python Tools/launch_msi_build.py --interactive
```

This will prompt you to select which optional components to include:

```
=== Component Installation Options ===
Select which additional components to include in the MSI installer:
Include LSP Server for editor integration? [y/N]: y
Include VS Code Extension? [y/N]: n
```

### Post-Installation Configuration

When optional components are selected, the installer automatically:

1. **LSP Server**:
   - Validates the LSP server binary
   - Creates registry entries for editor integration
   - Generates VS Code settings template

2. **VS Code Extension**:
   - Detects VS Code installation
   - Installs the WFL extension automatically
   - Configures language associations

### Dependencies

Building with optional components requires:

- **LSP Server**: Rust toolchain (cargo)
- **VS Code Extension**: Node.js (npm)

The build script will check for these dependencies and provide clear error messages if they're missing.

### Backward Compatibility

The enhanced MSI installer maintains full backward compatibility:

- Existing build commands work unchanged
- Default behavior installs core WFL only
- All existing command-line arguments are preserved
- Legacy MSI functionality remains intact
