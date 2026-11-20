#!/usr/bin/env python3
"""PRD format checker — enforcement arm of Docs/contributing/prd-process.md.

Checks every Product Requirements Document (PRD) under `Engineering/prds/`:

  1. File name `PRD-NNNN-short-name.md`; `id` matches the file name and the
     first heading (`# PRD-NNNN: <title>`). `PRD-0000` is the template only.
  2. Front matter has exactly the template's keys; `status` is allowed;
     dates are `YYYY-MM-DD` (the template may keep the `YYYY-MM-DD` text).
  3. The level-2 sections are exactly the template's, in its order.
  4. Requirements are `- **REQ-NN** — ...`, numbered from 01 without gaps,
     and each uses MUST, SHOULD or MAY. Acceptance criteria are
     `- **AC-NN** (REQ-NN, ...) — ...`, numbered from 01 without gaps; each
     names at least one existing requirement, and every requirement is named
     by at least one criterion.
  5. Approved / Implemented / Superseded PRDs carry `approved_by` (a PRD
     approver) and `approved_on`; Approved and Implemented PRDs have exactly
     `None.` under "Open questions". Draft / In review / Rejected PRDs have
     empty approval fields.
  6. `supersedes` names an existing PRD whose status is Superseded.

With `--base <git-ref>` it also compares the working tree with that revision:

  7. A PRD file is never deleted or renamed.
  8. A status change is one the process allows; a new PRD does not start as
     Implemented or Superseded.
  9. Changing the requirements or acceptance criteria of an Approved or
     Implemented PRD needs a new "Revision history" line.

Output: one `path:line: message` line per problem on stdout.
Exit codes: 0 clean, 1 problems found, 2 usage error. Dependency-free:
Python 3 standard library only.
"""

import argparse
import datetime
import re
import subprocess
import sys
from pathlib import Path

PRD_DIR = Path("Engineering") / "prds"
TEMPLATE_NAME = "PRD-0000-template.md"
TEMPLATE_ID = "PRD-0000"
FILE_RE = re.compile(r"^PRD-(\d{4})-[a-z0-9]+(?:-[a-z0-9]+)*\.md$")
ID_RE = re.compile(r"^PRD-\d{4}$")
DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
PLACEHOLDER_DATE = "YYYY-MM-DD"

KEYS = ["id", "title", "status", "owner", "created", "approved_by", "approved_on", "supersedes"]
STATUSES = ["Draft", "In review", "Approved", "Implemented", "Rejected", "Superseded"]
APPROVED_STATES = {"Approved", "Implemented", "Superseded"}
NO_OPEN_QUESTIONS_STATES = {"Approved", "Implemented"}
UNAPPROVED_STATES = {"Draft", "In review", "Rejected"}
SECTIONS = [
    "Problem",
    "Goals",
    "Non-goals",
    "Requirements",
    "Acceptance criteria",
    "Compatibility",
    "Security and privacy",
    "Test plan",
    "Open questions",
    "Revision history",
]
# Only these names may approve a PRD (GOVERNANCE.md §3.11, GOV-01). The real
# control is the code-owner review in .github/CODEOWNERS; this list catches
# an approval field filled with any other name.
APPROVERS = {"Brad"}
# Allowed status moves between the base revision and the working tree. One
# pull request may skip "In review" (Draft -> Approved or Rejected), because
# the approver may approve in the same pull request that proposes the PRD.
ALLOWED_MOVES = {
    ("Draft", "In review"),
    ("Draft", "Approved"),
    ("Draft", "Rejected"),
    ("In review", "Draft"),
    ("In review", "Approved"),
    ("In review", "Rejected"),
    ("Approved", "Implemented"),
    ("Approved", "Superseded"),
    ("Implemented", "Superseded"),
}
NEW_FILE_FORBIDDEN = {"Implemented", "Superseded"}

REQ_LINE_RE = re.compile(r"^\s*[-*]\s+\*\*(REQ-\d{2})\*\*")
AC_LINE_RE = re.compile(r"^\s*[-*]\s+\*\*(AC-\d{2})\*\*(.*)$")
AC_REFS_RE = re.compile(r"^\s*\(([^)]*)\)")
REQ_REF_RE = re.compile(r"REQ-\d{2}")
KEYWORD_RE = re.compile(r"\b(MUST|SHOULD|MAY)\b")
FENCE_RE = re.compile(r"^\s*(```|~~~)")


class UsageError(Exception):
    pass


