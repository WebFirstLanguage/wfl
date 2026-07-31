#!/usr/bin/env python3
"""Repository hygiene checker — enforcement arm of REPOSITORY_HYGIENE.md.

Reads the machine-readable profile (`.repo-hygiene.toml`) and verifies the
repository against the Repository Hygiene and Layout Policy. The prose policy
at the repository root is authoritative; this checker proves compliance.

Modes:
  static        Verify the *tracked* tree: root allowlist, forbidden
                suffixes/names, retired paths, undeclared binaries and
                oversized files, personal absolute paths, generated-file
                exception metadata, archive manifest integrity (checksums,
                required fields, non-normativity), experiment metadata and
                review expiry, single rustfmt configuration, and product
                version agreement across declared sources.
  working-tree  Verify the tree is clean after generators/test suites ran:
                no modified tracked files, no untracked or newly ignored
                files outside the approved output roots.

Exit codes: 0 clean, 1 violations (one `HYGIENE:` line each), 2 usage or
profile error. Dependency-free: Python 3.11+ standard library only.
"""

import argparse
import datetime
import fnmatch
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

try:
    import tomllib
except ImportError:  # pragma: no cover
    print("HYGIENE-ERROR: Python 3.11+ with tomllib is required", file=sys.stderr)
    sys.exit(2)

ARCHIVE_MANIFEST_REQUIRED_KEYS = [
    "path",
    "original_path",
    "source_commit",
    "sha256",
    "document_date",
    "archived_at",
    "kind",
    "status_at_archive",
    "reason",
    "normative",
    "superseded_by",
    "related_issues_or_prs",
    "security_classification",
]

GENERATED_REQUIRED_KEYS = [
    "path",
    "generator",
    "consumer",
    "owner",
    "justification",
    "drift-check",
    "regeneration",
]

EXPERIMENT_REQUIRED_FIELDS = [
    "Owner",
    "Status",
    "Issue",
    "Created",
    "Review-by",
    "Exit criteria",
]

# Magic prefixes that mark a file as binary even before the NUL-byte sniff.
BINARY_MAGIC = [
    b"\x7fELF",          # ELF executables/objects
    b"MZ",               # PE/DOS executables
    b"\xca\xfe\xba\xbe",  # Mach-O fat / Java class
    b"\xcf\xfa\xed\xfe",  # Mach-O 64-bit
    b"PK\x03\x04",       # zip containers (vsix, docx, jar, ...)
]

PERSONAL_PATH_RE = re.compile(
    r"(?:/home/([A-Za-z0-9_.-]+)|/Users/([A-Za-z0-9_.-]+)|[A-Za-z]:\\+Users\\+([A-Za-z0-9_.-]+))"
)


class Report:
    def __init__(self):
        self.violations = []

    def add(self, rule, path, detail):
        self.violations.append(f"HYGIENE: {rule}: {path}: {detail}")

    def emit(self):
        for line in self.violations:
            print(line)
        return 1 if self.violations else 0


def git(root, *args, check=True):
    result = subprocess.run(
        ["git", "-C", str(root), *args],
        capture_output=True,
        check=check,
    )
    return result.stdout


def tracked_files(root):
    out = git(root, "ls-files", "-z")
    return [p.decode("utf-8", "surrogateescape") for p in out.split(b"\0") if p]


def load_profile(root, profile_path):
    path = Path(profile_path) if profile_path else Path(root) / ".repo-hygiene.toml"
    if not path.is_file():
        print(f"HYGIENE-ERROR: profile not found: {path}", file=sys.stderr)
        sys.exit(2)
    with open(path, "rb") as f:
        try:
            return tomllib.load(f)
        except tomllib.TOMLDecodeError as e:
            print(f"HYGIENE-ERROR: invalid profile {path}: {e}", file=sys.stderr)
            sys.exit(2)


def read_head(root, relpath, limit=8192):
    try:
        with open(Path(root) / relpath, "rb") as f:
            return f.read(limit)
    except OSError:
        return None


def is_binary(head):
    if head is None:
        return False
    for magic in BINARY_MAGIC:
        if head.startswith(magic):
            return True
    return b"\0" in head


def matches_any(path, patterns):
    return any(fnmatch.fnmatch(path, pat) for pat in patterns)


# ---------------------------------------------------------------------------
# static mode
# ---------------------------------------------------------------------------

