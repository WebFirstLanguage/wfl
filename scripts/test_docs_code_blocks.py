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
                                            [--audit path.md] [--show-errors]

Each block gets a run *classification*:
  PASS            - ran to completion (and matched expected output if present)
  OUTPUT_MISMATCH - ran, but stdout differed from the documented Output block
  SNIPPET         - looks like an incomplete fragment (execution skipped)
  TIMEOUT         - exceeded the timeout (e.g. web-server / wait-forever demos)
  ERROR           - non-zero exit / crash while running a full program

Exit code is 1 only when a block is a *hard* failure (ERROR or OUTPUT_MISMATCH).
TIMEOUT and SNIPPET are expected outcomes (server demos, illustrative
fragments) and do NOT fail the run.

With --audit, ERROR/PASS results are further grouped into finer categories
(DOC_SYNTAX, PLACEHOLDER, FRAGMENT, NEEDS_MODULE, LANG_GAP, ...) and written as
a Markdown report (the same format as TestPrograms/docs_examples/DOC_CODE_AUDIT.md),
so that report is reproducible from this script alone.
"""

import argparse
import json
import re
import subprocess
import sys
import tempfile
from collections import Counter, defaultdict
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import List, Optional, Tuple

FENCE_RE = re.compile(r"^([ \t]*)```([a-zA-Z0-9_-]*)\s*$")

# Tokens that strongly suggest a fragment rather than a full program.
SNIPPET_HINTS = ("...", "// ...", "# ...")

# Lead tokens that mean "this block starts mid-construct" -> not standalone.
FRAGMENT_LEADS = ("otherwise", "and ", "or ", "with ", "then ", "end ")

# Lead tokens for non-WFL shell/tooling lines.
SHELL_LEADS = ("wfl ", "$", "cargo", "npm")


@dataclass
class Block:
    doc: str
    start_line: int
    code: str
    expected_output: Optional[str] = None
    classification: str = ""
    category: str = ""
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


def extract_blocks(md_path: Path) -> Tuple[List[Block], List[int]]:
    """Extract wfl code blocks and any immediately-following Output block.

    Returns (blocks, unclosed_fence_lines) so callers can warn about a fenced
    block that never closes (a common Markdown authoring slip that would
    otherwise silently swallow the rest of the file)."""
    text = md_path.read_text(encoding="utf-8", errors="replace")
    lines = text.splitlines()
    blocks: List[Block] = []
    unclosed: List[int] = []
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
        if not closed:
            unclosed.append(fence_start + 1)
        if lang != "wfl":
            continue
        code = "\n".join(strip_indent(body, indent)).strip("\n")
        if not code.strip():
            continue
        blk = Block(doc=str(md_path), start_line=fence_start + 1, code=code)
        # Look ahead for an "Output" label + fenced block within a few lines.
        blk.expected_output = find_expected_output(lines, i)
        blocks.append(blk)
    return blocks, unclosed


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
            lang = fm.group(2)
            # collect until closing fence
            out_lines = []
            j += 1
            while j < n:
                cm = FENCE_RE.match(lines[j])
                if cm is not None and cm.group(2) == "":
                    break
                out_lines.append(lines[j])
                j += 1
            # Treat this as the expected stdout when it is either explicitly
            # labelled "Output:", or is an unlabelled `text`/bare fenced block
            # sitting directly after the code (the common "rendered output"
            # convention). A block tagged with another language (js, python,
            # bash, ...) is a comparison/alternative, not this program's output.
            if seen_label or lang in ("", "text"):
                return "\n".join(out_lines).strip("\n")
            return None
        # first meaningful non-label, non-fence line: stop.
        break
    return None


# A `<placeholder>` template (e.g. `store <name> as <value>`) is documentation
# pseudo-syntax, not runnable WFL — recognize it so we skip execution instead of
# running it and counting a guaranteed lexing failure as a hard error.
PLACEHOLDER_RE = re.compile(r"<[^>\n]{1,40}>")

# WFL's built-in test framework (describe/test/expect) only runs under `--test`.
TEST_FRAMEWORK_RE = re.compile(r"^\s*(describe|test)\s+\"|^\s*expect\s", re.MULTILINE)


def looks_like_snippet(code: str) -> bool:
    stripped = [l for l in code.splitlines() if l.strip()]
    if not stripped:
        return True
    if any(hint in code for hint in SNIPPET_HINTS):
        return True
    if PLACEHOLDER_RE.search(code):
        return True
    lowered = stripped[0].lstrip().lower()
    if lowered.startswith(SHELL_LEADS):
        return True
    # Fragment if it starts mid-construct (e.g. "otherwise:", "and ...", "with").
    if lowered.startswith(FRAGMENT_LEADS):
        return True
    return False


def uses_test_framework(code: str) -> bool:
    return bool(TEST_FRAMEWORK_RE.search(code))


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
    cmd = [str(Path(wfl_bin).resolve())]
    if uses_test_framework(blk.code):
        # describe/test/expect blocks need test mode to execute.
        cmd.append("--test")
        blk.note = "run with --test (describe/test/expect)"
    cmd.append("example.wfl")
    try:
        proc = subprocess.run(
            cmd,
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
        want = blk.expected_output.strip("\n")
        # Docs often show abbreviated output ("...", "…") — treat those as
        # illustrative rather than an exact contract, so they don't mis-flag.
        if "..." in want or "…" in want:
            blk.classification = "PASS"
            blk.note = "expected output is abbreviated; ran clean"
            return
        got = blk.stdout.strip("\n")
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


# --- Finer categorization (used by --audit) --------------------------------

ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")

CATEGORY_DESC = {
    "PASS": "Runs clean (matches Output block if present)",
    "SNIPPET": "Illustrative fragment / shell command",
    "PLACEHOLDER": "Template with <placeholder> / ... markers",
    "FRAGMENT": "References a variable/action defined elsewhere in the prose",
    "NEEDS_MODULE": "Imports a module file that does not exist standalone",
    "NEEDS_FIXTURE": "Reads/writes a data file that must exist first",
    "NEEDS_NETWORK": "Live network request / bound port",
    "NEEDS_TLS": "Requires a TLS certificate on disk",
    "NEEDS_TESTMODE": "Uses describe/test/expect (run with --test)",
    "NEEDS_PERMISSION": "Blocked by the default security policy",
    "SERVER_DEMO": "Long-running server / wait-forever demo (times out by design)",
    "LANG_GAP": "Reads like valid WFL but the construct is unsupported (see issue #571)",
    "DOC_SYNTAX": "Doc uses syntax the language does not accept, OR an intentional error demo",
    "RUNTIME": "Valid syntax, fails at runtime (usually missing resource)",
    "OUTPUT_DRIFT": "Runs, but stdout differs from the documented Output block",
    "OTHER": "Uncategorized failure",
}
CATEGORY_ORDER = [
    "PASS", "DOC_SYNTAX", "LANG_GAP", "OUTPUT_DRIFT", "FRAGMENT", "PLACEHOLDER",
    "SNIPPET", "NEEDS_MODULE", "NEEDS_FIXTURE", "NEEDS_NETWORK", "NEEDS_TLS",
    "NEEDS_TESTMODE", "NEEDS_PERMISSION", "SERVER_DEMO", "RUNTIME", "OTHER",
]
SECTION_KEYS = [
    "01-introduction", "02-getting-started", "03-language-basics",
    "04-advanced-features", "05-standard-library", "06-best-practices",
    "guides", "reference", "development",
]


def categorize(blk: Block) -> str:
    code, cl = blk.code, blk.classification
    se = ANSI_RE.sub("", blk.stderr or "")
    if cl == "PASS":
        return "PASS"
    if cl == "TIMEOUT":
        return "SERVER_DEMO"
    if cl == "OUTPUT_MISMATCH":
        return "OUTPUT_DRIFT"
    # `<placeholder>` templates are pseudo-syntax; surface them as PLACEHOLDER
    # whether they were skipped as SNIPPET or attempted and failed.
    if PLACEHOLDER_RE.search(code):
        return "PLACEHOLDER"
    if cl == "SNIPPET":
        return "SNIPPET"
    # cl == ERROR below
    if "..." in code:
        return "PLACEHOLDER"
    if "Lexing error" in se and ("`>`" in se or "`<`" in se):
        return "PLACEHOLDER"
    if re.search(r"between\b", code) and "KeywordBetween" in se:
        return "LANG_GAP"
    if re.search(r"repeat\s+\d+\s+times", code):
        return "LANG_GAP"
    if any(s in se for s in ("Cannot resolve module path", "no project.wfl",
                             "Cannot resolve package")):
        return "NEEDS_MODULE"
    if "test mode" in se:
        return "NEEDS_TESTMODE"
    if "TLS certificate" in se or "certificate is configured" in se:
        return "NEEDS_TLS"
    if re.search(r"No such file|does not exist|Failed to open file|Source file does not exist", se):
        return "NEEDS_FIXTURE"
    if re.search(r"Failed to send HTTP|error sending request|Connection refused|"
                 r"Address already in use|start web server", se):
        return "NEEDS_NETWORK"
    if "blocked by security policy" in se:
        return "NEEDS_PERMISSION"
    if (re.search(r"is not defined|Undefined action|Undefined variable|Undefined function", se)
            and "Parse errors" not in se):
        return "FRAGMENT"
    if "Parse errors" in se:
        return "DOC_SYNTAX"
    if "Runtime errors" in se:
        return "RUNTIME"
    return "OTHER"


def first_error_line(blk: Block) -> str:
    s = ANSI_RE.sub("", blk.stderr or "")
    for l in s.splitlines():
        l = l.strip()
        if re.search(r"error\[|Lexing error|expected|Unexpected", l, re.I):
            return re.sub(r"\s+", " ", l)[:100]
    return (s.strip().splitlines() or [""])[0][:100]


def section_of(doc: str) -> str:
    for s in SECTION_KEYS:
        if "/" + s + "/" in doc:
            return s
    return "other"


def write_audit(blocks: List[Block], path: str) -> None:
    for b in blocks:
        b.category = categorize(b)
    cats = Counter(b.category for b in blocks)
    total = len(blocks)
    o: List[str] = []
    o.append("# Documentation Code Audit\n")
    o.append("Generated by `scripts/test_docs_code_blocks.py --audit`, which extracts "
             "every ` ```wfl ` block from `Docs/`, runs it through the release WFL "
             "binary, and (where a doc shows an **Output:** block) compares actual vs. "
             "expected stdout, then groups the results into the categories below.\n")
    pct = round(100 * cats["PASS"] / total) if total else 0
    o.append(f"**Corpus:** {total} `wfl` code blocks. **{cats['PASS']} run clean ({pct}%).**\n")

    o.append("## By section\n")
    o.append("| Section | Runnable | Fixable remaining |\n|---|---:|---:|")
    bysec = defaultdict(Counter)
    for b in blocks:
        bysec[section_of(b.doc)][b.category] += 1
    for s in SECTION_KEYS + ["other"]:
        c = bysec[s]
        fx = c["DOC_SYNTAX"] + c["LANG_GAP"] + c["OUTPUT_DRIFT"]
        if c:
            o.append(f"| {s} | {c['PASS']} | {fx} |")
    o.append("\n> `reference` and `development` include intentional error "
             "demonstrations (e.g. `error-codes.md`, `reserved-keywords.md`). "
             "Remaining counts elsewhere are mostly intentional `<placeholder>` "
             "templates and deliberate \"Wrong:\" examples.\n")

    o.append("## Summary\n")
    o.append("| Category | Count | Meaning |\n|---|---:|---|")
    for k in CATEGORY_ORDER:
        if cats.get(k):
            o.append(f"| `{k}` | {cats[k]} | {CATEGORY_DESC[k]} |")
    o.append("")

    o.append("## Doc-fixable blocks by file (`DOC_SYNTAX`, `LANG_GAP`, `OUTPUT_DRIFT`)\n")
    fix = [b for b in blocks if b.category in ("DOC_SYNTAX", "LANG_GAP", "OUTPUT_DRIFT")]
    bydoc = defaultdict(list)
    for b in fix:
        bydoc[b.doc].append(b)
    for doc in sorted(bydoc):
        o.append(f"### {doc}  ({len(bydoc[doc])})\n")
        o.append("| Line | Category | First error |\n|---:|---|---|")
        for b in sorted(bydoc[doc], key=lambda x: x.start_line):
            o.append(f"| {b.start_line} | {b.category} | {first_error_line(b).replace('|', chr(92) + '|')} |")
        o.append("")
    Path(path).write_text("\n".join(o), encoding="utf-8")
    print(f"Wrote audit {path} ({len(fix)} fixable blocks remain)")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--docs", default="Docs")
    ap.add_argument("--wfl-bin", default="target/release/wfl")
    ap.add_argument("--json", default=None)
    ap.add_argument("--audit", default=None,
                    help="write the categorized Markdown audit report to this path")
    ap.add_argument("--filter", default=None, help="only docs whose path contains this")
    ap.add_argument("--timeout", type=int, default=20)
    ap.add_argument("--show-errors", action="store_true")
    args = ap.parse_args()

    # Preflight: fail fast with a clear message if the binary is missing,
    # instead of a confusing per-block FileNotFoundError traceback.
    if not Path(args.wfl_bin).exists():
        print(f"error: WFL binary not found at '{args.wfl_bin}'.\n"
              f"Build it with `cargo build --release`, or pass --wfl-bin <path>.",
              file=sys.stderr)
        return 2

    docs_root = Path(args.docs)
    if not docs_root.exists():
        print(f"error: docs directory not found at '{args.docs}'.", file=sys.stderr)
        return 2
    md_files = sorted(docs_root.rglob("*.md"))
    if args.filter:
        md_files = [p for p in md_files if args.filter in str(p)]

    all_blocks: List[Block] = []
    for md in md_files:
        if "/Archive/" in str(md).replace("\\", "/"):
            continue
        blocks, unclosed = extract_blocks(md)
        for ln in unclosed:
            print(f"WARNING: unclosed fence in {md}:{ln}", file=sys.stderr)
        all_blocks.extend(blocks)

    for blk in all_blocks:
        run_block(blk, args.wfl_bin, args.timeout)

    counts = Counter(blk.classification for blk in all_blocks)

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

    if args.audit:
        write_audit(all_blocks, args.audit)

    hard_fail = counts.get("ERROR", 0) + counts.get("OUTPUT_MISMATCH", 0)
    return 1 if hard_fail else 0


if __name__ == "__main__":
    sys.exit(main())