class Prd:
    """A parsed PRD. Line numbers are 1-based."""

    def __init__(self, rel_path, text):
        self.rel_path = rel_path
        self.lines = text.splitlines()
        self.front = {}
        self.front_lines = {}
        self.front_end = None
        self.heading = None
        self.heading_line = None
        self.sections = []  # (name, heading_line, body_lines[(lineno, text)])
        self._parse()

    def _parse(self):
        lines = self.lines
        if not lines or lines[0].strip() != "---":
            return
        for index in range(1, len(lines)):
            if lines[index].strip() == "---":
                self.front_end = index + 1
                break
        if self.front_end is None:
            return
        in_fence = False
        current = None
        for index in range(self.front_end, len(lines)):
            lineno = index + 1
            text = lines[index]
            if FENCE_RE.match(text):
                in_fence = not in_fence
            if not in_fence and text.startswith("# ") and self.heading is None:
                self.heading = text.rstrip()
                self.heading_line = lineno
                continue
            if not in_fence and text.startswith("## "):
                current = (text[3:].strip(), lineno, [])
                self.sections.append(current)
                continue
            if current is not None:
                current[2].append((lineno, text))

    def section(self, name):
        for entry in self.sections:
            if entry[0] == name:
                return entry
        return None

    def section_text(self, name):
        entry = self.section(name)
        if entry is None:
            return ""
        return "\n".join(text.rstrip() for _, text in entry[2]).strip()

    def revision_entries(self):
        entry = self.section("Revision history")
        if entry is None:
            return 0
        return sum(1 for _, text in entry[2] if re.match(r"^\s*[-*]\s+\S", text))

    def status(self):
        return self.front.get("status")


