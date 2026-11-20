#!/usr/bin/env python3
"""Unit tests for scripts/check_prd_docs.py (PRD format checker).

The checker enforces the machine-checkable parts of the PRD process
(`Docs/contributing/prd-process.md`): file names, front matter, the fixed
section order, requirement and acceptance-criterion numbering, approval
fields, `supersedes` links, and (with `--base`) the allowed status moves.
These tests run it against small fixture trees, one clean tree and one tree
per rule, so the checker is verified independently of the real repository.

Contract under test:

  python3 scripts/check_prd_docs.py [--root <dir>] [--base <git-ref>]

Exit codes: 0 = clean, 1 = problems found (one `path:line: message` line
each on stdout), 2 = usage error (for example an unknown `--base` ref).
The checker is dependency-free (Python standard library only).
"""

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
CHECKER = REPO_ROOT / "scripts" / "check_prd_docs.py"
PRD_DIR = Path("Engineering") / "prds"


def run_checker(root, extra=None):
    cmd = [sys.executable, str(CHECKER), "--root", str(root)]
    if extra:
        cmd.extend(extra)
    return subprocess.run(cmd, capture_output=True, text=True)


def prd(
    number="0001",
    title="Example feature",
    status="Draft",
    approved_by="",
    approved_on="",
    supersedes="",
    created="2026-11-20",
    requirements=None,
    criteria=None,
    open_questions="None.",
    heading=None,
    sections=None,
    extra_front_matter="",
    revision_lines=None,
):
    """Build one PRD document. Every argument defaults to a valid value."""
    if requirements is None:
        requirements = [
            "- **REQ-01** — WFL MUST print the greeting.",
            "- **REQ-02** — WFL SHOULD exit with status 0.",
        ]
    if criteria is None:
        criteria = [
            "- **AC-01** (REQ-01) — Given a program, when it runs, then the greeting is printed.",
            "- **AC-02** (REQ-02) — Given a program, when it ends, then the exit status is 0.",
        ]
    if revision_lines is None:
        revision_lines = ["- 2026-11-20 — Created."]
    if heading is None:
        heading = f"# PRD-{number}: {title}"
    bodies = {
        "Problem": "People cannot greet.",
        "Goals": "- Greet people.",
        "Non-goals": "- Translating the greeting.",
        "Requirements": "\n".join(requirements),
        "Acceptance criteria": "\n".join(criteria),
        "Compatibility": "No existing program changes.",
        "Security and privacy": "No new risk: output only.",
        "Test plan": "R2. Red test plus an end-to-end TestPrograms program per AC.",
        "Open questions": open_questions,
        "Revision history": "\n".join(revision_lines),
    }
    if sections is None:
        sections = list(bodies)
    parts = [
        "---",
        f"id: PRD-{number}",
        f"title: {title}",
        f"status: {status}",
        "owner: Brad",
        f"created: {created}",
        f"approved_by: {approved_by}".rstrip(),
        f"approved_on: {approved_on}".rstrip(),
        f"supersedes: {supersedes}".rstrip(),
    ]
    if extra_front_matter:
        parts.append(extra_front_matter)
    parts += ["---", "", heading, ""]
    for name in sections:
        parts += [f"## {name}", "", bodies.get(name, "Text."), ""]
    return "\n".join(parts)


APPROVED = {"status": "Approved", "approved_by": "Brad", "approved_on": "2026-11-21"}


class CheckerTestCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        (self.root / PRD_DIR).mkdir(parents=True)
        (self.root / PRD_DIR / "README.md").write_text("# PRDs\n", encoding="utf-8")

    def tearDown(self):
        self._tmp.cleanup()

    def write(self, name, text):
        path = self.root / PRD_DIR / name
        path.write_text(text, encoding="utf-8")
        return path

    def assert_clean(self, result):
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout.strip(), "")

    def assert_problem(self, result, name, fragment):
        """Exit 1, and a `path:line: message` line naming the file and text."""
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        lines = result.stdout.splitlines()
        expected_path = f"{PRD_DIR.as_posix()}/{name}:"
        matching = [ln for ln in lines if ln.startswith(expected_path) and fragment in ln]
        self.assertTrue(
            matching,
            f"expected a line starting with {expected_path!r} containing {fragment!r};\n"
            f"got:\n{result.stdout}",
        )
        for line in matching:
            location = line[len(expected_path):].split(":", 1)[0]
            self.assertTrue(location.isdigit(), f"line number missing in {line!r}")


