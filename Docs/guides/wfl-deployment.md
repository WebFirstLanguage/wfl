# WFL Deployment Guide

This document outlines the deployment process for WFL (WebFirst Language), specifically focusing on building Windows MSI installers.

## Overview

The WFL project uses the WiX Toolset and cargo-wix to build Windows MSI installers. This document explains the prerequisites, build process, common issues, and best practices based on our experiences.

## Prerequisites

### Required Software

1. **Rust and Cargo**: The latest stable version is recommended
2. **WiX Toolset**: Version 3.14 or higher
   - Installation options:
     - Via Chocolatey: `choco install wixtoolset -y` (requires admin privileges)
     - Direct download from [wixtoolset.org/releases](https://wixtoolset.org/releases/)
3. **cargo-wix**: Rust crate for integrating with WiX Toolset
   - Install using: `cargo install cargo-wix@0.3.3 --locked`

### Environment Setup

- Ensure WiX Toolset's bin directory is in your PATH
- If using GitHub Actions, the workflow in `.github/workflows/nightly.yml` provides a reference implementation

## Build Process

The MSI build process consists of these steps:

1. **Dependency Checking**: Verify all required tools are installed
2. **Configuration**: Create necessary config files
3. **WiX Source Generation**: Generate WiX source files if not present
4. **Version Configuration**: Set correct version in wix.toml
5. **Binary Building**: Build the WFL executable(s)
6. **MSI Creation**: Package everything into the MSI installer

The `build_msi.ps1` script in the root directory automates this process.

## Key Files

- **build_msi.ps1**: Main script for building the MSI installer
- **wix.toml**: WiX configuration file for the project
- **.wflcfg**: Default configuration file included in the installer
- **wix/main.wxs**: WiX source file defining the installer structure
- **wix/License.rtf**: License file included in the installer

## Common Issues and Solutions

### 1. Parameter Conflicts

**Issue**: Using `--define` with cargo-wix can conflict with existing variables in WiX source files.
**Solution**: Directly update the `wix.toml` file with the desired version instead of passing it via command line.

Example:
```powershell
# Update version in wix.toml
$wixTomlContent = Get-Content -Path "wix.toml" -Raw
$updatedWixTomlContent = $wixTomlContent -replace 'version = "0.0.0.0"', 'version = "2025.4.0.0"'
Set-Content -Path "wix.toml" -Value $updatedWixTomlContent
```

### 2. Missing Binaries

**Issue**: cargo-wix reports "The system cannot find the file..." when the binary doesn't exist.
**Solution**: Always build the binary before running cargo-wix, or remove the `--no-build` option.

```powershell
# Check and build the binary if necessary
if (-not (Test-Path "target/release/wfl.exe")) {
    cargo build --release -p wfl
}
```

### 3. Workspace Projects

**Issue**: cargo-wix needs to know which package to build in a workspace project.
**Solution**: Always specify the package with `-p <package_name>`.

```powershell
cargo wix --nocapture -p wfl
```

### 4. Missing WiX Source Files

**Issue**: No WXS files available to create the installer.
**Solution**: Initialize WiX source files first using `cargo wix init`.

```powershell
if (-not (Test-Path "wix")) {
    cargo wix init -p wfl
}
```

### 5. File Association Configuration

**Issue**: Custom file associations may not be correctly registered or may conflict with existing applications.
**Solution**: The WiX configuration in `wix/main.wxs` handles file associations through ProgId elements.

```xml
<!-- Example file association configuration -->
<Component Id='binary0' Guid='*'>
    <File Id='exe0' Name='wfl.exe' KeyPath='yes'/>
    
    <!-- File association is defined at the Component level, not inside the File element -->
    <ProgId Id='wfl.wflfile' Description='WFL Script File'>
        <Extension Id='wfl' ContentType='application/x-wfl'>
            <Verb Id='open' Command='Open' TargetFile='exe0' Argument='"%1"' />
            <Verb Id='edit' Command='Edit' TargetFile='exe0' Argument='--edit "%1"' />
        </Extension>
    </ProgId>
</Component>
```

Key components:
- `ProgId`: Defines the program identifier for the file type
- `Extension`: Specifies the file extension to associate (.wfl)
- `Verb`: Defines actions that can be performed (Open, Edit)
- `Icon` attribute on ProgId: References the executable file (with IconIndex) to use as the file type icon

## Versioning

The WFL project uses a comprehensive versioning system that manages version consistency across multiple files:

### Versioning Scheme

- Core format: `YYYY.build` (e.g., `2025.4`)
- MSI installer format: `YYYY.build.0.0` (e.g., `2025.4.0.0` - Windows requires four components)
- Cargo/SemVer format: `YYYY.build.0` (e.g., `2025.4.0`)

### Versioning Files

- `.build_meta.json`: Source of truth containing year and build number
- `src/version.rs`: Core version constant used in the application
- `Cargo.toml`: Package and metadata versions in SemVer format 
- `wix.toml`: MSI installer version in Windows quad-format
- VS Code extension `package.json` files: Version information for extensions

### Version Management Script

The project provides `scripts/bump_version.py`, a comprehensive version management tool:

```bash
# Usage examples:

# Increment the build number and update all version references
python scripts/bump_version.py --update-all

# Update all version references without incrementing the build number
python scripts/bump_version.py --skip-bump --update-all

# Update only wix.toml for MSI builds (used by build_msi.ps1)
python scripts/bump_version.py --update-wix-only --skip-git

# Increment build and commit changes to git
python scripts/bump_version.py --update-all
```

#### Command Line Options

| Option | Description |
|--------|-------------|
| `--skip-bump` | Use the current version without incrementing |
| `--update-all` | Update versions in all project files |
| `--update-wix-only` | Only update wix.toml for MSI builds |
| `--skip-git` | Prevent automatic git commit of version changes |
| `--verbose` | Show detailed output during updates |

### Automated Integration

The versioning system is integrated with:

- `build_msi.ps1` script for local builds
- GitHub Actions workflow for CI/CD builds
- Release automation for consistent version tagging

### Version Override

For specific releases, you can override the version:

- In CI/CD: Use the `version_override` input in the GitHub Actions workflow
- In local builds: Manually edit `.build_meta.json` or directly use `--skip-bump` with the script

## Best Practices

1. **Separate Configuration**: Store default configuration in a separate file (.wflcfg)
2. **Dependency Checking**: Always verify dependencies before building
3. **Error Handling**: Provide clear error messages and recovery options
4. **Doc Generation**: Update documentation as part of the build process
5. **Build Artifacts**: Store build artifacts in a structured directory hierarchy
6. **CI Integration**: Use GitHub Actions to automate builds for each release
7. **Testing**: Test the installer in a clean environment to verify all components are correctly included

## MSI Features

The WFL MSI installer includes the following features:

- WFL compiler and runtime
- Default configuration file
- VS Code extension (optional)
- .wfl file association with application icon
- Double-click to execute .wfl files directly
- Right-click options for opening or editing .wfl files
- PATH environment variable integration
- Customizable installation directory
- Support for 64-bit Windows systems

## Future Improvements

1. **Digital Signing**: Implement code signing for the MSI installer
2. **Multi-platform Packages**: Extend build process to create Debian and macOS packages
3. **Auto-Updates**: Add mechanism for checking and applying updates
4. **Telemetry Option**: Allow users to opt in/out of anonymous usage statistics
5. **Custom Actions**: Add post-install configuration steps

## References

- [WiX Toolset Documentation](https://wixtoolset.org/docs/)
- [cargo-wix Repository](https://github.com/volks73/cargo-wix)
- [Windows Installer Documentation](https://docs.microsoft.com/en-us/windows/desktop/msi/windows-installer-portal)
- [Rust Packaging Guide](https://doc.rust-lang.org/cargo/guide/creating-a-new-project.html)