def check_root_allowlist(report, files, profile):
    allowed_files = set(profile.get("root", {}).get("allowed-files", []))
    allowed_dirs = set(profile.get("root", {}).get("allowed-dirs", []))
    seen = set()
    for f in files:
        top = f.split("/", 1)[0]
        if top in seen:
            continue
        seen.add(top)
        if "/" in f:
            if top not in allowed_dirs:
                report.add("root-allowlist", top,
                           "tracked root directory is not on the allowlist")
        else:
            if top not in allowed_files:
                report.add("root-allowlist", top,
                           "tracked root file is not on the allowlist")


def check_forbidden(report, files, profile):
    forbidden = profile.get("forbidden", {})
    suffixes = forbidden.get("suffixes", [])
    names = forbidden.get("names", [])
    retired = forbidden.get("retired-paths", [])
    exceptions = set(forbidden.get("allowed", []))
    flagged_retired = set()
    for f in files:
        if f in exceptions:
            continue
        base = f.rsplit("/", 1)[-1]
        for suf in suffixes:
            if f.endswith(suf):
                report.add("forbidden-suffix", f,
                           f"tracked files may not end in {suf}")
                break
        if base in names:
            report.add("forbidden-name", f,
                       f"'{base}' is local/ephemeral and must not be tracked")
        for r in retired:
            if (f == r or f.startswith(r.rstrip("/") + "/")) and r not in flagged_retired:
                flagged_retired.add(r)
                report.add("retired-path", r,
                           "retired path still present in the tracked tree")


def check_binaries_and_size(report, root, files, profile):
    binaries = profile.get("binaries", {})
    allowed = binaries.get("allowed", [])
    size_allowed = binaries.get("size-allowed", [])
    max_kb = binaries.get("max-size-kb", 400)
    for f in files:
        p = Path(root) / f
        if not p.is_file():
            continue
        if matches_any(f, allowed):
            continue
        head = read_head(root, f)
        if is_binary(head):
            report.add("undeclared-binary", f,
                       "binary content requires a [binaries] allowed entry "
                       "in .repo-hygiene.toml")
            continue
        if matches_any(f, size_allowed):
            continue
        size = p.stat().st_size
        if size > max_kb * 1024:
            report.add("oversized-file", f,
                       f"{size} bytes exceeds the {max_kb} KiB limit and has "
                       "no [binaries] allowed entry")


def check_personal_paths(report, root, files, profile):
    cfg = profile.get("personal-paths", {})
    placeholders = set(cfg.get("placeholder-users", []))
    allowed_files = cfg.get("allowed-files", [])
    for f in files:
        if matches_any(f, allowed_files):
            continue
        head = read_head(root, f, limit=1 << 20)
        if head is None or is_binary(head[:8192]):
            continue
        text = head.decode("utf-8", "replace")
        for m in PERSONAL_PATH_RE.finditer(text):
            user = next(g for g in m.groups() if g)
            if user not in placeholders:
                report.add("personal-path", f,
                           f"contains personal absolute path '{m.group(0)}'")
                break


def check_generated_exceptions(report, root, files, profile):
    tracked = set(files)
    for entry in profile.get("generated", []):
        path = entry.get("path", "<missing path>")
        missing = [k for k in GENERATED_REQUIRED_KEYS if not entry.get(k)]
        if missing:
            report.add("generated-exception", path,
                       f"exception metadata incomplete, missing: {', '.join(missing)}")
        if entry.get("path") and entry["path"] not in tracked:
            report.add("generated-exception", path,
                       "declared generated file is not tracked")


