#!/usr/bin/env python3
"""Unit tests for scripts/check_repo_hygiene.py (Repository Hygiene checker).

The checker is the enforcement arm of the root Repository Hygiene and Layout
Policy (`REPOSITORY_HYGIENE.md`) with its machine-readable profile
(`.repo-hygiene.toml`). These tests exercise it against small fixture git
trees — one deliberately clean, and one per violation class — so the checker
itself is verified independently of the real repository's state.

Contract under test:

  python3 scripts/check_repo_hygiene.py --mode static --root <dir>
  python3 scripts/check_repo_hygiene.py --mode working-tree --root <dir>

Exit codes: 0 = clean, 1 = violations found (one `HYGIENE:` line each on
stdout), 2 = usage or profile error. The checker is dependency-free (Python
stdlib only) and reads the tracked file list from git.
"""

import json
import hashlib
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
CHECKER = REPO_ROOT / "scripts" / "check_repo_hygiene.py"


def run_checker(mode, root, extra=None):
    cmd = [sys.executable, str(CHECKER), "--mode", mode, "--root", str(root)]
    if extra:
        cmd.extend(extra)
    return subprocess.run(cmd, capture_output=True, text=True)


BASE_PROFILE = """\
schema = 1

[root]
allowed-files = [
    ".gitignore",
    ".repo-hygiene.toml",
    "Cargo.toml",
    "README.md",
    "rustfmt.toml",
]
allowed-dirs = ["Archive", "experiments", "src"]

[forbidden]
suffixes = [".ast.txt", ".lex.txt", ".orig", ".rej", ".vsix", ".log"]
names = ["settings.local.json"]
retired-paths = ["Tools", "Dev diary", "crates/wfl_core"]

[personal-paths]
placeholder-users = ["user", "runner", "username", "example"]
allowed-files = []

[binaries]
max-size-kb = 64
allowed = []

[version]
[[version.sources]]
file = "Cargo.toml"
kind = "cargo-package"

[archive]
root = "Archive"
manifest = "Archive/manifest.json"

[experiments]
root = "experiments"
max-days-without-review = 0

[formatting]
rustfmt = "rustfmt.toml"

[working-tree]
allowed-untracked-roots = ["target/"]
allowed-ignored-roots = ["target/"]
"""


class FixtureTree:
    """A throwaway git repository the checker can be pointed at."""

    def __init__(self):
        self._tmp = tempfile.TemporaryDirectory(prefix="hygiene-fixture-")
        self.root = Path(self._tmp.name)
        self._git("init", "-q")
        self._git("config", "user.email", "fixture@example.invalid")
        self._git("config", "user.name", "Fixture")

    def _git(self, *args):
        subprocess.run(
            ["git", "-C", str(self.root), *args],
            check=True,
            capture_output=True,
        )

    def write(self, relpath, content):
        path = self.root / relpath
        path.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(content, bytes):
            path.write_bytes(content)
        else:
            path.write_text(content)
        return path

    def commit_all(self):
        self._git("add", "-A", "-f")
        self._git("commit", "-q", "-m", "fixture", "--no-verify")

    def cleanup(self):
        self._tmp.cleanup()


def make_clean_tree():
    """The minimal tree that must pass static mode with BASE_PROFILE."""
    t = FixtureTree()
    t.write(".repo-hygiene.toml", BASE_PROFILE)
    t.write(".gitignore", "/target\n")
    t.write("Cargo.toml", '[package]\nname = "wfl"\nversion = "26.7.59"\n')
    t.write("README.md", "# fixture\n")
    t.write("rustfmt.toml", 'max_width = 100\n')
    t.write("src/main.rs", "fn main() {}\n")
    t.commit_all()
    return t


