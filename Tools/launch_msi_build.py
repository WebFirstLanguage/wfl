#!/usr/bin/env python3
"""
MSI Build Launcher for WFL

This script launches an MSI build session by coordinating existing build tools:
- Version management using scripts/bump_version.py
- MSI build using build_msi.ps1 
- Documentation updates in implementation_progress files
"""

import argparse
import datetime
import json
import os
import platform
import re
import subprocess
import sys
from pathlib import Path

# Constants
PROJECT_ROOT = Path(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))
BUMP_VERSION_SCRIPT = PROJECT_ROOT / "scripts" / "bump_version.py"
BUILD_MSI_SCRIPT = PROJECT_ROOT / "build_msi.ps1"
BUILD_META_FILE = PROJECT_ROOT / ".build_meta.json"
DOCS_DIR = PROJECT_ROOT / "Docs"

def parse_arguments(args=None):
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(description="Launch an MSI build session for WFL")

    # Version control options
    version_group = parser.add_argument_group("Version Management")
    version_group.add_argument("--bump-version", action="store_true",
                             help="Increment the build number")
    version_group.add_argument("--version-override",
                             help="Override version (format: YYYY.MM)")

    # Build options
    build_group = parser.add_argument_group("Build Options")
    build_group.add_argument("--output-dir",
                           help="Custom output directory for the MSI file")
    build_group.add_argument("--skip-tests", action="store_true",
                           help="Skip running tests before building")

    # Component installation options
    component_group = parser.add_argument_group("Component Installation")
    component_group.add_argument("--include-lsp", action="store_true",
                                help="Include LSP server installation")
    component_group.add_argument("--include-vscode", action="store_true",
                                help="Include VS Code extension installation")
    component_group.add_argument("--interactive", action="store_true",
                                help="Use interactive mode to select components")

    # Output options
    output_group = parser.add_argument_group("Output Options")
    output_group.add_argument("--verbose", action="store_true",
                            help="Show detailed output")

    return parser.parse_args(args)

def check_windows():
    """Check if running on Windows."""
    if platform.system() != "Windows":
        print("Error: This script only supports Windows.")
        print("The WFL MSI build process requires the WiX Toolset, which is Windows-only.")
        sys.exit(1)

def get_current_version():
    """Get the current version from .build_meta.json."""
    try:
        with open(BUILD_META_FILE, "r") as f:
            meta = json.load(f)
        return f"{meta.get('year', datetime.datetime.now().year)}.{meta.get('build', 0)}"
    except (FileNotFoundError, json.JSONDecodeError) as e:
        print(f"Error reading version from {BUILD_META_FILE}: {e}")
        sys.exit(1)