def check_archive(report, root, files, profile):
    cfg = profile.get("archive", {})
    archive_root = cfg.get("root", "Archive")
    manifest_rel = cfg.get("manifest", f"{archive_root}/manifest.json")
    archived = [f for f in files
                if f.startswith(archive_root + "/")
                and f != manifest_rel
                and f != f"{archive_root}/README.md"]
    if not archived and manifest_rel not in files:
        return
    manifest_path = Path(root) / manifest_rel
    if not manifest_path.is_file():
        report.add("archive-manifest", manifest_rel,
                   "archive manifest is missing")
        return
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        report.add("archive-manifest", manifest_rel, f"unreadable manifest: {e}")
        return
    entries = manifest.get("entries", [])
    by_path = {}
    for entry in entries:
        path = entry.get("path", "<missing path>")
        by_path[path] = entry
        missing = [k for k in ARCHIVE_MANIFEST_REQUIRED_KEYS if k not in entry]
        if missing:
            report.add("archive-manifest", path,
                       f"entry missing required fields: {', '.join(missing)}")
            continue
        if entry["normative"] is not False:
            report.add("archive-manifest", path,
                       "archived content must declare normative: false")
        target = Path(root) / path
        if not target.is_file():
            report.add("archive-manifest", path,
                       "manifest entry points at a file that does not exist")
            continue
        digest = hashlib.sha256(target.read_bytes()).hexdigest()
        if digest != entry["sha256"]:
            report.add("archive-manifest", path,
                       f"sha256 drift: manifest {entry['sha256'][:12]}… vs "
                       f"actual {digest[:12]}…")
    for f in archived:
        if f not in by_path:
            report.add("archive-manifest", f,
                       "archived file has no manifest entry")


def check_experiments(report, root, files, profile):
    cfg = profile.get("experiments", {})
    exp_root = cfg.get("root", "experiments")
    dirs = sorted({f.split("/")[1] for f in files
                   if f.startswith(exp_root + "/") and f.count("/") >= 2})
    # Files directly under experiments/ (not in a topic dir) are misplaced.
    for f in files:
        if f.startswith(exp_root + "/") and f.count("/") == 1:
            report.add("experiment-metadata", f,
                       "experiments must live in experiments/<topic>/ with a README.md")
    today = datetime.date.today()
    for d in dirs:
        readme_rel = f"{exp_root}/{d}/README.md"
        readme = Path(root) / readme_rel
        if readme_rel not in files or not readme.is_file():
            report.add("experiment-metadata", f"{exp_root}/{d}",
                       "missing README.md with owner/status/issue/created/"
                       "review-by/exit criteria")
            continue
        text = readme.read_text(encoding="utf-8", errors="replace")
        missing = [field for field in EXPERIMENT_REQUIRED_FIELDS
                   if not re.search(rf"^[-*\s]*{re.escape(field)}\s*:", text,
                                    re.MULTILINE | re.IGNORECASE)]
        if missing:
            report.add("experiment-metadata", readme_rel,
                       f"missing required fields: {', '.join(missing)}")
            continue
        m = re.search(r"^[-*\s]*Review-by\s*:\s*(\d{4}-\d{2}-\d{2})", text,
                      re.MULTILINE | re.IGNORECASE)
        if not m:
            report.add("experiment-metadata", readme_rel,
                       "Review-by must be a YYYY-MM-DD date")
            continue
        review_by = datetime.date.fromisoformat(m.group(1))
        if review_by < today:
            report.add("experiment-expired", readme_rel,
                       f"Review-by {review_by} has passed: promote, archive, "
                       "or remove the experiment")


def check_formatting(report, files, profile):
    canonical = profile.get("formatting", {}).get("rustfmt", "rustfmt.toml")
    configs = [f for f in files if f in ("rustfmt.toml", ".rustfmt.toml")]
    if len(configs) > 1:
        report.add("rustfmt-config", ", ".join(sorted(configs)),
                   "multiple Rust formatting configurations are tracked; "
                   f"keep only {canonical}")
    elif configs and configs[0] != canonical:
        report.add("rustfmt-config", configs[0],
                   f"the canonical rustfmt configuration is {canonical}")