class StaticModeTest(unittest.TestCase):
    def setUp(self):
        self.trees = []

    def tearDown(self):
        for t in self.trees:
            t.cleanup()

    def tree(self):
        t = make_clean_tree()
        self.trees.append(t)
        return t

    def assert_violation(self, tree, needle):
        result = run_checker("static", tree.root)
        self.assertEqual(
            result.returncode,
            1,
            f"expected violations, got rc={result.returncode}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}",
        )
        self.assertIn(needle, result.stdout)

    def test_checker_exists_and_is_tracked_here(self):
        self.assertTrue(CHECKER.is_file(), f"{CHECKER} does not exist")

    def test_clean_tree_passes(self):
        t = self.tree()
        result = run_checker("static", t.root)
        self.assertEqual(
            result.returncode,
            0,
            f"clean fixture should pass\nstdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}",
        )

    def test_root_entry_not_in_allowlist(self):
        t = self.tree()
        t.write("stray_root_file.txt", "should not be here\n")
        t.commit_all()
        self.assert_violation(t, "stray_root_file.txt")

    def test_forbidden_suffix(self):
        t = self.tree()
        t.write("src/dump.ast.txt", "AST dump\n")
        t.commit_all()
        self.assert_violation(t, "dump.ast.txt")

    def test_merge_remnant(self):
        t = self.tree()
        t.write("src/main.rs.orig", "old\n")
        t.commit_all()
        self.assert_violation(t, "main.rs.orig")

    def test_local_settings_file(self):
        t = self.tree()
        t.write("src/.claude/settings.local.json", "{}\n")
        t.commit_all()
        self.assert_violation(t, "settings.local.json")

    def test_retired_path_present(self):
        t = self.tree()
        t.write("Tools/leftover.py", "print('hi')\n")
        t.commit_all()
        self.assert_violation(t, "Tools")

    def test_undeclared_binary_magic_bytes(self):
        t = self.tree()
        t.write("src/mystery", b"\x7fELF\x02\x01\x01\0" + b"\0" * 64)
        t.commit_all()
        self.assert_violation(t, "mystery")

    def test_declared_binary_is_allowed(self):
        t = self.tree()
        profile = BASE_PROFILE.replace(
            'allowed = []\n\n[version]',
            'allowed = ["src/logo.png"]\n\n[version]',
        )
        t.write(".repo-hygiene.toml", profile)
        t.write("src/logo.png", b"\x89PNG\r\n\x1a\n" + b"\0" * 32)
        t.commit_all()
        result = run_checker("static", t.root)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_oversized_file(self):
        t = self.tree()
        t.write("src/huge.rs", "// x\n" * 40_000)  # > 64 KiB of text
        t.commit_all()
        self.assert_violation(t, "huge.rs")

    def test_personal_absolute_path(self):
        t = self.tree()
        t.write("src/notes.rs", '// see C:\\Users\\bradpe\\wfl\\notes.txt\n')
        t.commit_all()
        self.assert_violation(t, "notes.rs")

    def test_placeholder_user_path_is_fine(self):
        t = self.tree()
        t.write("src/doc.rs", "// e.g. /home/user/wfl or /home/runner/work\n")
        t.commit_all()
        result = run_checker("static", t.root)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_generated_exception_requires_full_metadata(self):
        t = self.tree()
        profile = BASE_PROFILE + (
            '\n[[generated]]\npath = "src/gen.rs"\ngenerator = "gen.py"\n'
            # consumer / owner / justification / drift-check / regeneration
            # deliberately missing
        )
        t.write(".repo-hygiene.toml", profile)
        t.write("src/gen.rs", "// generated\n")
        t.commit_all()
        result = run_checker("static", t.root)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("gen.rs", result.stdout)

    def test_archive_file_missing_from_manifest(self):
        t = self.tree()
        t.write("Archive/README.md", "Non-normative archive.\n")
        t.write("Archive/manifest.json", json.dumps({"entries": []}))
        t.write("Archive/old/plan.md", "ancient plan\n")
        t.commit_all()
        self.assert_violation(t, "Archive/old/plan.md")

    def test_archive_checksum_drift(self):
        t = self.tree()
        content = "ancient plan\n"
        entry = {
            "path": "Archive/old/plan.md",
            "original_path": "plan.md",
            "source_commit": "0" * 40,
            "sha256": hashlib.sha256(b"different content").hexdigest(),
            "document_date": "2026-01-01",
            "archived_at": "2026-07-31",
            "kind": "plan",
            "status_at_archive": "completed",
            "reason": "superseded",
            "normative": False,
            "superseded_by": None,
            "related_issues_or_prs": [],
            "security_classification": "public",
        }
        t.write("Archive/README.md", "Non-normative archive.\n")
        t.write("Archive/manifest.json", json.dumps({"entries": [entry]}))
        t.write("Archive/old/plan.md", content)
        t.commit_all()
        self.assert_violation(t, "sha256")

    def test_archive_valid_manifest_passes(self):
        t = self.tree()
        content = "ancient plan\n"
        entry = {
            "path": "Archive/old/plan.md",
            "original_path": "plan.md",
            "source_commit": "0" * 40,
            "sha256": hashlib.sha256(content.encode()).hexdigest(),
            "document_date": "2026-01-01",
            "archived_at": "2026-07-31",
            "kind": "plan",
            "status_at_archive": "completed",
            "reason": "superseded",
            "normative": False,
            "superseded_by": None,
            "related_issues_or_prs": [],
            "security_classification": "public",
        }
        t.write("Archive/README.md", "Non-normative archive.\n")
        t.write("Archive/manifest.json", json.dumps({"entries": [entry]}))
        t.write("Archive/old/plan.md", content)
        t.commit_all()
        result = run_checker("static", t.root)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_experiment_missing_metadata(self):
        t = self.tree()
        t.write("experiments/probe/probe.wfl", "display \"hi\"\n")
        t.commit_all()
        self.assert_violation(t, "experiments/probe")

    def test_experiment_expired_review(self):
        t = self.tree()
        t.write(
            "experiments/probe/README.md",
            "# Probe\n\n- Owner: Brad\n- Status: active\n"
            "- Issue: https://github.com/WebFirstLanguage/wfl/issues/1\n"
            "- Created: 2026-01-01\n- Review-by: 2026-01-02\n"
            "- Exit criteria: proves the thing\n",
        )
        t.write("experiments/probe/probe.wfl", "display \"hi\"\n")
        t.commit_all()
        self.assert_violation(t, "Review-by")

    def test_experiment_current_review_passes(self):
        t = self.tree()
        t.write(
            "experiments/probe/README.md",
            "# Probe\n\n- Owner: Brad\n- Status: active\n"
            "- Issue: https://github.com/WebFirstLanguage/wfl/issues/1\n"
            "- Created: 2026-01-01\n- Review-by: 2099-01-01\n"
            "- Exit criteria: proves the thing\n",
        )
        t.write("experiments/probe/probe.wfl", "display \"hi\"\n")
        t.commit_all()
        result = run_checker("static", t.root)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_multiple_rustfmt_configs(self):
        t = self.tree()
        t.write(".rustfmt.toml", "edition = \"2021\"\n")
        # .rustfmt.toml is also not on the root allowlist, but the dedicated
        # rule must fire regardless of allowlisting.
        t.commit_all()
        self.assert_violation(t, "rustfmt")

    def test_version_drift(self):
        t = self.tree()
        profile = BASE_PROFILE + (
            '\n[[version.sources]]\nfile = "src/version.rs"\nkind = "rust-const"\n'
        )
        t.write(".repo-hygiene.toml", profile)
        t.write("src/version.rs", 'pub const VERSION: &str = "26.7.58";\n')
        t.commit_all()
        self.assert_violation(t, "version")

    def test_version_agreement_passes(self):
        t = self.tree()
        profile = BASE_PROFILE + (
            '\n[[version.sources]]\nfile = "src/version.rs"\nkind = "rust-const"\n'
            '\n[[version.sources]]\nfile = ".build_meta.json"\nkind = "build-meta-json"\n'
            '\n[[version.sources]]\nfile = "wix.toml"\nkind = "wix-toml"\n'
        )
        # .build_meta.json and wix.toml need root allowlisting in the fixture.
        profile = profile.replace(
            '"rustfmt.toml",\n]',
            '"rustfmt.toml",\n    ".build_meta.json",\n    "wix.toml",\n]',
        )
        t.write(".repo-hygiene.toml", profile)
        t.write("src/version.rs", 'pub const VERSION: &str = "26.7.59";\n')
        t.write(".build_meta.json", '{"year": 26, "month": 7, "build": 59}\n')
        t.write("wix.toml", '[package]\nversion = "26.7.59.0"\n')
        t.commit_all()
        result = run_checker("static", t.root)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


