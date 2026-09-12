"""Exercise the diagnostics CLI with synthetic, secret-bearing SDK messages."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


REPORTER = Path(__file__).resolve().parents[2] / "scripts" / "report_claude_failure.py"
SENTINEL = "private-credential-that-must-never-appear"


class ClaudeFailureReportTests(unittest.TestCase):
    def report(self, messages=None, raw=None, missing=False, fallback=False):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "claude-execution-output.json"
            if not missing:
                path.write_bytes(raw if raw is not None else json.dumps(messages).encode())
            env = dict(os.environ, RUNNER_TEMP=directory)
            command = [sys.executable, "-I", str(REPORTER)]
            if not fallback:
                command.append(str(path))
            result = subprocess.run(command, capture_output=True, text=True, env=env, timeout=10)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stderr, "")
            self.assertNotIn(SENTINEL, result.stdout)
            return json.loads(result.stdout)

    def test_classifies_terminal_errors_without_exposing_error_text(self):
        cases = [
            ("Invalid OAuth token " + SENTINEL, "authentication"),
            ("OAuth token refresh failed: " + SENTINEL, "token_refresh"),
            ("You've hit your usage limit " + SENTINEL, "quota"),
            ("Your credit balance is too low " + SENTINEL, "quota"),
            ("Rate limit exceeded " + SENTINEL, "rate_limit"),
            ("The model claude-sonnet-5 does not exist " + SENTINEL, "model_unavailable"),
            ("Unknown slash command: code-review:code-review " + SENTINEL, "unknown_command"),
        ]
        for message, category in cases:
            with self.subTest(category=category):
                report = self.report([{"type": "result", "subtype": "success", "is_error": True, "result": message}])
                self.assertIn(category, report["categories"])
                self.assertEqual(report["status"], "parsed")
                self.assertEqual(report["result_subtype"], "success")

    def test_reports_only_selected_init_metadata_and_structured_error_codes(self):
        report = self.report([
            {"type": "system", "subtype": "init", "model": SENTINEL,
             "plugins": [{"name": "code-review@claude-code-plugins", "path": SENTINEL}],
             "slash_commands": ["code-review:code-review", SENTINEL],
             "plugin_errors": [{"plugin": SENTINEL, "message": SENTINEL}]},
            {"type": "system", "subtype": "api_retry", "error": "authentication_failed", "error_status": 401},
            {"type": "assistant", "error": "oauth_org_not_allowed", "message": {"content": [{"type": "text", "text": SENTINEL}]}},
            {"type": "result", "subtype": "error_during_execution", "is_error": True, "num_turns": 1, "errors": [SENTINEL]},
        ])
        self.assertTrue(report["code_review_loaded"])
        self.assertTrue(report["code_review_command_registered"])
        self.assertEqual(report["plugin_error_count"], 1)
        self.assertEqual(report["http_statuses"], [401])
        self.assertEqual(report["categories"], ["authentication"])
        self.assertEqual(report["turns"], 1)

    def test_ignores_normal_transcript_content_and_arbitrary_metadata(self):
        report = self.report([
            {"type": "user", "message": {"content": "Invalid OAuth token " + SENTINEL}},
            {"type": "assistant", "message": {"content": [{"type": "text", "text": "Rate limit " + SENTINEL}]}},
            {"type": "system", "subtype": "api_retry", "error": SENTINEL, "error_status": SENTINEL},
            {"type": "result", "subtype": SENTINEL, "is_error": True, "num_turns": SENTINEL, "result": SENTINEL},
        ])
        self.assertEqual(report["categories"], [])
        self.assertEqual(report["http_statuses"], [])
        self.assertEqual(report["result_subtype"], "other")
        self.assertIsNone(report["turns"])
        self.assertIsNone(report["code_review_loaded"])

    def test_bounds_file_size_and_handles_missing_malformed_and_wrong_shape(self):
        self.assertEqual(self.report(missing=True)["status"], "missing")
        self.assertEqual(self.report(raw=b"{" + SENTINEL.encode())["status"], "invalid")
        self.assertEqual(self.report(messages={"secret": SENTINEL})["status"], "invalid")
        self.assertEqual(self.report(raw=b" " * (1024 * 1024 + 1))["status"], "too_large")
        self.assertEqual(self.report(raw=b"[" * 2000)["status"], "invalid")

    def test_runner_temp_fallback_and_absent_result_remain_diagnostic(self):
        report = self.report([{"type": "system", "subtype": "init", "plugins": []}], fallback=True)
        self.assertFalse(report["code_review_loaded"])
        self.assertFalse(report["has_result"])
        self.assertIsNone(report["result_is_error"])

    def test_success_text_is_not_treated_as_a_startup_error(self):
        report = self.report([{"type": "result", "subtype": "success", "is_error": False,
                               "result": "Reviewed authentication_failed and rate limit handling " + SENTINEL}])
        self.assertEqual(report["categories"], [])
        self.assertFalse(report["result_is_error"])


if __name__ == "__main__":
    unittest.main()
