"""Guards for tag-based nightly versioning and no protected-branch version pushes."""

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def job(text, name):
    match = re.search(rf"^  {re.escape(name)}:\n(.*?)(?=^  [\w-]+:|\Z)", text, re.M | re.S)
    if not match:
        raise AssertionError(f"Required workflow job {name!r} is missing")
    return match.group(1)


def required_check_names(ci_text, config_lint_text):
    names = []
    for job_id, expected in (
        ("repo-hygiene", "Repository Hygiene"),
        ("fmt", "Check formatting"),
        ("fuzz-check", "Fuzz targets compile"),
        ("release-scripts", "Release Script Tests"),
        ("clippy-and-test", "Build, Test, Clippy"),
        ("integration-tests", "Integration Tests"),
        ("database-tests", "Database Tests (PostgreSQL + MariaDB)"),
        ("run-wfl-programs", "Run WFL Programs"),
    ):
        body = job(ci_text, job_id)
        match = re.search(r"^    name: (.+)$", body, re.M)
        self_name = match.group(1) if match else job_id
        names.append(self_name)
        assert self_name == expected, f"{job_id} name changed: {self_name!r}"
    config = job(config_lint_text, "config-lint")
    config_name_match = re.search(r"^    name: (.+)$", config, re.M)
    names.append(config_name_match.group(1) if config_name_match else "config-lint")
    return names


class VersioningWorkflowTests(unittest.TestCase):
    def setUp(self):
        self.ci = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        self.nightly = (ROOT / ".github/workflows/nightly.yml").read_text(encoding="utf-8")
        self.versioning = (ROOT / ".github/workflows/versioning.yml").read_text(
            encoding="utf-8"
        )
        self.config_lint = (ROOT / ".github/workflows/wfl-config-lint.yml").read_text(
            encoding="utf-8"
        )

    def test_required_check_job_names_are_unchanged(self):
        names = required_check_names(self.ci, self.config_lint)
        self.assertEqual(
            names,
            [
                "Repository Hygiene",
                "Check formatting",
                "Fuzz targets compile",
                "Release Script Tests",
                "Build, Test, Clippy",
                "Integration Tests",
                "Database Tests (PostgreSQL + MariaDB)",
                "Run WFL Programs",
                "config-lint",
            ],
        )
        self.assertIn("os: [blacksmith-2vcpu-ubuntu-2404-arm, blacksmith-4vcpu-windows-2025]", job(self.ci, "repo-hygiene"))
        self.assertIn("os: [blacksmith-4vcpu-ubuntu-2404, blacksmith-4vcpu-windows-2025]", job(self.ci, "integration-tests"))
        self.assertIn("os: [blacksmith-4vcpu-ubuntu-2404, blacksmith-4vcpu-windows-2025]", job(self.ci, "run-wfl-programs"))

    def test_ci_does_not_push_version_bumps_to_protected_branches(self):
        self.assertNotIn("push_version_bump.sh", self.ci)
        self.assertNotRegex(self.ci, r"^  bump-version:", re.M)
        self.assertNotIn("git push origin HEAD", self.ci)
        self.assertNotIn("./scripts/test_push_version_bump.sh", self.ci)
        self.assertIn("./scripts/test_publish_spaces.sh", job(self.ci, "release-scripts"))
        self.assertIn("test_bump_version.py", job(self.ci, "release-scripts"))

    def test_versioning_workflow_does_not_push_to_main_or_dev(self):
        self.assertNotIn("git push origin HEAD", self.versioning)
        self.assertNotRegex(self.versioning, r"git push origin .*(main|dev)", re.S)
        self.assertIn("--from-tags", self.versioning)

    def test_nightly_computes_version_from_tags_once_and_sets_it_before_compile(self):
        check = job(self.nightly, "check-for-changes")
        self.assertIn("--from-tags", check)
        self.assertIn("--print", check)
        self.assertIn("inputs.version_override", check)
        self.assertIn("version:", check)
        windows = job(self.nightly, "build")
        linux = job(self.nightly, "build-linux")
        for body, compile_needle in (
            (windows, "cargo build --release --locked --target ${{ env.TARGET }} --bin wfl"),
            (linux, 'cargo build --release --locked --target "$TARGET" --bin wfl'),
        ):
            self.assertIn("--set-version", body)
            self.assertIn("--update-all", body)
            self.assertIn("--skip-git", body)
            self.assertLess(body.index("--set-version"), body.index(compile_needle))

    def test_nightly_version_checks_and_artifact_names_use_computed_version(self):
        windows = job(self.nightly, "build")
        linux = job(self.nightly, "build-linux")
        self.assertIn("WebFirst Language (WFL) version", windows)
        self.assertIn("needs.check-for-changes.outputs.version", windows)
        self.assertIn("WebFirst Language (WFL) version", linux)
        self.assertIn("needs.check-for-changes.outputs.version", linux)
        self.assertIn(
            "wfl-${{ needs.check-for-changes.outputs.version }}.msi", windows
        )
        self.assertIn(
            "vscode-wfl-${{ needs.check-for-changes.outputs.version }}.vsix", windows
        )
        self.assertIn(
            "wfl-${{ needs.check-for-changes.outputs.version }}-linux-x86_64", linux
        )

    def test_nightly_tags_published_version_and_keeps_nightly_date_tag(self):
        release = job(self.nightly, "release")
        self.assertIn("contents: write", release)
        self.assertIn('TAG="v${VERSION}"', release.replace(" ", ""))
        self.assertIn("git push origin", release)
        self.assertIn("refs/tags/", release)
        self.assertIn("date -u", release)
        self.assertIn("nightly-", release)
        publish_at = release.index("publish_spaces.sh")
        tag_at = release.index('TAG="v${VERSION}"') if 'TAG="v${VERSION}"' in release else release.index("v${VERSION}")
        self.assertLess(publish_at, tag_at)

    def test_workflows_do_not_add_ruleset_bypasses_or_version_pats(self):
        combined = self.ci + self.nightly + self.versioning
        self.assertNotIn("ruleset", combined.lower())
        self.assertNotIn("bypass", combined.lower())
        self.assertNotIn("GH_PAT", combined)
        self.assertNotIn("PERSONAL_ACCESS_TOKEN", combined)
        self.assertNotIn("secrets.VERSION", combined)


if __name__ == "__main__":
    unittest.main()
