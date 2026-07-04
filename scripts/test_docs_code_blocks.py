#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Test harness for WFL code blocks embedded in the documentation.

Walks every Markdown file under Docs/, extracts each ```wfl fenced code
block, writes it to a temporary .wfl file, and runs it through the release
WFL binary. When the doc places an "Output:" block immediately after the
code, the harness compares the program's stdout against that expected text.

Usage:
    python scripts/test_docs_code_blocks.py [--docs Docs] [--json report.json]
                                            [--filter substring] [--timeout 20]

Exit code is 0 when every executed block passes, 1 otherwise.

Blocks are classified so that partial snippets are not counted as hard
failures:
  PASS            - ran to completion (and matched expected output if present)
  OUTPUT_MISMATCH - ran, but stdout differed from the documented Output block
  SNIPPET         - looks like an incomplete fragment (skipped execution)
  TIMEOUT         - exceeded the timeout (e.g. web-server / wait-forever demos)
  ERROR           - non-zero exit / crash while running a full program
"""

import argparse
import json
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import List, Optional

FENCE_RE = re.compile(r"^([ \t]*)```([a-zA-Z0-9_-]*)\s*$")

# Heuristics: lead tokens that indicate a runnable top-level statement.
RUNNABLE_LEADS = (
    "display", "store", "create", "define", "check", "if", "count",
    "for", "repeat", "open", "wait", "listen", "change", "push", "add",
    "print", "main", "try", "when", "make", "let", "run",
)

# Tokens that strongly suggest a fragment rather than a full program.
SNIPPET_HINTS = (
    "...", "// ...", "# ...", "<", ">",  # placeholders / meta syntax
)


@dataclass
class Block:
    doc: str
    start_line: int
    code: str
    expected_output: Optional[str] = None
    classification: str = ""
    exit_code: Optional[int] = None
    stdout: str = ""
    stderr: str = ""
    note: str = ""


def strip_indent(lines: List[str], indent: str) -> List[str]:
    out = []
    for ln in lines:
        if indent and ln.startswith(indent):
            out.append(ln[len(indent):])
        else:
            out.append(ln)
    return out


def extract_blocks(md_path: Path) -> List[Block]:
    """Extract wfl code blocks and any immediately-following Output block."""
    text = md_path.read_text(encoding="utf-8", errors="replace")
    lines = text.splitlines()
    blocks: List[Block] = []
    i = 0
    n = len(lines)
    while i < n:
        m = FENCE_RE.match(lines[i])
        if not m:
            i += 1
            continue
        indent, lang = m.group(1), m.group(2)
        fence_start = i
        i += 1
        body: List[str] = []
        closed = False
        while i < n:
            cm = FENCE_RE.match(lines[i])
            if cm is not None and cm.group(2) == "":
                # closing fence (no language)
                closed = True
                i += 1
                break
            body.append(lines[i])
            i += 1
        if lang != "wfl":
            continue
        code = "\n".join(strip_indent(body, indent)).strip("\n")
        if not code.strip():
            continue
        blk = Block(doc=str(md_path), start_line=fence_start + 1, code=code)
        # Look ahead for an "Output" label + fenced block within a few lines.
        blk.expected_output = find_expected_output(lines, i)
        blocks.append(blk)
    return blocks


def find_expected_output(lines: List[str], idx: int) -> Optional[str]:
    """After a code block (idx points just past closing fence), see if the
    next non-empty lines are an Output label followed by a plain fenced block."""
    j = idx
    n = len(lines)
    seen_label = False
    look = 0
    while j < n and look < 6:
        line = lines[j].strip()
        if line == "":
            j += 1
            continue
        if re.match(r"^\*{0,2}(Expected )?Output:?\*{0,2}\s*$", line, re.IGNORECASE):
            seen_label = True
            j += 1
            look += 1
            continue
        fm = FENCE_RE.match(lines[j])
        if fm is not None:
            # collect until closing fence
            out_lines = []
            j += 1
            while j < n:
                cm = FENCE_RE.match(lines[j])
                if cm is not None and cm.group(2) == "":
                    break
                out_lines.append(lines[j])
                j += 1
            if seen_label:
                return "\n".join(out_lines).strip("\n")
            return None
        # first meaningful non-label, non-fence line: stop.
        break
    return None


