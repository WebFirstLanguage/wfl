# PRD format checker and autonomous-development policy — change evidence

## Scope and risk

Risk class: **R3**, assigned before implementation. The executable part is
`scripts/check_prd_docs.py`, a new blocking step in the required
**Repository Hygiene** CI job. It is a merge control, and the same change
widens what project-run agents may do (features under an Approved PRD,
project execution, `dev → main` promotion, website operations). When the class
is ambiguous, `testing.md` §5 takes the higher one.

No shipped artifact changes: the `wfl` binary, `wfl-lsp`, the VS Code
extension, the installer and the nightly workflow are untouched. The checker
is repository automation, so under the new TEST-01 exception it is tested by
running the real script against fixture trees (`tests/tooling/`), on both CI
operating systems.

## Acceptance criteria → tests

All in `tests/tooling/test_check_prd_docs.py` (38 tests in the first cycle;
45 after the second cycle below).

| Criterion | Tests |
|---|---|
| Valid PRDs, the template with placeholder dates, a tree without PRDs, and the real repository pass | `CleanTreeTests` (4) |
| File name, `id`, first heading and the reserved `PRD-0000` are checked | `NamingTests` (4) |
| Front matter keys, status values and dates are checked | `FrontMatterTests` (5) |
| The ten `##` sections are required, in order, with no others | `SectionTests` (3) |
| `REQ`/`AC` numbering, keywords and coverage are checked | `RequirementTests` (7) |
| Approval fields, the approver name and "Open questions: None." follow the status | `ApprovalTests` (5) |
| `supersedes` names an existing, Superseded PRD | `SupersedesTests` (3) |
| With `--base`: allowed moves pass; forbidden moves, new Implemented PRDs, deleted PRDs and unrecorded changes to approved requirements fail; an unknown ref exits 2 | `BaseComparisonTests` (7) |

## Red → Green

- Base: `dc38db2f01834fd3984cab1f70ac9933d3e7922a` (`origin/dev`).
- Red: `a8f741541ba2ae41d7fc902469761ffcabdc10a2` — test-only commit with a
  no-op stub checker.
  - Command: `python3 -m unittest tests.tooling.test_check_prd_docs`
  - Result: `Ran 38 tests ... FAILED (failures=32)`. 30 failures were
    `AssertionError: 0 != 1` (the stub found no problem), 1 was
    `0 != 2` (the stub accepted an unknown `--base` ref), and 1 was the missing
    `Engineering/prds/PRD-0000-template.md`. These are the intended reasons.
    The 6 passing tests are the clean-input cases, which a no-op stub also
    passes.
- Green: `6386940d454bfab80c97ba98f82983e270020d95` — checker, template,
  README and CI step.
  - `python3 -m unittest tests.tooling.test_check_prd_docs` → `Ran 38 tests ... OK`
  - `python3 -m unittest discover -s tests/tooling` → `Ran 138 tests ... OK`
    (the existing hygiene, Docker, workflow and installer tests still pass)
  - `python3 scripts/check_prd_docs.py` → exit 0, no output
  - `python3 scripts/check_prd_docs.py --base origin/dev` → exit 0, no output
  - A copy of the template set to `Approved` with no approver and a
    requirement without a keyword → exit 1 with
    `Engineering/prds/PRD-0001-broken-copy.md:7: approved_by and approved_on must be filled when status is Approved`
    and `...:30: REQ-01 must contain MUST, SHOULD or MAY`
  - `python3 scripts/check_repo_hygiene.py --mode static` → exit 0

Local environment: Linux, Python 3.13. Windows was not run locally; the
Repository Hygiene job runs the same tests on Windows in CI.

## Second cycle: Devin review findings on #815

Devin review on head `4a4feaf` raised three findings. Each was reproduced
against that head before any change:

| Finding | Reproduction on `4a4feaf` | Disposition |
|---|---|---|
| An unbolded `- REQ-02 — ...` next to valid entries escapes every check | exit 0, no output | Confirmed; fixed |
| Two files sharing one PRD number pass | exit 0, no output | Confirmed; fixed |
| An empty `created:` passes | exit 1: `PRD-0001-example.md:6: 'created' must be a date YYYY-MM-DD` | Not reproduced; regression test added |

- Red: `dded73c` (test-only). `python3 -m unittest tests.tooling.test_check_prd_docs`
  → `Ran 45 tests ... FAILED (failures=4)`, all `AssertionError: 0 != 1`. The
  failing tests were the unbolded requirement, the short `REQ-2` number, the
  unbolded criterion, and the duplicate number. The empty-`created`
  regression test, the prose-mention guard and the distinct-numbers guard pass.
- Green: the next commit. `python3 -m unittest tests.tooling.test_check_prd_docs`
  → `Ran 45 tests ... OK`. The three reproduction fixtures now exit 1 with
  `malformed requirement entry`, `PRD-0001 is used by more than one file`
  (reported on both files), and the unchanged date message.

## Not applicable, with reason

- Rust unit/integration, WFL end-to-end programs, web tests, fuzz: no Rust or
  WFL code changed. CI still runs them on the PR.
- Coverage: the repository has no coverage gate (testing.md conformance gap).

## Residual risk

- The policy relies on controls that are not set up yet (GOVERNANCE.md §13):
  separate agent identities, code-owner review, the `agent-proposed` label,
  and `wfl-web` CI, branch protection and a deploy procedure. Until then, those
  rules are honour-system or fail closed, as each rule's *Enforced by* line
  says.
- The checker verifies format, not meaning. Whether a diff stays inside its
  PRD is judged by the independent reviewer.

## Rollback

Revert the PR. No external state changes.