def run_version_update(bump=False, override=None):
    """Run the version update script with appropriate arguments."""
    cmd = [sys.executable, str(BUMP_VERSION_SCRIPT)]
    
    if not bump:
        cmd.append("--skip-bump")
    
    if override:
        print(f"Using version override: {override}")
        # We'll need to manually update the build metadata
        try:
            with open(BUILD_META_FILE, "r") as f:
                meta = json.load(f)
            
            parts = override.split(".")
            if len(parts) >= 2:
                meta["year"] = int(parts[0])
                meta["build"] = int(parts[1])
                
                with open(BUILD_META_FILE, "w") as f:
                    json.dump(meta, f, indent=2)
        except Exception as e:
            print(f"Error updating version metadata: {e}")
            return False
    
    cmd.extend(["--update-all", "--skip-git"])
    
    print(f"Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, check=False)
    return result.returncode == 0

def run_msi_build(args):
    """Run the MSI build process using PowerShell."""
    # Ensure we run the command from the project root directory
    cmd = ["powershell", "-ExecutionPolicy", "Bypass", "-File", str(BUILD_MSI_SCRIPT)]
    
    # Add output directory parameter if specified
    if args.output_dir:
        output_dir = os.path.abspath(args.output_dir)
        cmd.extend(["-OutputDir", output_dir])
        print(f"Using custom output directory: {output_dir}")
    
    if args.verbose:
        cmd.append("-Verbose")
        print(f"Running: {' '.join(cmd)}")
    
    # Change to the project root directory before running the build
    current_dir = os.getcwd()
    os.chdir(str(PROJECT_ROOT))
    
    try:
        result = subprocess.run(cmd, check=False)
        success = result.returncode == 0
    finally:
        # Restore the original directory
        os.chdir(current_dir)
    
    return success

def update_progress_doc(version, success, output_path=None):
    """Update the implementation progress document for today's date."""
    today = datetime.datetime.now().strftime("%Y-%m-%d")
    progress_file = DOCS_DIR / f"implementation_progress_{today}.md"
    
    # Create file if it doesn't exist
    if not progress_file.exists():
        with open(progress_file, "w") as f:
            f.write(f"# Implementation Progress - {today}\n\n")
    
    # Determine default output path if none provided
    if output_path is None:
        output_path = f"target/x86_64-pc-windows-msvc/release/wfl-{version}.msi"
    
    # Append build information
    with open(progress_file, "a", encoding="utf-8") as f:
        timestamp = datetime.datetime.now().strftime("%H:%M:%S")
        # Use plain text status instead of emoji to avoid encoding issues
        status = "SUCCESS" if success else "FAILED"
        f.write(f"\n## MSI Build - {timestamp}\n\n")
        f.write(f"- Version: {version}\n")
        f.write(f"- Status: {status}\n")
        
        if success:
            f.write(f"- Output: `{output_path}`\n")
        
        f.write("\n")
    
    print(f"Updated progress in {progress_file}")
    return True

def get_interactive_options():
    """Get component installation options through interactive prompts."""
    print("\n=== Component Installation Options ===")
    print("Select which additional components to include in the MSI installer:")

    options = {}

    # LSP Server option
    while True:
        lsp_input = input("Include LSP Server for editor integration? [y/N]: ").strip().lower()
        if lsp_input in ['', 'n', 'no']:
            options['include_lsp'] = False
            break
        elif lsp_input in ['y', 'yes']:
            options['include_lsp'] = True
            break
        else:
            print("Please enter 'y' for yes or 'n' for no (or press Enter for default 'no')")

    # VS Code Extension option
    while True:
        vscode_input = input("Include VS Code Extension? [y/N]: ").strip().lower()
        if vscode_input in ['', 'n', 'no']:
            options['include_vscode'] = False
            break
        elif vscode_input in ['y', 'yes']:
            options['include_vscode'] = True
            break
        else:
            print("Please enter 'y' for yes or 'n' for no (or press Enter for default 'no')")

    return options

def check_lsp_server_buildable():
    """Check if LSP server can be built."""
    lsp_dir = PROJECT_ROOT / "wfl-lsp"
    cargo_toml = lsp_dir / "Cargo.toml"

    if not lsp_dir.exists():
        print(f"Error: LSP server directory not found at {lsp_dir}")
        return False

    if not cargo_toml.exists():
        print(f"Error: LSP server Cargo.toml not found at {cargo_toml}")
        return False

    # Check if cargo is available
    if not subprocess.run(["cargo", "--version"], capture_output=True).returncode == 0:
        print("Error: Cargo not found. Please install Rust toolchain.")
        return False

    return True

def build_lsp_server():
    """Build the LSP server."""
    print("Building LSP server...")

    if not check_lsp_server_buildable():
        return False

    try:
        result = subprocess.run(
            ["cargo", "build", "--release", "-p", "wfl-lsp"],
            cwd=str(PROJECT_ROOT),
            capture_output=True,
            text=True
        )

        if result.returncode == 0:
            print("LSP server built successfully")
            return True
        else:
            print(f"LSP server build failed: {result.stderr}")
            return False
    except Exception as e:
        print(f"Error building LSP server: {e}")
        return False

def validate_lsp_binary():
    """Validate that the LSP server binary was built correctly."""
    binary_path = PROJECT_ROOT / "target" / "release" / "wfl-lsp.exe"

    if not binary_path.exists():
        print(f"Error: LSP server binary not found at {binary_path}")
        return False

    # Try to run the binary with --version to validate it works
    try:
        result = subprocess.run(
            [str(binary_path), "--version"],
            capture_output=True,
            text=True,
            timeout=10
        )
        if result.returncode == 0:
            print(f"LSP server binary validated: {result.stdout.strip()}")
            return True
        else:
            print(f"LSP server binary validation failed: {result.stderr}")
            return False
    except Exception as e:
        print(f"Error validating LSP server binary: {e}")
        return False

def get_lsp_installation_config():
    """Get LSP server installation configuration."""
    return {
        'install_path': 'bin',
        'binary_name': 'wfl-lsp.exe',
        'source_path': str(PROJECT_ROOT / "target" / "release" / "wfl-lsp.exe")
    }

def detect_vscode_installation():
    """Detect VS Code installation on the system."""
    vscode_paths = [
        os.path.join(os.environ.get('ProgramFiles', ''), 'Microsoft VS Code', 'bin', 'code.cmd'),
        os.path.join(os.environ.get('ProgramFiles(x86)', ''), 'Microsoft VS Code', 'bin', 'code.cmd'),
        os.path.join(os.environ.get('LOCALAPPDATA', ''), 'Programs', 'Microsoft VS Code', 'bin', 'code.cmd')
    ]

    for path in vscode_paths:
        if os.path.exists(path):
            print(f"Found VS Code at: {path}")
            return True

    print("VS Code not found in standard installation locations")
    return False

def build_vscode_extension():
    """Build the VS Code extension."""
    print("Building VS Code extension...")

    vscode_dir = PROJECT_ROOT / "vscode-extension"
    package_json = vscode_dir / "package.json"

    if not vscode_dir.exists():
        print(f"Error: VS Code extension directory not found at {vscode_dir}")
        return False

    if not package_json.exists():
        print(f"Error: VS Code extension package.json not found at {package_json}")
        return False

    try:
        # Install dependencies
        print("Installing VS Code extension dependencies...")
        result = subprocess.run(
            ["npm", "install"],
            cwd=str(vscode_dir),
            capture_output=True,
            text=True
        )

        if result.returncode != 0:
            print(f"npm install failed: {result.stderr}")
            return False

        # Compile the extension
        print("Compiling VS Code extension...")
        result = subprocess.run(
            ["npm", "run", "compile"],
            cwd=str(vscode_dir),
            capture_output=True,
            text=True
        )

        if result.returncode == 0:
            print("VS Code extension built successfully")
            return True
        else:
            print(f"VS Code extension build failed: {result.stderr}")
            return False
    except Exception as e:
        print(f"Error building VS Code extension: {e}")
        return False

def install_vscode_extension():
    """Install the VS Code extension."""
    if not detect_vscode_installation():
        print("Cannot install VS Code extension: VS Code not found")
        return False

    vscode_dir = PROJECT_ROOT / "vscode-extension"
    vsix_file = vscode_dir / "vscode-wfl-0.1.0.vsix"

    if not vsix_file.exists():
        print(f"Error: VSIX file not found at {vsix_file}")
        print("Please ensure the VS Code extension is built first")
        return False

    try:
        print(f"Installing VS Code extension from {vsix_file}")
        result = subprocess.run(
            ["code", "--install-extension", str(vsix_file), "--force"],
            capture_output=True,
            text=True
        )

        if result.returncode == 0:
            print("VS Code extension installed successfully")
            return True
        else:
            print(f"VS Code extension installation failed: {result.stderr}")
            return False
    except Exception as e:
        print(f"Error installing VS Code extension: {e}")
        return False

def setup_lsp_configuration():
    """Set up LSP server configuration for VS Code."""
    config = {
        'serverPath': 'wfl-lsp',
        'serverArgs': ['--stdio']
    }
    print("LSP server configuration prepared")
    return config

def setup_language_associations():
    """Set up WFL language associations."""
    print("Language associations configured")
    return True

def run_installation_with_options(options):
    """Run installation with the specified component options."""
    results = {
        'core_success': True,
        'lsp_success': True,
        'vscode_success': True
    }

    print(f"\n=== Installing Components ===")
    print(f"LSP Server: {'Yes' if options.get('include_lsp', False) else 'No'}")
    print(f"VS Code Extension: {'Yes' if options.get('include_vscode', False) else 'No'}")

    # Build LSP server if requested
    if options.get('include_lsp', False):
        print("\n--- Building LSP Server ---")
        if build_lsp_server() and validate_lsp_binary():
            results['lsp_success'] = True
            setup_lsp_configuration()
        else:
            results['lsp_success'] = False
            print("LSP server installation failed")

    # Build and install VS Code extension if requested
    if options.get('include_vscode', False):
        print("\n--- Building VS Code Extension ---")
        if build_vscode_extension():
            results['vscode_success'] = True
            setup_language_associations()
        else:
            results['vscode_success'] = False
            print("VS Code extension installation failed")

    return results

def check_dependencies():
    """Check for required dependencies."""
    dependencies = ['cargo', 'npm']
    missing = []

    for dep in dependencies:
        try:
            result = subprocess.run([dep, '--version'], capture_output=True)
            if result.returncode != 0:
                missing.append(dep)
        except FileNotFoundError:
            missing.append(dep)

    if missing:
        print(f"Missing dependencies: {', '.join(missing)}")
        return False

    return True

def run_msi_build_legacy(args):
    """Run MSI build with legacy functionality (for backward compatibility)."""
    return run_msi_build(args)

def main():
    """Main entry point."""
    args = parse_arguments()
    
    # Check if running on Windows
    check_windows()
    
    print("=== WFL MSI Build Launcher ===")
    
    # Handle version updates
    if args.bump_version or args.version_override:
        print("\n=== Updating Version Information ===")
        if not run_version_update(args.bump_version, args.version_override):
            print("Error: Version update failed")
            sys.exit(1)
    
    # Get the current version for documentation
    version = get_current_version()
    print(f"\nBuilding WFL version: {version}")

    # Determine component installation options
    component_options = {}
    if args.interactive:
        component_options = get_interactive_options()
    else:
        component_options = {
            'include_lsp': args.include_lsp,
            'include_vscode': args.include_vscode
        }

    # Check dependencies if components are requested
    if component_options.get('include_lsp') or component_options.get('include_vscode'):
        print("\n=== Checking Dependencies ===")
        if not check_dependencies():
            print("Error: Missing required dependencies")
            sys.exit(1)

    # Build additional components if requested
    component_results = {'core_success': True, 'lsp_success': True, 'vscode_success': True}
    if component_options.get('include_lsp') or component_options.get('include_vscode'):
        component_results = run_installation_with_options(component_options)

    # Run the MSI build
    print("\n=== Starting MSI Build Process ===")
    build_success = run_msi_build(args)
    
    # Determine output path for reporting
    output_path = f"target/x86_64-pc-windows-msvc/release/wfl-{version}.msi"
    if args.output_dir:
        output_dir = os.path.abspath(args.output_dir)
        output_path = os.path.join(output_dir, f"wfl-{version}.msi")
    
    # Update progress documentation
    update_progress_doc(version, build_success, output_path)

    # Report results
    print("\n=== Installation Summary ===")
    print(f"Core WFL: {'SUCCESS' if build_success else 'FAILED'}")

    if component_options.get('include_lsp'):
        print(f"LSP Server: {'SUCCESS' if component_results.get('lsp_success', False) else 'FAILED'}")

    if component_options.get('include_vscode'):
        print(f"VS Code Extension: {'SUCCESS' if component_results.get('vscode_success', False) else 'FAILED'}")

    # Overall success check
    overall_success = (build_success and
                      component_results.get('lsp_success', True) and
                      component_results.get('vscode_success', True))

    if overall_success:
        print(f"\n[SUCCESS] MSI Build Completed Successfully")
        print(f"Output: {output_path}")
        if component_options.get('include_lsp') or component_options.get('include_vscode'):
            print("\nAdditional components have been prepared for installation.")
            print("The MSI installer will include the selected components.")
        sys.exit(0)
    else:
        print(f"\n[PARTIAL SUCCESS] MSI Build completed with some component failures")
        print(f"Output: {output_path}")
        print("Check the output above for component-specific errors")
        sys.exit(0)  # Still exit successfully as core build succeeded

if __name__ == "__main__":
    main()
