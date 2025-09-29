# Version Synchronization Guide

This document explains how version synchronization works in the WFL project and how to prevent version drift between build files.

## Overview

The WFL project uses multiple files to track version information:
- `Cargo.toml` - Main Rust package version AND `package.metadata.bundle` version
- `Cargo.lock` - Lock file with exact dependency versions (including WFL itself)
- `src/version.rs` - Runtime version constant
- `.build_meta.json` - Build metadata with version tracking
- Various package.json files for VS Code extensions
- `wix.toml` - Windows installer version

**Important**: The `Cargo.toml` file contains TWO version fields that must be synchronized:
1. `[package] version = "X.Y.Z"` - Main package version
2. `[package.metadata.bundle] version = "X.Y.Z"` - Bundle metadata version

## The Problem

When versions are updated manually or through automated processes, it's possible for these files to become out of sync. The most common issue is when `Cargo.toml` is updated but `Cargo.lock` is not regenerated, leading to:

- Build inconsistencies
- Confusion about the actual project version
- Potential deployment issues
- CI/CD pipeline failures

## Automated Version Management

### Version Bump Script

The project uses `scripts/bump_version.py` to handle version updates across all files:

```bash
# Update all version files
python scripts/bump_version.py --update-all

# Skip git commit (for testing)
python scripts/bump_version.py --update-all --skip-git

# Only update wix.toml
python scripts/bump_version.py --update-wix-only
```

### CI/CD Integration

The `.github/workflows/versioning.yml` workflow automatically:
1. Runs the version bump script with `--update-all`
2. Commits changes with `[skip ci]` to prevent infinite loops
3. Tags the new version
4. Pushes changes to the repository

## Manual Version Synchronization

If you need to manually fix version synchronization:

### Step 1: Update Cargo.lock
```bash
# Update Cargo.lock to match Cargo.toml
cargo update --package wfl

# Or update all dependencies
cargo update
```

### Step 2: Verify Synchronization
```bash
# Check that all versions match
echo "Main package version:"
grep '^version = ' Cargo.toml | head -1

echo "Bundle metadata version:"
grep -A10 '\[package\.metadata\.bundle\]' Cargo.toml | grep 'version = '

echo "Cargo.lock version:"
grep 'name = "wfl"' -A1 Cargo.lock

# Build to ensure everything works
cargo build
```

### Step 3: Commit Changes
```bash
git add Cargo.lock
git commit -m "fix: Synchronize Cargo.lock with Cargo.toml version"
```

## Prevention Strategies

### 1. Always Use the Version Bump Script
Never manually edit version numbers. Always use:
```bash
python scripts/bump_version.py --update-all
```

### 2. Pre-commit Hooks (Recommended)
Consider adding a pre-commit hook to verify version synchronization:

```bash
#!/bin/bash
# .git/hooks/pre-commit
CARGO_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
LOCK_VERSION=$(grep -A1 'name = "wfl"' Cargo.lock | grep version | sed 's/version = "\(.*\)"/\1/')

if [ "$CARGO_VERSION" != "$LOCK_VERSION" ]; then
    echo "Error: Version mismatch between Cargo.toml ($CARGO_VERSION) and Cargo.lock ($LOCK_VERSION)"
    echo "Run: cargo update --package wfl"
    exit 1
fi
```

### 3. CI/CD Validation
The CI pipeline should validate version synchronization:

```yaml
- name: Validate version synchronization
  run: |
    CARGO_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    LOCK_VERSION=$(grep -A1 'name = "wfl"' Cargo.lock | grep version | sed 's/version = "\(.*\)"/\1/')
    if [ "$CARGO_VERSION" != "$LOCK_VERSION" ]; then
      echo "Version mismatch detected!"
      exit 1
    fi
```

## Troubleshooting

### Common Issues

1. **Cargo.lock shows old version after Cargo.toml update**
   - Solution: Run `cargo update --package wfl`

2. **package.metadata.bundle version doesn't match main package version**
   - This indicates a regex pattern issue in the version bump script
   - Solution: Run the version bump script to synchronize both fields

3. **Version bump script fails**
   - Check that all required files exist
   - Ensure cargo is in PATH
   - Verify git configuration
   - Check regex patterns for multiline matching issues

4. **CI/CD creates version drift**
   - Ensure the version bump script includes Cargo.lock updates
   - Check that all modified files are committed together
   - Verify regex patterns handle all version fields correctly

### Verification Commands

```bash
# Check current versions across files
echo "Main package: $(grep '^version = ' Cargo.toml | head -1)"
echo "Bundle metadata: $(grep -A10 '\[package\.metadata\.bundle\]' Cargo.toml | grep 'version = ')"
echo "Cargo.lock: $(grep -A1 'name = "wfl"' Cargo.lock | grep version)"
echo "Runtime: $(cargo run -- --version 2>/dev/null | grep version)"

# Test version bump script
python scripts/bump_version.py --skip-bump --update-all --skip-git --verbose
```

## Best Practices

1. **Never edit versions manually** - Always use the automated script
2. **Test locally first** - Run the script with `--skip-git` to test changes
3. **Commit atomically** - All version-related files should be committed together
4. **Validate after changes** - Always run `cargo build` after version updates
5. **Monitor CI/CD** - Watch for version-related failures in automated workflows

## Related Files

- `scripts/bump_version.py` - Main version management script
- `.github/workflows/versioning.yml` - Automated version bump workflow
- `Cargo.toml` - Package configuration
- `Cargo.lock` - Dependency lock file
- `src/version.rs` - Runtime version constant