class WorkingTreeModeTest(unittest.TestCase):
    def setUp(self):
        self.trees = []

    def tearDown(self):
        for t in self.trees:
            t.cleanup()

    def tree(self):
        t = make_clean_tree()
        self.trees.append(t)
        return t

    def test_clean_working_tree_passes(self):
        t = self.tree()
        result = run_checker("working-tree", t.root)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_modified_tracked_file_fails(self):
        t = self.tree()
        (t.root / "README.md").write_text("# fixture (scribbled on by a test)\n")
        result = run_checker("working-tree", t.root)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("README.md", result.stdout)

    def test_untracked_file_outside_approved_roots_fails(self):
        t = self.tree()
        (t.root / "src" / "leak.txt").write_text("test droppings\n")
        result = run_checker("working-tree", t.root)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("leak.txt", result.stdout)

    def test_untracked_file_inside_approved_root_passes(self):
        t = self.tree()
        (t.root / "target").mkdir()
        (t.root / "target" / "artifact.bin").write_bytes(b"\0\1\2")
        result = run_checker("working-tree", t.root)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_ignored_file_outside_approved_roots_fails(self):
        t = self.tree()
        # .gitignore in the clean tree ignores /target only; extend it so an
        # ignored file can exist outside the approved ignored roots.
        (t.root / ".gitignore").write_text("/target\n*.scratch\n")
        subprocess.run(
            ["git", "-C", str(t.root), "add", ".gitignore"],
            check=True, capture_output=True,
        )
        subprocess.run(
            ["git", "-C", str(t.root), "commit", "-q", "-m", "ignore scratch",
             "--no-verify"],
            check=True, capture_output=True,
        )
        (t.root / "src" / "junk.scratch").write_text("ignored droppings\n")
        result = run_checker("working-tree", t.root)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("junk.scratch", result.stdout)


if __name__ == "__main__":
    unittest.main()