class CleanTreeTests(CheckerTestCase):
    def test_valid_draft_and_approved_prds_pass(self):
        self.write("PRD-0001-example-feature.md", prd())
        self.write("PRD-0002-second.md", prd(number="0002", title="Second", **APPROVED))
        self.assert_clean(run_checker(self.root))

    def test_template_with_placeholder_dates_passes(self):
        text = prd(number="0000", title="<short title>", created="YYYY-MM-DD",
                   revision_lines=["- YYYY-MM-DD — Created."])
        self.write("PRD-0000-template.md", text)
        self.assert_clean(run_checker(self.root))

    def test_tree_without_prd_directory_is_clean(self):
        empty = tempfile.TemporaryDirectory()
        self.addCleanup(empty.cleanup)
        self.assert_clean(run_checker(Path(empty.name)))

    def test_real_repository_passes_and_has_template(self):
        self.assertTrue((REPO_ROOT / PRD_DIR / "PRD-0000-template.md").is_file())
        self.assert_clean(run_checker(REPO_ROOT))


class NamingTests(CheckerTestCase):
    def test_bad_file_name(self):
        self.write("prd-1-Example.md", prd())
        self.assert_problem(run_checker(self.root), "prd-1-Example.md",
                            "file name must match PRD-NNNN-short-name.md")

    def test_id_must_match_file_name(self):
        self.write("PRD-0003-example.md", prd(number="0001"))
        self.assert_problem(run_checker(self.root), "PRD-0003-example.md",
                            "does not match the file name")

    def test_first_heading_must_name_the_id(self):
        self.write("PRD-0001-example.md", prd(heading="# Example feature"))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "first heading must be '# PRD-0001: <title>'")

    def test_template_number_is_reserved_for_the_template(self):
        self.write("PRD-0000-greeting.md", prd(number="0000"))
        self.assert_problem(run_checker(self.root), "PRD-0000-greeting.md",
                            "PRD-0000 is reserved for PRD-0000-template.md")


class FrontMatterTests(CheckerTestCase):
    def test_missing_front_matter(self):
        self.write("PRD-0001-example.md", "# PRD-0001: Example\n")
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "missing front matter")

    def test_unknown_key(self):
        self.write("PRD-0001-example.md", prd(extra_front_matter="priority: high"))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "unknown front matter key 'priority'")

    def test_missing_key(self):
        text = prd().replace("owner: Brad\n", "")
        self.write("PRD-0001-example.md", text)
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "front matter key 'owner' is missing")

    def test_invalid_status(self):
        self.write("PRD-0001-example.md", prd(status="Done"))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "status 'Done' is not one of")

    def test_bad_date(self):
        self.write("PRD-0001-example.md", prd(created="20 Nov 2026"))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "'created' must be a date YYYY-MM-DD")


class SectionTests(CheckerTestCase):
    ORDER = ["Problem", "Goals", "Non-goals", "Requirements", "Acceptance criteria",
             "Compatibility", "Security and privacy", "Test plan", "Open questions",
             "Revision history"]

    def test_missing_section(self):
        sections = [s for s in self.ORDER if s != "Compatibility"]
        self.write("PRD-0001-example.md", prd(sections=sections))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "missing section '## Compatibility'")

    def test_sections_out_of_order(self):
        sections = list(self.ORDER)
        sections[0], sections[1] = sections[1], sections[0]
        self.write("PRD-0001-example.md", prd(sections=sections))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "is out of order")

    def test_unknown_section(self):
        sections = self.ORDER[:2] + ["Timeline"] + self.ORDER[2:]
        self.write("PRD-0001-example.md", prd(sections=sections))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "unknown section '## Timeline'")


