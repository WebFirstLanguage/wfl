#!/usr/bin/env python3
import json
import datetime
import os
import re
import subprocess
import sys
import argparse

# File paths
BUILD_META_FILE = ".build_meta.json"
VERSION_FILE = "src/version.rs"
CARGO_TOML = "Cargo.toml"
WIX_TOML = "wix.toml"
VSCODE_EXTENSION_DIRS = ["vscode-extension", "vscode-wfl", "editors/vscode-wfl"]
MODIFIED_FILES = []

def parse_args():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(description="Update WFL version numbers across the project.")
    parser.add_argument("--skip-bump", action="store_true", help="Skip incrementing the build number")
    parser.add_argument("--update-all", action="store_true", help="Update all version files")
    parser.add_argument("--update-wix-only", action="store_true", help="Only update wix.toml")
    parser.add_argument("--skip-git", action="store_true", help="Skip git commit")
    parser.add_argument("--verbose", action="store_true", help="Show detailed output")
    return parser.parse_args()

def get_current_version():
    """Get the current version from build_meta.json."""
    if not os.path.exists(BUILD_META_FILE):
        print(f"Error: {BUILD_META_FILE} not found")
        sys.exit(1)
    
    with open(BUILD_META_FILE, "r") as f:
        try:
            meta = json.load(f)
        except json.JSONDecodeError:
            print(f"Error: {BUILD_META_FILE} is not valid JSON")
            sys.exit(1)
    
    # New version format: YY.MM.BUILD
    year = meta.get('year', datetime.datetime.now().year % 100)
    month = meta.get('month', datetime.datetime.now().month)
    build = meta.get('build', 1)
    return meta, f"{year}.{month}.{build}"

def bump_version(skip_bump=False):
    """Increment the build number in build_meta.json and update version.rs."""
    meta, old_version = get_current_version()
    
    if skip_bump:
        print(f"Using current version: {old_version}")
        return meta, old_version
    
    now = datetime.datetime.now()
    current_year = now.year % 100  # Get 2-digit year
    current_month = now.month
    
    build_num = meta.get("build", 1)
    last_year = meta.get("year", current_year)
    last_month = meta.get("month", current_month)
    
    # Reset build number if year or month changes
    if current_year != last_year or current_month != last_month:
        build_num = 1
        meta["year"] = current_year
        meta["month"] = current_month
    else:
        build_num += 1
    
    meta["build"] = build_num
    
    new_version = f"{current_year}.{current_month}.{build_num}"
    print(f"Bumped version: {old_version} -> {new_version}")
    
    with open(BUILD_META_FILE, "w") as f:
        json.dump(meta, f, indent=2)
    MODIFIED_FILES.append(BUILD_META_FILE)
    
    os.makedirs(os.path.dirname(VERSION_FILE), exist_ok=True)
    
    with open(VERSION_FILE, "w") as vf:
        vf.write(f'pub const VERSION: &str = "{new_version}";\n')
    MODIFIED_FILES.append(VERSION_FILE)
    
    return meta, new_version

def update_cargo_toml(version):
    """Update version in Cargo.toml."""
    if not os.path.exists(CARGO_TOML):
        print(f"Warning: {CARGO_TOML} not found, skipping")
        return False
    
    print(f"Updating {CARGO_TOML}...")
    
    with open(CARGO_TOML, "r") as f:
        content = f.read()
    
    # Convert version to semver format for Cargo.toml (YY.MM.BUILD)
    semver_version = version
    
    # Update package version
    new_content = re.sub(r'(version = )"(\d+\.\d+\.\d+)"', f'\\1"{semver_version}"', content, count=1)
    
    # Update package.metadata.bundle version
    new_content = re.sub(r'(\[package\.metadata\.bundle\].*?version = )"([^"]*)"',
                         f'\\1"{semver_version}"', new_content, flags=re.DOTALL)
    
    # Write updated content
    with open(CARGO_TOML, "w") as f:
        f.write(new_content)
    
    MODIFIED_FILES.append(CARGO_TOML)
    return True