def _read_version(root, source):
    file = source.get("file")
    kind = source.get("kind")
    path = Path(root) / (file or "")
    if not file or not kind:
        return None, "version source needs both 'file' and 'kind'"
    if not path.is_file():
        return None, f"version source file not found: {file}"
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as e:
        return None, f"unreadable version source {file}: {e}"
    if kind == "cargo-package":
        m = re.search(r'^\[package\][^\[]*?^version\s*=\s*"([^"]+)"',
                      text, re.MULTILINE | re.DOTALL)
        return (m.group(1), None) if m else (None, f"no [package] version in {file}")
    if kind == "rust-const":
        m = re.search(r'VERSION:\s*&str\s*=\s*"([^"]+)"', text)
        return (m.group(1), None) if m else (None, f"no VERSION constant in {file}")
    if kind == "build-meta-json":
        try:
            meta = json.loads(text)
            return f"{meta['year']}.{meta['month']}.{meta['build']}", None
        except (json.JSONDecodeError, KeyError) as e:
            return None, f"bad build meta {file}: {e}"
    if kind == "wix-toml":
        m = re.search(r'^version\s*=\s*"(\d+\.\d+\.\d+)(?:\.\d+)?"', text, re.MULTILINE)
        return (m.group(1), None) if m else (None, f"no version in {file}")
    if kind == "npm-package":
        try:
            return json.loads(text).get("version"), None
        except json.JSONDecodeError as e:
            return None, f"bad JSON in {file}: {e}"
    if kind == "npm-lock":
        try:
            data = json.loads(text)
        except json.JSONDecodeError as e:
            return None, f"bad JSON in {file}: {e}"
        top = data.get("version")
        pkg = data.get("packages", {}).get("", {}).get("version")
        if top and pkg and top != pkg:
            return None, f"{file} disagrees with itself ({top} vs {pkg})"
        return top or pkg, None
    if kind == "cargo-lock-wfl":
        m = re.search(r'name = "wfl"\nversion = "([^"]+)"', text)
        return (m.group(1), None) if m else (None, f"no wfl package in {file}")
    return None, f"unknown version source kind '{kind}' for {file}"


def check_versions(report, root, profile):
    sources = profile.get("version", {}).get("sources", [])
    if len(sources) < 2:
        return
    readings = []
    for source in sources:
        value, err = _read_version(root, source)
        if err:
            report.add("version-drift", source.get("file", "<unknown>"), err)
        elif value:
            readings.append((source["file"], value))
    if not readings:
        return
    canonical_file, canonical = readings[0]
    for file, value in readings[1:]:
        if value != canonical:
            report.add("version-drift", file,
                       f"version {value} disagrees with {canonical_file} "
                       f"({canonical})")


def run_static(root, profile):
    report = Report()
    files = tracked_files(root)
    check_root_allowlist(report, files, profile)
    check_forbidden(report, files, profile)
    check_binaries_and_size(report, root, files, profile)
    check_personal_paths(report, root, files, profile)
    check_generated_exceptions(report, root, files, profile)
    check_archive(report, root, files, profile)
    check_experiments(report, root, files, profile)
    check_formatting(report, files, profile)
    check_versions(report, root, profile)
    return report.emit()


# ---------------------------------------------------------------------------
# working-tree mode
# ---------------------------------------------------------------------------

def _allowed_by_roots(path, roots):
    for r in roots:
        prefix = r.rstrip("/") + "/"
        if path == r or path.startswith(prefix) or path.rstrip("/") + "/" == prefix:
            return True
    return False


def run_working_tree(root, profile):
    report = Report()
    cfg = profile.get("working-tree", {})
    untracked_ok = cfg.get("allowed-untracked-roots", [])
    ignored_ok = cfg.get("allowed-ignored-roots", [])
    out = git(root, "status", "--porcelain=v1", "--ignored", "-z")
    for record in out.split(b"\0"):
        if not record:
            continue
        entry = record.decode("utf-8", "surrogateescape")
        if len(entry) < 4:
            continue
        status, path = entry[:2], entry[3:]
        if status == "??":
            if not _allowed_by_roots(path, untracked_ok):
                report.add("dirty-tree", path,
                           "untracked file outside the approved output roots — "
                           "a generator or test wrote where it must not")
        elif status == "!!":
            if not _allowed_by_roots(path, ignored_ok):
                report.add("dirty-tree", path,
                           "ignored file outside the approved output roots")
        else:
            report.add("dirty-tree", path,
                       f"tracked file modified during the run (status '{status}')")
    return report.emit()


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--mode", choices=["static", "working-tree"],
                        required=True)
    parser.add_argument("--root", default=".",
                        help="repository root to check (default: cwd)")
    parser.add_argument("--profile", default=None,
                        help="profile path (default: <root>/.repo-hygiene.toml)")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    if not (root / ".git").exists():
        print(f"HYGIENE-ERROR: {root} is not a git repository", file=sys.stderr)
        return 2
    profile = load_profile(root, args.profile)

    if args.mode == "static":
        return run_static(root, profile)
    return run_working_tree(root, profile)


if __name__ == "__main__":
    sys.exit(main())