class RequirementTests(CheckerTestCase):
    def test_requirement_needs_a_keyword(self):
        reqs = ["- **REQ-01** — WFL MUST print the greeting.",
                "- **REQ-02** — WFL exits cleanly."]
        self.write("PRD-0001-example.md", prd(requirements=reqs))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "REQ-02 must contain MUST, SHOULD or MAY")

    def test_requirement_numbers_have_no_gaps(self):
        reqs = ["- **REQ-01** — WFL MUST print the greeting.",
                "- **REQ-03** — WFL SHOULD exit with status 0."]
        acs = ["- **AC-01** (REQ-01) — Given x, when y, then z.",
               "- **AC-02** (REQ-03) — Given x, when y, then z."]
        self.write("PRD-0001-example.md", prd(requirements=reqs, criteria=acs))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "requirement IDs must be numbered REQ-01, REQ-02, ... without gaps")

    def test_at_least_one_requirement(self):
        self.write("PRD-0001-example.md", prd(requirements=["Nothing yet."], criteria=["None."]))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "at least one requirement")

    def test_requirement_without_acceptance_criterion(self):
        acs = ["- **AC-01** (REQ-01) — Given x, when y, then z."]
        self.write("PRD-0001-example.md", prd(criteria=acs))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "REQ-02 is not covered by any acceptance criterion")

    def test_criterion_names_unknown_requirement(self):
        acs = ["- **AC-01** (REQ-01) — Given x, when y, then z.",
               "- **AC-02** (REQ-02, REQ-09) — Given x, when y, then z."]
        self.write("PRD-0001-example.md", prd(criteria=acs))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "AC-02 names REQ-09, which does not exist")

    def test_criterion_must_name_a_requirement(self):
        acs = ["- **AC-01** (REQ-01) — Given x, when y, then z.",
               "- **AC-02** (REQ-02) — Given x, when y, then z.",
               "- **AC-03** — Given x, when y, then z."]
        self.write("PRD-0001-example.md", prd(criteria=acs))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "AC-03 must name at least one requirement")

    def test_criterion_numbers_have_no_gaps(self):
        acs = ["- **AC-01** (REQ-01) — Given x, when y, then z.",
               "- **AC-03** (REQ-02) — Given x, when y, then z."]
        self.write("PRD-0001-example.md", prd(criteria=acs))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "acceptance criterion IDs must be numbered AC-01, AC-02, ... without gaps")


class ApprovalTests(CheckerTestCase):
    def test_approved_needs_approval_fields(self):
        self.write("PRD-0001-example.md", prd(status="Approved"))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "approved_by and approved_on must be filled when status is Approved")

    def test_approver_must_be_a_prd_approver(self):
        self.write("PRD-0001-example.md",
                   prd(status="Approved", approved_by="SomeAgent", approved_on="2026-11-21"))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "approved_by 'SomeAgent' is not a PRD approver")

    def test_approved_needs_no_open_questions(self):
        self.write("PRD-0001-example.md", prd(open_questions="- Which port?", **APPROVED))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "'Open questions' must be exactly 'None.' when status is Approved")

    def test_draft_must_not_carry_approval(self):
        self.write("PRD-0001-example.md",
                   prd(status="In review", approved_by="Brad", approved_on="2026-11-21"))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "approval fields must be empty when status is In review")

    def test_rejected_must_not_carry_approval(self):
        self.write("PRD-0001-example.md",
                   prd(status="Rejected", approved_by="Brad", approved_on="2026-11-21"))
        self.assert_problem(run_checker(self.root), "PRD-0001-example.md",
                            "approval fields must be empty when status is Rejected")


