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
CARGO_LOCK = "Cargo.lock"
FUZZ_CARGO_LOCK = os.path.join("fuzz", "Cargo.lock")
WIX_TOML = "wix.toml"
VSCODE_EXTENSION_DIRS = ["vscode-extension", "vscode-wfl", "editors/vscode-wfl"]
MODIFIED_FILES = []

VERSION_TAG_RE = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")
VERSION_RE = re.compile(r"^(\d+)\.(\d+)\.(\d+)$")


def parse_args(argv=None):
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(description="Update WFL version numbers across the project.")
    parser.add_argument("--skip-bump", action="store_true", help="Skip incrementing the build number")
    parser.add_argument("--update-all", action="store_true", help="Update all version files")
    parser.add_argument("--update-wix-only", action="store_true", help="Only update wix.toml")
    parser.add_argument("--skip-git", action="store_true", help="Skip git commit")
    parser.add_argument("--verbose", action="store_true", help="Show detailed output")
    parser.add_argument(
        "--from-tags",
        action="store_true",
        help="Compute the next YY.M.BUILD from published v-tags and the same-month floor",
    )
    parser.add_argument(
        "--print",
        dest="print_version",
        action="store_true",
        help="Print the computed or current version to stdout and do not write files",
    )
    parser.add_argument(
        "--set-version",
        metavar="X.Y.Z",
        help="Write this exact version into the version mirrors without incrementing",
    )
    return parser.parse_args(argv)


def utc_now():
    return datetime.datetime.now(datetime.timezone.utc)


def calendar_parts(now=None):
    """Return (YY, M, YYYY-MM-DD) from a UTC timestamp.

    Naive datetimes are treated as UTC so a nightly just after midnight UTC on
    the 1st uses the new month — the same calendar date as nightly-YYYY-MM-DD.
    """
    if now is None:
        now = utc_now()
    elif now.tzinfo is None:
        now = now.replace(tzinfo=datetime.timezone.utc)
    else:
        now = now.astimezone(datetime.timezone.utc)
    return now.year % 100, now.month, now.strftime("%Y-%m-%d")


def nightly_tag_date(now=None):
    return calendar_parts(now)[2]


def parse_version(version):
    match = VERSION_RE.fullmatch(version)
    if not match:
        print(f"Error: invalid version {version!r}, expected YY.M.N", file=sys.stderr)
        sys.exit(1)
    return int(match.group(1)), int(match.group(2)), int(match.group(3))


def version_tag_names_from_ls_remote(output):
    names = []
    seen = set()
    for line in output.splitlines():
        parts = line.split()
        if len(parts) < 2:
            continue
        ref = parts[1]
        if ref.endswith("^{}"):
            continue
        if not ref.startswith("refs/tags/"):
            continue
        name = ref[len("refs/tags/"):]
        if VERSION_TAG_RE.match(name) and name not in seen:
            seen.add(name)
            names.append(name)
    return names


def head_version_tags_from_ls_remote(output, head_sha):
    peeled = {}
    unpeeled = {}
    for line in output.splitlines():
        parts = line.split()
        if len(parts) < 2:
            continue
        sha, ref = parts[0], parts[1]
        is_peeled = ref.endswith("^{}")
        if is_peeled:
            ref = ref[:-3]
        if not ref.startswith("refs/tags/"):
            continue
        name = ref[len("refs/tags/"):]
        if not VERSION_TAG_RE.match(name):
            continue
        if is_peeled:
            peeled[name] = sha
        else:
            unpeeled[name] = sha
    matches = []
    for name, sha in unpeeled.items():
        commit = peeled.get(name, sha)
        if commit == head_sha:
            matches.append(name)
    return matches


