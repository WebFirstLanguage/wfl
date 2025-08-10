# Building and Releasing WFL

This document describes how to build the WFL compiler from source, how the nightly build pipeline works, and how to cut a release.

## Building from Source

WFL is built using Rust and Cargo. To build from source:

1. Ensure you have Rust installed (see [rustup.rs](https://rustup.rs/) for installation instructions)
2. Clone the repository: `git clone https://github.com/logbie/wfl.git`
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