class SupersedesTests(CheckerTestCase):
    def test_supersedes_must_exist(self):
        self.write("PRD-0002-second.md", prd(number="0002", supersedes="PRD-0009"))
        self.assert_problem(run_checker(self.root), "PRD-0002-second.md",
                            "supersedes 'PRD-0009', which does not exist")

    def test_superseded_prd_must_have_status_superseded(self):
        self.write("PRD-0001-example.md", prd(**APPROVED))
        self.write("PRD-0002-second.md", prd(number="0002", supersedes="PRD-0001"))
        self.assert_problem(run_checker(self.root), "PRD-0002-second.md",
                            "PRD-0001 must have status Superseded")

    def test_valid_supersede_passes(self):
        self.write("PRD-0001-example.md", prd(status="Superseded", approved_by="Brad",
                                              approved_on="2026-11-21"))
        self.write("PRD-0002-second.md", prd(number="0002", supersedes="PRD-0001"))
        self.assert_clean(run_checker(self.root))


class BaseComparisonTests(CheckerTestCase):
    """`--base <ref>` compares the working tree against a git revision."""

    def setUp(self):
        super().setUp()
        self.git("init", "-q")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "user.name", "Test")
        self.git("config", "commit.gpgsign", "false")

    def git(self, *args):
        return subprocess.run(["git", "-C", str(self.root), *args],
                              check=True, capture_output=True, text=True)

    def commit_base(self):
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "base")
        return self.git("rev-parse", "HEAD").stdout.strip()

    def test_allowed_move_passes(self):
        self.write("PRD-0001-example.md", prd(status="Draft"))
        base = self.commit_base()
        self.write("PRD-0001-example.md", prd(status="In review"))
        self.assert_clean(run_checker(self.root, ["--base", base]))

    def test_disallowed_move_fails(self):
        self.write("PRD-0001-example.md", prd(status="Draft"))
        base = self.commit_base()
        self.write("PRD-0001-example.md", prd(status="Implemented", approved_by="Brad",
                                              approved_on="2026-11-21"))
        self.assert_problem(run_checker(self.root, ["--base", base]), "PRD-0001-example.md",
                            "status change Draft -> Implemented is not allowed")

    def test_new_prd_cannot_start_implemented(self):
        self.write("PRD-0001-example.md", prd())
        base = self.commit_base()
        self.write("PRD-0002-second.md", prd(number="0002", status="Implemented",
                                             approved_by="Brad", approved_on="2026-11-21"))
        self.assert_problem(run_checker(self.root, ["--base", base]), "PRD-0002-second.md",
                            "a new PRD cannot start with status Implemented")

    def test_deleted_prd_fails(self):
        self.write("PRD-0001-example.md", prd())
        base = self.commit_base()
        (self.root / PRD_DIR / "PRD-0001-example.md").unlink()
        self.assert_problem(run_checker(self.root, ["--base", base]), "PRD-0001-example.md",
                            "PRD files are never deleted or renamed")

    def test_approved_scope_change_needs_revision_line(self):
        self.write("PRD-0001-example.md", prd(**APPROVED))
        base = self.commit_base()
        reqs = ["- **REQ-01** — WFL MUST print the greeting twice.",
                "- **REQ-02** — WFL SHOULD exit with status 0."]
        self.write("PRD-0001-example.md", prd(requirements=reqs, **APPROVED))
        self.assert_problem(run_checker(self.root, ["--base", base]), "PRD-0001-example.md",
                            "without a new 'Revision history' line")

    def test_approved_correction_with_revision_line_passes(self):
        self.write("PRD-0001-example.md", prd(**APPROVED))
        base = self.commit_base()
        reqs = ["- **REQ-01** — WFL MUST print the greeting.",
                "- **REQ-02** — WFL SHOULD exit with status zero."]
        lines = ["- 2026-11-20 — Created.", "- 2026-11-22 — Correction: spelled out zero."]
        self.write("PRD-0001-example.md",
                   prd(requirements=reqs, revision_lines=lines, **APPROVED))
        self.assert_clean(run_checker(self.root, ["--base", base]))

    def test_unknown_base_ref_is_a_usage_error(self):
        self.write("PRD-0001-example.md", prd())
        self.commit_base()
        result = run_checker(self.root, ["--base", "no-such-ref"])
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