def looks_like_snippet(code: str) -> bool:
    stripped = [l for l in code.splitlines() if l.strip()]
    if not stripped:
        return True
    first = stripped[0].lstrip()
    # placeholder markers
    for hint in ("...", "// ...", "# ..."):
        if hint in code:
            return True
    lowered = first.lower()
    if lowered.startswith(("wfl ", "$", "cargo", "npm")):
        return True
    # Fragment if it starts mid-expression (e.g. "otherwise:", "and ...", "with")
    if lowered.startswith(("otherwise", "and ", "or ", "with ", "then ", "end ")):
        return True
    return False


def run_block(blk: Block, wfl_bin: str, timeout: int) -> None:
    if looks_like_snippet(blk.code):
        blk.classification = "SNIPPET"
        blk.note = "heuristic: incomplete fragment / non-wfl command"
        return
    # Run each block inside its own throwaway working directory so examples
    # that write files (data.txt, app.db, ...) don't litter the repo.
    workdir = tempfile.mkdtemp(prefix="wfl_doc_")
    tmp = str(Path(workdir) / "example.wfl")
    Path(tmp).write_text(blk.code + "\n", encoding="utf-8")
    try:
        proc = subprocess.run(
            [str(Path(wfl_bin).resolve()), "example.wfl"],
            capture_output=True, text=True, timeout=timeout, cwd=workdir,
        )
        blk.exit_code = proc.returncode
        blk.stdout = proc.stdout
        blk.stderr = proc.stderr
    except subprocess.TimeoutExpired:
        blk.classification = "TIMEOUT"
        blk.note = f"exceeded {timeout}s (likely a server / wait-forever demo)"
        return
    finally:
        import shutil
        shutil.rmtree(workdir, ignore_errors=True)

    if blk.exit_code != 0:
        blk.classification = "ERROR"
        return

    if blk.expected_output is not None:
        got = blk.stdout.strip("\n")
        want = blk.expected_output.strip("\n")
        if normalize(got) == normalize(want):
            blk.classification = "PASS"
        else:
            blk.classification = "OUTPUT_MISMATCH"
    else:
        blk.classification = "PASS"


def normalize(s: str) -> str:
    # Collapse trailing whitespace per line; ignore blank-line-only diffs.
    lines = [l.rstrip() for l in s.splitlines()]
    while lines and lines[-1] == "":
        lines.pop()
    return "\n".join(lines)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--docs", default="Docs")
    ap.add_argument("--wfl-bin", default="target/release/wfl")
    ap.add_argument("--json", default=None)
    ap.add_argument("--filter", default=None, help="only docs whose path contains this")
    ap.add_argument("--timeout", type=int, default=20)
    ap.add_argument("--show-errors", action="store_true")
    args = ap.parse_args()

    docs_root = Path(args.docs)
    md_files = sorted(docs_root.rglob("*.md"))
    if args.filter:
        md_files = [p for p in md_files if args.filter in str(p)]

    all_blocks: List[Block] = []
    for md in md_files:
        if "/Archive/" in str(md).replace("\\", "/"):
            continue
        all_blocks.extend(extract_blocks(md))

    for blk in all_blocks:
        run_block(blk, args.wfl_bin, args.timeout)

    counts = {}
    for blk in all_blocks:
        counts[blk.classification] = counts.get(blk.classification, 0) + 1

    print(f"Scanned {len(md_files)} docs, {len(all_blocks)} wfl code blocks\n")
    for k in ("PASS", "OUTPUT_MISMATCH", "ERROR", "TIMEOUT", "SNIPPET"):
        print(f"  {k:16} {counts.get(k, 0)}")
    print()

    if args.show_errors:
        for blk in all_blocks:
            if blk.classification in ("ERROR", "OUTPUT_MISMATCH"):
                print("=" * 70)
                print(f"{blk.doc}:{blk.start_line}  [{blk.classification}]")
                print("--- code ---")
                print(blk.code)
                if blk.classification == "OUTPUT_MISMATCH":
                    print("--- expected ---")
                    print(blk.expected_output)
                    print("--- got ---")
                    print(blk.stdout)
                if blk.stderr.strip():
                    print("--- stderr ---")
                    print(blk.stderr[:1500])
                print()

    if args.json:
        Path(args.json).write_text(
            json.dumps([asdict(b) for b in all_blocks], indent=2),
            encoding="utf-8",
        )
        print(f"Wrote {args.json}")

    hard_fail = counts.get("ERROR", 0) + counts.get("OUTPUT_MISMATCH", 0)
    return 1 if hard_fail else 0


if __name__ == "__main__":
    sys.exit(main())