def ls_remote_tags():
    for target in ("origin", "."):
        result = subprocess.run(
            ["git", "ls-remote", "--tags", target],
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            return result.stdout
    print("Error: git ls-remote --tags failed", file=sys.stderr)
    sys.exit(1)


def read_floor_meta(path=None):
    path = path or BUILD_META_FILE
    if not os.path.exists(path):
        return None
    with open(path, "r") as handle:
        try:
            meta = json.load(handle)
        except json.JSONDecodeError:
            print(f"Error: {path} is not valid JSON", file=sys.stderr)
            sys.exit(1)
    return {
        "year": int(meta.get("year", 0)),
        "month": int(meta.get("month", 0)),
        "build": int(meta.get("build", 0)),
    }


def discover_tag_state():
    output = ls_remote_tags()
    try:
        head = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    except subprocess.CalledProcessError as exc:
        print(f"Error: cannot resolve HEAD: {exc}", file=sys.stderr)
        sys.exit(1)
    return (
        version_tag_names_from_ls_remote(output),
        head_version_tags_from_ls_remote(output, head),
        read_floor_meta(),
    )


def next_version_from_tags(now=None, tag_names=None, floor=None, head_version_tags=None):
    """Next YY.M.BUILD for the current UTC month, or the v-tag already on HEAD."""
    if tag_names is None or floor is None or head_version_tags is None:
        discovered_names, discovered_head, discovered_floor = discover_tag_state()
        if tag_names is None:
            tag_names = discovered_names
        if head_version_tags is None:
            head_version_tags = discovered_head
        if floor is None:
            floor = discovered_floor

    for name in head_version_tags or []:
        match = VERSION_TAG_RE.match(name)
        if match:
            return f"{int(match.group(1))}.{int(match.group(2))}.{int(match.group(3))}"

    year, month, _ = calendar_parts(now)
    max_n = 0
    for name in tag_names or []:
        match = VERSION_TAG_RE.match(name)
        if not match:
            continue
        tag_year, tag_month, tag_build = (int(match.group(1)), int(match.group(2)), int(match.group(3)))
        if tag_year == year and tag_month == month:
            max_n = max(max_n, tag_build)
    next_n = max_n + 1 if max_n else 1
    if floor:
        floor_year = int(floor.get("year", 0))
        floor_month = int(floor.get("month", 0))
        floor_build = int(floor.get("build", 0))
        if floor_year == year and floor_month == month:
            next_n = max(next_n, floor_build)
    return f"{year}.{month}.{next_n}"


def write_version_mirrors(version):
    """Write .build_meta.json and src/version.rs for an exact version."""
    year, month, build = parse_version(version)
    meta = {}
    if os.path.exists(BUILD_META_FILE):
        with open(BUILD_META_FILE, "r") as handle:
            try:
                meta = json.load(handle)
            except json.JSONDecodeError:
                print(f"Error: {BUILD_META_FILE} is not valid JSON", file=sys.stderr)
                sys.exit(1)
    meta["year"] = year
    meta["month"] = month
    meta["build"] = build
    with open(BUILD_META_FILE, "w") as handle:
        json.dump(meta, handle, indent=2)
    MODIFIED_FILES.append(BUILD_META_FILE)

    directory = os.path.dirname(VERSION_FILE)
    if directory:
        os.makedirs(directory, exist_ok=True)
    with open(VERSION_FILE, "w") as handle:
        handle.write(f'pub const VERSION: &str = "{version}";\n')
    MODIFIED_FILES.append(VERSION_FILE)
    return meta, version


def rewrite_wfl_lock_version(lock_path, version):
    """Rewrite the pinned `wfl` package version in a Cargo.lock file."""
    if not os.path.exists(lock_path):
        print(f"Warning: {lock_path} not found, skipping")
        return False
    with open(lock_path, "r") as handle:
        content = handle.read()
    new_content, count = re.subn(
        r'(name = "wfl"\s*version = ")[^"]+(")',
        rf"\g<1>{version}\2",
        content,
        count=1,
        flags=re.DOTALL,
    )
    if count != 1:
        print(f"Error: Could not update wfl version in {lock_path}", file=sys.stderr)
        sys.exit(1)
    with open(lock_path, "w") as handle:
        handle.write(new_content)
    MODIFIED_FILES.append(lock_path)
    print(f"Updated {lock_path} to {version}")
    return True

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
    
    current_year, current_month, _ = calendar_parts()
    
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

def _extract_wfl_lock_version(lock_path):
    """Return the pinned `wfl` package version from a Cargo.lock file.

    Shared by `update_cargo_lock` (root) and `update_fuzz_cargo_lock` (the
    standalone fuzz workspace) so the `[[package]] name = "wfl"` parse can't
    drift out of sync between them. Exits (SystemExit) if the file can't be read
    or the `wfl` entry is absent, so a malformed/missing lock fails the bump.
    """
    try:
        with open(lock_path, "r") as f:
            content = f.read()
    except OSError as e:
        print(f"Error reading {lock_path}: {e}")
        sys.exit(1)

    match = re.search(
        r'\[\[package\]\]\s*name = "wfl"\s*version = "([^"]+)"',
        content,
        re.DOTALL,
    )
    if not match:
        print(f"Error: Could not find WFL package version in {lock_path}")
        sys.exit(1)
    return match.group(1)

def update_cargo_lock(expected_version):
    """Rewrite the root Cargo.lock `wfl` version to match Cargo.toml."""
    rewrite_wfl_lock_version(CARGO_LOCK, expected_version)
    actual_version = _extract_wfl_lock_version(CARGO_LOCK)
    if actual_version != expected_version:
        print("Error: Version mismatch!")
        print(f"  expected: {expected_version}")
        print(f"  Cargo.lock version: {actual_version}")
        sys.exit(1)
    print(f"✓ Cargo.lock synchronized: {expected_version}")


def update_fuzz_cargo_lock(expected_version):
    """Rewrite the standalone fuzz workspace's Cargo.lock `wfl` version.

    `fuzz/` is a separate cargo workspace that path-depends on root `wfl`, so
    its lock pins the root version too. A path-dep version change is a string
    rewrite — no dependency resolution — and keeps `cargo check --locked
    --manifest-path fuzz/Cargo.toml` honest without shelling out to Cargo.
    """
    if not os.path.exists(FUZZ_CARGO_LOCK):
        print(f"Note: {FUZZ_CARGO_LOCK} not present; skipping fuzz lock sync.")
        return
    rewrite_wfl_lock_version(FUZZ_CARGO_LOCK, expected_version)
    fuzz_version = _extract_wfl_lock_version(FUZZ_CARGO_LOCK)
    if fuzz_version != expected_version:
        print("Error: fuzz/Cargo.lock version mismatch!")
        print(f"  expected: {expected_version}")
        print(f"  fuzz/Cargo.lock: {fuzz_version}")
        sys.exit(1)
    print(f"✓ fuzz/Cargo.lock synchronized: {expected_version}")

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

        # package-lock.json mirrors the version twice (top level and the ""
        # root package entry); leaving it behind causes the version drift the
        # repo-hygiene checker rejects.
        lock_file = os.path.join(ext_dir, "package-lock.json")
        if os.path.exists(lock_file):
            print(f"Updating {lock_file}...")
            with open(lock_file, "r") as f:
                try:
                    lock_data = json.load(f)
                except json.JSONDecodeError:
                    print(f"Warning: {lock_file} is not valid JSON, skipping")
                    lock_data = None
            if lock_data is not None:
                lock_data["version"] = semver_version
                root_pkg = lock_data.get("packages", {}).get("", None)
                if root_pkg is not None:
                    root_pkg["version"] = semver_version
                with open(lock_file, "w") as f:
                    json.dump(lock_data, f, indent=2)
                    f.write("\n")
                MODIFIED_FILES.append(lock_file)

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

def _apply_all_mirrors(version):
    update_cargo_toml(version)
    update_cargo_lock(version)
    update_fuzz_cargo_lock(version)
    update_vscode_extensions(version)
    update_wix_toml(version)
    print(f"Updated all version references to {version}", file=sys.stderr)


def main_with_args(argv=None):
    args = parse_args(argv)

    if args.from_tags and args.set_version:
        print("Error: --from-tags and --set-version cannot be combined", file=sys.stderr)
        return 2

    if args.from_tags:
        version = next_version_from_tags()
        if args.print_version:
            print(version)
            return 0
        meta, version = write_version_mirrors(version)
    elif args.set_version:
        version = args.set_version
        if args.print_version:
            print(version)
            return 0
        meta, version = write_version_mirrors(version)
    elif args.update_wix_only:
        meta, version = get_current_version()
        update_wix_toml(version)
        print(f"Updated wix.toml with version {version}", file=sys.stderr)
        return 0
    else:
        if args.print_version:
            _meta, version = get_current_version()
            print(version)
            return 0
        meta, version = bump_version(args.skip_bump)

    if args.update_all:
        _apply_all_mirrors(version)

    if not args.skip_git:
        if not commit_changes(version, args.skip_git):
            return 1

    return 0


def main():
    return main_with_args()


if __name__ == "__main__":
    sys.exit(main())