class Checker:
    def __init__(self, root):
        self.root = root
        self.problems = []

    def report(self, rel_path, line, message):
        self.problems.append((rel_path, line or 1, message))

    # --- per-file checks -------------------------------------------------

    def load(self, path):
        rel_path = path.relative_to(self.root).as_posix()
        text = path.read_text(encoding="utf-8", errors="replace")
        return Prd(rel_path, text)

    def check_file(self, path):
        prd = self.load(path)
        match = FILE_RE.match(path.name)
        expected_id = None
        if not match:
            self.report(prd.rel_path, 1, "file name must match PRD-NNNN-short-name.md "
                        "(four digits, then lowercase words joined by '-')")
        else:
            expected_id = f"PRD-{match.group(1)}"
            if expected_id == TEMPLATE_ID and path.name != TEMPLATE_NAME:
                self.report(prd.rel_path, 1,
                            f"{TEMPLATE_ID} is reserved for {TEMPLATE_NAME}; use the next free number")
        is_template = path.name == TEMPLATE_NAME

        if prd.front_end is None:
            self.report(prd.rel_path, 1, "missing front matter: the file must start with a "
                        "'---' block copied from the template")
            return prd
        self.check_front_matter(prd, expected_id, is_template)
        self.check_heading(prd, expected_id)
        self.check_sections(prd)
        self.check_requirements(prd)
        self.check_approval(prd)
        return prd

    def check_front_matter(self, prd, expected_id, is_template):
        seen = set()
        for index in range(1, prd.front_end - 1):
            lineno = index + 1
            raw = prd.lines[index]
            if not raw.strip():
                continue
            if ":" not in raw:
                self.report(prd.rel_path, lineno, "front matter line is not 'key: value'")
                continue
            key, value = raw.split(":", 1)
            key = key.strip()
            value = value.strip()
            if key in seen:
                self.report(prd.rel_path, lineno, f"duplicate front matter key '{key}'")
                continue
            seen.add(key)
            if key not in KEYS:
                self.report(prd.rel_path, lineno, f"unknown front matter key '{key}'")
                continue
            prd.front[key] = value
            prd.front_lines[key] = lineno
        for key in KEYS:
            if key not in seen:
                self.report(prd.rel_path, 1, f"front matter key '{key}' is missing")

        pid = prd.front.get("id")
        if pid is not None and expected_id is not None and pid != expected_id:
            self.report(prd.rel_path, prd.front_lines["id"],
                        f"id '{pid}' does not match the file name (expected '{expected_id}')")

        status = prd.front.get("status")
        if status is not None and status not in STATUSES:
            self.report(prd.rel_path, prd.front_lines["status"],
                        f"status '{status}' is not one of: {', '.join(STATUSES)}")
        if is_template and status is not None and status != "Draft":
            self.report(prd.rel_path, prd.front_lines["status"], "the template must have status Draft")

        for key, required in (("created", True), ("approved_on", False)):
            value = prd.front.get(key)
            if value is None or (not value and not required):
                continue
            if is_template and value == PLACEHOLDER_DATE:
                continue
            if not valid_date(value):
                self.report(prd.rel_path, prd.front_lines[key], f"'{key}' must be a date YYYY-MM-DD")

        supersedes = prd.front.get("supersedes")
        if supersedes and not ID_RE.match(supersedes):
            self.report(prd.rel_path, prd.front_lines["supersedes"],
                        f"supersedes '{supersedes}' must be a PRD id such as PRD-0001")

    def check_heading(self, prd, expected_id):
        pid = expected_id or prd.front.get("id") or "PRD-NNNN"
        prefix = f"# {pid}: "
        if prd.heading is None or not prd.heading.startswith(prefix) or not prd.heading[len(prefix):].strip():
            self.report(prd.rel_path, prd.heading_line or prd.front_end,
                        f"first heading must be '# {pid}: <title>'")
            return
        title = prd.front.get("title")
        if title is not None and prd.heading[len(prefix):].strip() != title:
            self.report(prd.rel_path, prd.heading_line,
                        "first heading title must match the front matter 'title'")

    def check_sections(self, prd):
        seen = {}
        last_index = -1
        for name, lineno, _ in prd.sections:
            if name not in SECTIONS:
                self.report(prd.rel_path, lineno, f"unknown section '## {name}' "
                            "(use '###' for subsections inside a template section)")
                continue
            if name in seen:
                self.report(prd.rel_path, lineno, f"section '## {name}' appears more than once")
                continue
            seen[name] = lineno
            index = SECTIONS.index(name)
            if index < last_index:
                self.report(prd.rel_path, lineno, f"section '## {name}' is out of order "
                            f"(expected order: {', '.join(SECTIONS)})")
            last_index = max(last_index, index)
        for name in SECTIONS:
            if name not in seen:
                self.report(prd.rel_path, prd.heading_line or 1, f"missing section '## {name}'")

    def check_requirements(self, prd):
        req_entry = prd.section("Requirements")
        ac_entry = prd.section("Acceptance criteria")
        requirements = []  # (id, lineno)
        if req_entry is not None:
            for lineno, text in req_entry[2]:
                match = REQ_LINE_RE.match(text)
                if not match:
                    continue
                rid = match.group(1)
                requirements.append((rid, lineno))
                if not KEYWORD_RE.search(text[match.end():]):
                    self.report(prd.rel_path, lineno, f"{rid} must contain MUST, SHOULD or MAY")
            if not requirements:
                self.report(prd.rel_path, req_entry[1], "Requirements must list at least one "
                            "requirement as '- **REQ-01** — ...'")
        self.check_numbering(prd, requirements, "REQ", "requirement")
        req_ids = {rid for rid, _ in requirements}

        criteria = []
        covered = set()
        if ac_entry is not None:
            for lineno, text in ac_entry[2]:
                match = AC_LINE_RE.match(text)
                if not match:
                    continue
                aid = match.group(1)
                criteria.append((aid, lineno))
                refs_match = AC_REFS_RE.match(match.group(2))
                refs = REQ_REF_RE.findall(refs_match.group(1)) if refs_match else []
                if not refs:
                    self.report(prd.rel_path, lineno,
                                f"{aid} must name at least one requirement, as '(REQ-01)'")
                for ref in refs:
                    if ref not in req_ids:
                        self.report(prd.rel_path, lineno, f"{aid} names {ref}, which does not exist")
                    covered.add(ref)
            if not criteria:
                self.report(prd.rel_path, ac_entry[1], "Acceptance criteria must list at least one "
                            "criterion as '- **AC-01** (REQ-01) — ...'")
        self.check_numbering(prd, criteria, "AC", "acceptance criterion")
        for rid, lineno in requirements:
            if rid not in covered:
                self.report(prd.rel_path, lineno, f"{rid} is not covered by any acceptance criterion")

    def check_numbering(self, prd, entries, prefix, noun):
        seen = set()
        expected = 1
        for ident, lineno in entries:
            if ident in seen:
                self.report(prd.rel_path, lineno, f"duplicate {noun} ID {ident}")
                continue
            seen.add(ident)
            want = f"{prefix}-{expected:02d}"
            if ident != want:
                self.report(prd.rel_path, lineno,
                            f"{noun} IDs must be numbered {prefix}-01, {prefix}-02, ... without gaps "
                            f"(found {ident} where {want} was expected)")
                expected = int(ident.split("-")[1]) + 1
            else:
                expected += 1

    def check_approval(self, prd):
        status = prd.status()
        if status not in STATUSES:
            return
        by = prd.front.get("approved_by", "")
        on = prd.front.get("approved_on", "")
        line = prd.front_lines.get("approved_by") or prd.front_lines.get("status")
        if status in APPROVED_STATES:
            if not by or not on:
                self.report(prd.rel_path, line,
                            f"approved_by and approved_on must be filled when status is {status}")
            if by and by not in APPROVERS:
                self.report(prd.rel_path, line, f"approved_by '{by}' is not a PRD approver "
                            f"(allowed: {', '.join(sorted(APPROVERS))})")
        if status in NO_OPEN_QUESTIONS_STATES:
            entry = prd.section("Open questions")
            if entry is not None and prd.section_text("Open questions") != "None.":
                self.report(prd.rel_path, entry[1],
                            f"'Open questions' must be exactly 'None.' when status is {status}")
        if status in UNAPPROVED_STATES and (by or on):
            self.report(prd.rel_path, line, f"approval fields must be empty when status is {status}")

    # --- whole-tree checks -----------------------------------------------

    def check_supersedes(self, prds):
        by_id = {}
        for prd in prds:
            pid = prd.front.get("id")
            if pid:
                by_id.setdefault(pid, prd)
        for prd in prds:
            target = prd.front.get("supersedes")
            if not target or not ID_RE.match(target):
                continue
            line = prd.front_lines.get("supersedes")
            if target == prd.front.get("id"):
                self.report(prd.rel_path, line, "a PRD cannot supersede itself")
            elif target not in by_id:
                self.report(prd.rel_path, line, f"supersedes '{target}', which does not exist")
            elif by_id[target].status() != "Superseded":
                self.report(prd.rel_path, line, f"{target} must have status Superseded because "
                            f"{prd.front.get('id')} supersedes it")

    def check_against_base(self, base, prds):
        git_ok = run_git(self.root, "rev-parse", "--verify", "--quiet", f"{base}^{{commit}}")
        if git_ok is None:
            raise UsageError(f"unknown --base revision '{base}'")
        listing = run_git(self.root, "ls-tree", "-r", "--name-only", base, "--", PRD_DIR.as_posix())
        base_paths = [p for p in (listing or "").splitlines() if FILE_RE.match(Path(p).name)]
        current = {prd.rel_path: prd for prd in prds}
        for rel_path in base_paths:
            if rel_path not in current:
                self.report(rel_path, 1, "PRD files are never deleted or renamed; set status "
                            "Rejected or Superseded instead (numbers are never reused)")
                continue
            text = run_git(self.root, "show", f"{base}:./{rel_path}")
            if text is None:
                continue
            old = Prd(rel_path, text)
            new = current[rel_path]
            if old.front_end is None:
                continue
            old.front = parse_front_values(old)
            old_status, new_status = old.status(), new.status()
            if (old_status in STATUSES and new_status in STATUSES and old_status != new_status
                    and (old_status, new_status) not in ALLOWED_MOVES):
                self.report(rel_path, new.front_lines.get("status"),
                            f"status change {old_status} -> {new_status} is not allowed "
                            "(see Docs/contributing/prd-process.md, statuses)")
            if old_status in NO_OPEN_QUESTIONS_STATES:
                changed = any(old.section_text(name) != new.section_text(name)
                              for name in ("Requirements", "Acceptance criteria"))
                if changed and new.revision_entries() <= old.revision_entries():
                    entry = new.section("Requirements")
                    self.report(rel_path, entry[1] if entry else 1,
                                "requirements or acceptance criteria of an approved PRD changed "
                                "without a new 'Revision history' line (a change of meaning "
                                "needs a new PRD that supersedes this one)")
        base_set = set(base_paths)
        for rel_path, prd in current.items():
            if rel_path not in base_set and prd.status() in NEW_FILE_FORBIDDEN:
                self.report(rel_path, prd.front_lines.get("status"),
                            f"a new PRD cannot start with status {prd.status()}")

    def run(self, base=None):
        directory = self.root / PRD_DIR
        prds = []
        if directory.is_dir():
            for path in sorted(directory.iterdir()):
                if path.is_file() and path.suffix == ".md" and path.name != "README.md":
                    prds.append(self.check_file(path))
        self.check_supersedes(prds)
        if base is not None:
            self.check_against_base(base, prds)
        return self.problems


def parse_front_values(prd):
    values = {}
    for index in range(1, prd.front_end - 1):
        raw = prd.lines[index]
        if ":" in raw:
            key, value = raw.split(":", 1)
            values.setdefault(key.strip(), value.strip())
    return values


def valid_date(value):
    if not DATE_RE.match(value):
        return False
    try:
        datetime.date.fromisoformat(value)
    except ValueError:
        return False
    return True


def run_git(root, *args):
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True)
    if result.returncode != 0:
        return None
    return result.stdout.decode("utf-8", errors="replace")


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Check PRD files under Engineering/prds/ (Docs/contributing/prd-process.md).")
    parser.add_argument("--root", default=".", help="repository root (default: current directory)")
    parser.add_argument("--base", default=None,
                        help="git revision to compare with, for status moves and deletions")
    args = parser.parse_args(argv)
    root = Path(args.root).resolve()
    checker = Checker(root)
    try:
        problems = checker.run(args.base)
    except UsageError as error:
        print(f"PRD-ERROR: {error}", file=sys.stderr)
        return 2
    for rel_path, line, message in sorted(problems, key=lambda p: (p[0], p[1])):
        print(f"{rel_path}:{line}: {message}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