def update_cargo_lock():
    """Update Cargo.lock to match Cargo.toml version by running cargo update.

    Raises SystemExit on any error to ensure CI failure.
    """
    import subprocess

    CARGO_LOCK = "Cargo.lock"

    if not os.path.exists(CARGO_TOML):
        print(f"Error: {CARGO_TOML} not found, cannot update Cargo.lock")
        sys.exit(1)

    print("Updating Cargo.lock to match Cargo.toml version...")

    # Extract expected version from Cargo.toml
    try:
        with open(CARGO_TOML, "r") as f:
            cargo_toml_content = f.read()

        # Extract main package version
        import re
        version_match = re.search(r'^version = "([^"]+)"', cargo_toml_content, re.MULTILINE)
        if not version_match:
            print("Error: Could not extract version from Cargo.toml")
            sys.exit(1)

        expected_version = version_match.group(1)
        print(f"Expected version: {expected_version}")

    except Exception as e:
        print(f"Error reading Cargo.toml: {e}")
        sys.exit(1)

    # Run cargo update
    try:
        result = subprocess.run(
            ["cargo", "update", "--package", "wfl"],
            capture_output=True,
            text=True,
            check=True
        )
        print("Cargo update completed successfully")

    except subprocess.CalledProcessError as e:
        print(f"Error running cargo update: {e}")
        print(f"stdout: {e.stdout}")
        print(f"stderr: {e.stderr}")
        sys.exit(1)
    except FileNotFoundError:
        print("Error: cargo command not found. Make sure Rust/Cargo is installed.")
        sys.exit(1)

    # Verify Cargo.lock exists and extract version
    if not os.path.exists(CARGO_LOCK):
        print(f"Error: {CARGO_LOCK} not found after cargo update")
        sys.exit(1)

    try:
        # Extract version from Cargo.lock using cross-platform Python approach
        with open(CARGO_LOCK, "r") as f:
            cargo_lock_content = f.read()

        # Find WFL package version specifically (equivalent to grep -A1 'name = "wfl"')
        wfl_package_match = re.search(r'\[\[package\]\]\s*name = "wfl"\s*version = "([^"]+)"', cargo_lock_content, re.DOTALL)
        if not wfl_package_match:
            print("Error: Could not find WFL package version in Cargo.lock")
            sys.exit(1)

        actual_version = wfl_package_match.group(1)
        print(f"Cargo.lock version: {actual_version}")

        # Verify versions match
        if expected_version != actual_version:
            print(f"Error: Version mismatch!")
            print(f"  Cargo.toml version: {expected_version}")
            print(f"  Cargo.lock version: {actual_version}")
            print("Cargo.lock was not properly synchronized")
            sys.exit(1)

        print(f"✓ Version synchronization verified: {expected_version}")
        MODIFIED_FILES.append(CARGO_LOCK)

    except Exception as e:
        print(f"Error validating Cargo.lock: {e}")
        sys.exit(1)

def update_wix_toml(version):
    """Update version in wix.toml."""
    if not os.path.exists(WIX_TOML):
        print(f"Warning: {WIX_TOML} not found, skipping")
        return False
    
    print(f"Updating {WIX_TOML}...")
    
    with open(WIX_TOML, "r") as f:
        content = f.read()
    
    # Windows MSI version needs 4 components: major.minor.build.0
    # Our format YY.MM.BUILD already has 3 components, just add .0
    windows_version = f"{version}.0"
    
    if 'version = "' in content:
        # Replace existing version line
        new_content = re.sub(r'version = "([^"]*)"(.*)', 
                            f'version = "{windows_version}" # Updated by bump_version.py', 
                            content)
    else:
        # Add version to the top of the file
        new_content = f'version = "{windows_version}" # Updated by bump_version.py\n\n{content}'
    
    with open(WIX_TOML, "w") as f:
        f.write(new_content)
    
    MODIFIED_FILES.append(WIX_TOML)
    return True

def update_vscode_extensions(version):
    """Update version in VS Code extension package.json files."""
    updated = False
    
    for ext_dir in VSCODE_EXTENSION_DIRS:
        pkg_file = os.path.join(ext_dir, "package.json")
        if not os.path.exists(pkg_file):
            continue
        
        print(f"Updating {pkg_file}...")
        
        with open(pkg_file, "r") as f:
            try:
                pkg_data = json.load(f)
            except json.JSONDecodeError:
                print(f"Warning: {pkg_file} is not valid JSON, skipping")
                continue
        
        # VS Code extensions use semver (our format is already compatible)
        semver_version = version
        
        pkg_data["version"] = semver_version
        
        with open(pkg_file, "w") as f:
            json.dump(pkg_data, f, indent=2)
        
        MODIFIED_FILES.append(pkg_file)
        updated = True
    
    return updated

def commit_changes(version, skip_git=False):
    """Commit changes to git."""
    if skip_git:
        print("Skipping git commit as requested")
        return True
    
    if not MODIFIED_FILES:
        print("No files modified, skipping git commit")
        return True
    
    print(f"Committing changes to git: {', '.join(MODIFIED_FILES)}")
    
    try:
        subprocess.run(["git", "config", "user.name", "github-actions"], check=True)
        subprocess.run(["git", "config", "user.email", "github-actions@github.com"], check=True)
        subprocess.run(["git", "add"] + MODIFIED_FILES, check=True)
        commit_msg = f"Bump version to {version} [skip ci]"
        subprocess.run(["git", "commit", "-m", commit_msg], check=True)
        print(f"Successfully committed version bump to {version}")
        return True
    except subprocess.CalledProcessError as e:
        print(f"Error during git operations: {e}")
        return False

def main():
    args = parse_args()
    
    if args.update_wix_only:
        # Just get current version and update wix.toml
        meta, version = get_current_version()
        update_wix_toml(version)
        print(f"Updated wix.toml with version {version}")
        return 0
    
    # Bump the version in main files
    meta, version = bump_version(args.skip_bump)
    
    # Update additional files based on arguments
    if args.update_all:
        update_cargo_toml(version)
        # Update Cargo.lock after Cargo.toml to ensure version synchronization
        update_cargo_lock()
        update_vscode_extensions(version)
        update_wix_toml(version)
        print(f"Updated all version references to {version}")
    
    # Commit changes if needed
    if not args.skip_git:
        if not commit_changes(version, args.skip_git):
            return 1
    
    return 0

if __name__ == "__main__":
    sys.exit(main())
