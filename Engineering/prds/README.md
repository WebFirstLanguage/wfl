# Product Requirements Documents (PRDs)

A PRD describes one feature: the problem, the requirements (`REQ-NN`), and the
acceptance criteria (`AC-NN`) that tests prove. A project-run agent builds a
feature only after Brad approves its PRD. The rules are in
[Docs/contributing/prd-process.md](../../Docs/contributing/prd-process.md).

## Files

- One PRD per feature, one file per PRD:
  `Engineering/prds/PRD-NNNN-short-name.md`, for example
  `Engineering/prds/PRD-0001-ordered-imports.md`.
- `NNNN` is the next free four-digit number. Numbers are never reused, and a
  PRD file is never deleted or renamed; a PRD that is no longer wanted gets the
  status `Rejected` or `Superseded`.
- Start from [PRD-0000-template.md](PRD-0000-template.md). `PRD-0000` is the
  template only.

## Statuses

`Draft` → `In review` → `Approved` → `Implemented`. `In review` may go back to
`Draft`, or to `Rejected`. `Approved` and `Implemented` may go to `Superseded`.
Only Brad sets `Approved` or `Rejected`.

## Check

```bash
python scripts/check_prd_docs.py
```

To also check status moves against the target branch:

```bash
python scripts/check_prd_docs.py --base origin/dev
```

Design documents written before this process (for example
`Engineering/designs/2026-11-18-ordered-dns-resolution-design.md`) stay in
`Engineering/designs/` unchanged.
