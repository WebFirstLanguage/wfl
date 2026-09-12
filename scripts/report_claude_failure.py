#!/usr/bin/env python3
"""Print bounded, allowlisted diagnostics without publishing Claude's transcript.

The action's execution file may contain prompts, tool results, and credentials that
GitHub's secret masking might not recognize. Never emit its text, arbitrary
metadata, paths, or exception messages. Categories are hints from terminal error
fields, not an authentication check. This command always exits successfully so a
missing diagnostic file cannot replace the review action's original failure.

Usage: python scripts/report_claude_failure.py /path/to/claude-execution-output.json
Without an argument, inspect the execution file in RUNNER_TEMP. This is an
operator-invoked diagnostic tool; it does not rerun or change any review gate.
"""

import json
import os
from pathlib import Path
import sys


MAX_BYTES = 1024 * 1024
MAX_ERROR_CHARS = 8192
RESULT_SUBTYPES = {
    "success", "error_during_execution", "error_max_turns",
    "error_max_budget_usd", "error_max_structured_output_retries",
}
ERROR_CODES = {
    "authentication_failed": "authentication",
    "oauth_org_not_allowed": "authentication",
    "cloud_credential_error": "authentication",
    "account_on_hold": "account_on_hold",
    "billing_error": "quota",
    "rate_limit": "rate_limit",
    "model_not_found": "model_unavailable",
    "overloaded": "overloaded",
    "server_error": "server_error",
    "invalid_request": "invalid_request",
    "max_output_tokens": "output_limit",
}
ERROR_PHRASES = {
    "authentication": (
        "invalid oauth token", "oauth access token is invalid",
        "invalid api key", "failed to authenticate", "authentication_failed",
    ),
    "token_refresh": ("token refresh failed", "failed to refresh token", "failed to refresh oauth"),
    "quota": ("you've hit your usage limit", "your credit balance is too low", "usage limit reached"),
    "rate_limit": ("rate limit exceeded", "rate_limit_error", "too many requests"),
    "unknown_command": ("unknown slash command", "unknown command: code-review"),
}


def classify_error(value, categories):
    """Match known terminal error phrases; never copy the original text out."""
    if not isinstance(value, str):
        return
    value = value[:MAX_ERROR_CHARS].lower()
    for category, phrases in ERROR_PHRASES.items():
        if any(phrase in value for phrase in phrases):
            categories.add(category)
    if "model" in value and any(phrase in value for phrase in (
        "does not exist", "model not found", "model_not_found",
        "not available", "do not have access",
    )):
        categories.add("model_unavailable")


def summarize(messages):
    """Extract fixed labels and typed metadata from SDK envelopes only."""
    report = {
        "status": "parsed", "has_result": False, "result_is_error": None,
        "result_subtype": None, "turns": None, "code_review_loaded": None,
        "code_review_command_registered": None, "plugin_error_count": None,
        "categories": [], "http_statuses": [],
    }
    categories, statuses = set(), set()
    terminal = None
    for message in messages:
        if not isinstance(message, dict):
            continue
        kind, subtype = message.get("type"), message.get("subtype")
        if kind == "system" and subtype == "init":
            plugins = message.get("plugins")
            commands = message.get("slash_commands")
            errors = message.get("plugin_errors")
            if isinstance(plugins, list):
                report["code_review_loaded"] = any(
                    isinstance(plugin, dict) and plugin.get("name") in (
                        "code-review", "code-review@claude-code-plugins",
                    ) for plugin in plugins
                )
            if isinstance(commands, list):
                report["code_review_command_registered"] = "code-review:code-review" in commands
            if isinstance(errors, list):
                report["plugin_error_count"] = len(errors)
        if kind == "assistant" or (kind == "system" and subtype == "api_retry"):
            code = message.get("error")
            if isinstance(code, str) and code in ERROR_CODES:
                categories.add(ERROR_CODES[code])
            status = message.get("error_status")
            if type(status) is int and 100 <= status <= 599:
                statuses.add(status)
        if kind == "result":
            terminal = message
    if terminal is not None:
        report["has_result"] = True
        is_error = terminal.get("is_error")
        report["result_is_error"] = is_error if type(is_error) is bool else None
        subtype = terminal.get("subtype")
        report["result_subtype"] = subtype if isinstance(subtype, str) and subtype in RESULT_SUBTYPES else "other"
        turns = terminal.get("num_turns")
        if type(turns) is int and 0 <= turns <= 1000000:
            report["turns"] = turns
        if is_error is True:
            classify_error(terminal.get("result"), categories)
            errors = terminal.get("errors")
            if isinstance(errors, list):
                for error in errors[:100]:
                    classify_error(error, categories)
    report["categories"] = sorted(categories)
    report["http_statuses"] = sorted(statuses)
    return report


def read_report(path):
    """Limit input allocation and return fixed status labels for unreadable data."""
    try:
        with path.open("rb") as source:
            raw = source.read(MAX_BYTES + 1)
        if len(raw) > MAX_BYTES:
            return {"status": "too_large"}
        messages = json.loads(raw)
        if not isinstance(messages, list):
            return {"status": "invalid"}
        return summarize(messages)
    except FileNotFoundError:
        return {"status": "missing"}
    except OSError:
        return {"status": "unreadable"}
    except (ValueError, UnicodeError, RecursionError):
        return {"status": "invalid"}


if __name__ == "__main__":
    execution_path = sys.argv[1] if len(sys.argv) > 1 and sys.argv[1] else None
    runner_temp = os.environ.get("RUNNER_TEMP")
    if execution_path:
        result = read_report(Path(execution_path))
    elif runner_temp:
        result = read_report(Path(runner_temp) / "claude-execution-output.json")
    else:
        result = {"status": "missing"}
    print(json.dumps(result, sort_keys=True))
