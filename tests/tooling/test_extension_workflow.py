"""Guard the extension security and host suites in the existing presubmit gate."""

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def job(text, name):
    match = re.search(rf"^  {re.escape(name)}:\n(.*?)(?=^  [\w-]+:|\Z)", text, re.M | re.S)
    if not match:
        raise AssertionError(f"Required workflow job {name!r} is missing")
    return match.group(1)


class ExtensionWorkflowPolicyTests(unittest.TestCase):
    def setUp(self):
        self.workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        self.gate = job(self.workflow, "clippy-and-test")
        self.steps = re.split(r"^      - ", self.gate, flags=re.M)[1:]

    def required_step(self, command):
        matches = [step for step in self.steps if f"run: {command}\n" in step]
        self.assertEqual(len(matches), 1, f"Missing or duplicated required step: {command}")
        step = matches[0]
        self.assertNotRegex(step, r"(?m)^\s+(?:if|continue-on-error):")
        self.assertIn("working-directory: vscode-extension", step)
        return step

    def test_security_regression_runs_after_locked_install_in_presubmit(self):
        self.assertIn("pull_request:", self.workflow)
        install = self.required_step("npm ci")
        regression = self.required_step("npm run test:security")
        self.assertLess(self.steps.index(install), self.steps.index(regression))
        setup = next((step for step in self.steps if "actions/setup-node@" in step), None)
        self.assertIsNotNone(setup, "The extension gate must set up Node explicitly")
        self.assertLess(self.steps.index(setup), self.steps.index(install))

    def test_full_extension_suite_runs_against_the_built_lsp(self):
        host = self.required_step("xvfb-run -a npm test")
        build = next((step for step in self.steps if "cargo build -p wfl-lsp" in step), None)
        self.assertIsNotNone(build, "Host tests require the real LSP binary")
        self.assertLess(self.steps.index(build), self.steps.index(host))
        self.assertIn("clippy-and-test", job(self.workflow, "bump-version"))


if __name__ == "__main__":
    unittest.main()
