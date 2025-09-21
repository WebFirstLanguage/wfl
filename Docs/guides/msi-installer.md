# WFL Enhanced MSI Installer Guide

The WFL Windows MSI installer provides a comprehensive installation experience with optional components for enhanced development workflow.

## Overview

The enhanced MSI installer allows users to choose which components to install:

- **WFL Core** (Required): The main WFL compiler and runtime
- **LSP Server** (Optional): Language Server Protocol server for editor integration
- **VS Code Extension** (Optional): Visual Studio Code extension with full WFL support

## Installation Options

### Standard Installation

Download the MSI installer from the [GitHub Releases](https://github.com/WebFirstLanguage/wfl/releases) page and run it. The installer will present a feature selection dialog where you can choose which components to install.

### Component Details

#### WFL Core
- **What it includes**: WFL compiler (`wfl.exe`), runtime libraries, documentation
- **Installation path**: `%ProgramFiles%\WFL\bin\`
- **PATH integration**: Automatically adds WFL to system PATH
- **File associations**: Associates `.wfl` files with the WFL compiler
- **Required**: Yes

#### LSP Server
- **What it includes**: Language Server Protocol server (`wfl-lsp.exe`)
- **Installation path**: `%ProgramFiles%\WFL\bin\`
- **Features**: Syntax highlighting, code completion, error checking, hover information
- **Editor support**: VS Code, Vim/Neovim, Emacs, and other LSP-compatible editors
- **Required**: No
- **Dependencies**: None (self-contained)

#### VS Code Extension
- **What it includes**: WFL extension for Visual Studio Code
- **Installation**: Automatically installs to VS Code if detected
- **Features**: 
  - Syntax highlighting with WFL-specific grammar
  - IntelliSense and code completion (requires LSP Server)
  - Error diagnostics and linting
  - Code formatting and auto-indentation
  - Integrated terminal support
- **Required**: No
- **Dependencies**: Visual Studio Code must be installed

## Building Custom MSI

Developers can build custom MSI installers with specific component combinations.

### Prerequisites

- Windows 10/11 or Windows Server 2019+
- WiX Toolset 3.11+ (automatically installed by build script)
- Rust toolchain (for LSP server)
- Node.js and npm (for VS Code extension)

### Build Commands

```bash
# Interactive mode - prompts for component selection
python Tools/launch_msi_build.py --interactive

# Include all components
python Tools/launch_msi_build.py --include-lsp --include-vscode

# Include only LSP server
python Tools/launch_msi_build.py --include-lsp

# Include only VS Code extension
python Tools/launch_msi_build.py --include-vscode

# Core only (default behavior)
python Tools/launch_msi_build.py

# With custom output directory
python Tools/launch_msi_build.py --include-lsp --output-dir "C:\MyBuilds"

# With version override
python Tools/launch_msi_build.py --version-override "2025.10" --include-lsp
```

### Build Script Options

| Option | Description | Example |
|--------|-------------|---------|
| `--interactive` | Interactive component selection | `--interactive` |
| `--include-lsp` | Include LSP server | `--include-lsp` |
| `--include-vscode` | Include VS Code extension | `--include-vscode` |
| `--output-dir` | Custom output directory | `--output-dir "C:\Builds"` |
| `--bump-version` | Increment build number | `--bump-version` |
| `--version-override` | Override version | `--version-override "2025.10"` |
| `--skip-tests` | Skip running tests | `--skip-tests` |
| `--verbose` | Detailed output | `--verbose` |

## Post-Installation Configuration

### LSP Server Configuration

When the LSP server is installed, the installer automatically:

1. **Validates the LSP binary**: Ensures the server starts correctly
2. **Creates registry entries**: Stores LSP server path and version
3. **Generates VS Code settings template**: Creates configuration template at `%ProgramFiles%\WFL\config\vscode-settings-template.json`

#### Manual LSP Configuration

For editors other than VS Code, configure the LSP server manually:

**VS Code** (if extension not installed):
```json
{
    "wfl.serverPath": "C:\\Program Files\\WFL\\bin\\wfl-lsp.exe",
    "wfl.serverArgs": ["--stdio"]
}
```

**Vim/Neovim with coc.nvim**:
```json
{
    "languageserver": {
        "wfl": {
            "command": "C:\\Program Files\\WFL\\bin\\wfl-lsp.exe",
            "args": ["--stdio"],
            "filetypes": ["wfl"]
        }
    }
}
```

### VS Code Extension Configuration

The VS Code extension is automatically configured when installed. Key settings:

```json
{
    "wfl.serverPath": "C:\\Program Files\\WFL\\bin\\wfl-lsp.exe",
    "wfl.serverArgs": ["--stdio"],
    "wfl.versionMode": "warn",
    "wfl.format.enable": true,
    "wfl.format.formatOnSave": true
}
```

## Troubleshooting

### Common Issues

#### LSP Server Not Starting
1. Check if `wfl-lsp.exe` exists in `%ProgramFiles%\WFL\bin\`
2. Run `wfl-lsp.exe --version` in Command Prompt
3. Check Windows Event Viewer for error messages

#### VS Code Extension Not Working
1. Verify VS Code is installed and up to date
2. Check if the extension is enabled: `code --list-extensions | findstr wfl`
3. Restart VS Code after installation
4. Check VS Code's Output panel for WFL extension logs

#### Build Failures
1. Ensure all dependencies are installed (Rust, Node.js)
2. Run with `--verbose` flag for detailed output
3. Check that WiX Toolset is properly installed
4. Verify network connectivity for dependency downloads

### Log Files

The installer creates log files for troubleshooting:

- **MSI Installation Log**: `%TEMP%\wfl-install.log`
- **LSP Configuration Log**: `%ProgramFiles%\WFL\logs\lsp-config.log`
- **VS Code Extension Log**: `%ProgramFiles%\WFL\logs\vscode-install.log`

## Uninstallation

### Standard Uninstall
1. Open "Add or Remove Programs" in Windows Settings
2. Find "WebFirst Language (WFL)" in the list
3. Click "Uninstall" and follow the prompts

### Component Removal
Individual components can be removed by:
1. Running the MSI installer again
2. Selecting "Modify" installation
3. Unchecking components to remove
4. Completing the modification process

### Manual Cleanup
If needed, manually remove:
- Installation directory: `%ProgramFiles%\WFL\`
- Registry entries: `HKLM\SOFTWARE\WFL`
- PATH environment variable entry
- VS Code extension: `code --uninstall-extension wfl`

## Advanced Configuration

### Silent Installation

For automated deployments, use silent installation:

```cmd
# Install all components silently
msiexec /i wfl-25.9.1.msi /quiet ADDLOCAL=ALL

# Install core only
msiexec /i wfl-25.9.1.msi /quiet ADDLOCAL=Binaries,Environment

# Install with LSP server
msiexec /i wfl-25.9.1.msi /quiet ADDLOCAL=Binaries,Environment,LSPServerFeature

# Install with VS Code extension
msiexec /i wfl-25.9.1.msi /quiet ADDLOCAL=Binaries,Environment,VSCodeExtensionFeature
```

### Feature Names

| Component | Feature Name |
|-----------|--------------|
| WFL Core | `Binaries` |
| PATH Environment | `Environment` |
| LSP Server | `LSPServerFeature` |
| VS Code Extension | `VSCodeExtensionFeature` |

## Support

For issues with the MSI installer:

1. Check the [GitHub Issues](https://github.com/WebFirstLanguage/wfl/issues) page
2. Review the troubleshooting section above
3. Create a new issue with:
   - Windows version
   - Installation log files
   - Steps to reproduce the problem
   - Expected vs actual behavior
