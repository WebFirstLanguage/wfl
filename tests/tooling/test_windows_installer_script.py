#!/usr/bin/env python3
"""Placement and retirement guards for the Windows installer entry point.

The Repository Hygiene and Layout Policy moves MSI packaging behind a single
maintained entry point, `scripts/build_windows_installer.ps1`, and retires the
root `build_msi.ps1` plus the `Tools/launch_msi_build.py` wrapper. Full MSI
build/install verification is Windows-only and runs in the packaging jobs;
these tests are the cross-platform structural guards.
"""

import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
SCRIPT = REPO_ROOT / "scripts" / "build_windows_installer.ps1"


class WindowsInstallerScriptTest(unittest.TestCase):
    def test_entry_point_exists(self):
        self.assertTrue(
            SCRIPT.is_file(),
            "scripts/build_windows_installer.ps1 is the canonical MSI entry "
            "point and must exist",
        )

    def test_root_build_msi_retired(self):
        self.assertFalse(
            (REPO_ROOT / "build_msi.ps1").exists(),
            "root build_msi.ps1 is retired; use "
            "scripts/build_windows_installer.ps1",
        )

    def test_tools_launcher_retired(self):
        self.assertFalse(
            (REPO_ROOT / "Tools" / "launch_msi_build.py").exists(),
            "Tools/launch_msi_build.py is retired; its options live in "
            "scripts/build_windows_installer.ps1",
        )

    def test_script_does_not_reference_retired_paths(self):
        content = SCRIPT.read_text(encoding="utf-8")
        self.assertNotIn("Tools/", content)
        self.assertNotIn("launch_msi_build", content)

    def test_script_builds_with_cargo(self):
        content = SCRIPT.read_text(encoding="utf-8")
        self.assertIn("cargo", content)


if __name__ == "__main__":
    unittest.main()
