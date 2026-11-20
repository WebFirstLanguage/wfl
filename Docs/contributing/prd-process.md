# PRD process: from idea to an approved feature

This document says how a new WFL feature goes from an idea to an approved
Product Requirements Document (PRD), how project-run AI agents build an
approved PRD on their own, how bugs are reported, and how records are kept.

Brad (the primary Maintainer) is the **vision driver**: he decides which
features WFL gets, by approving or rejecting PRDs. Agents may propose ideas
and do the engineering, but they never decide what WFL becomes. Authority,
merge and website rules are in [GOVERNANCE.md](../../GOVERNANCE.md) §3.11–§3.14.
Issue types and merge gates are in [the issue policy](issue-policy.md). Test
rules are in [testing.md](../../testing.md). How a policy itself is changed is
in GOVERNANCE.md §10.

The words MUST, MUST NOT, SHOULD, SHOULD NOT and MAY are used as defined in
RFC 2119. Each rule has an ID (`PROC-NN`) that pull requests, reviews and
agents can cite. IDs are never reused.

| Term | Meaning |
|---|---|
| **PRD** | One file under `Engineering/prds/` that describes one feature: problem, requirements (`REQ-NN`) and acceptance criteria (`AC-NN`) |
| **Approved PRD** | A PRD whose `status` is `Approved` and whose approval fields Brad filled |
| **Project-run agent** | An AI agent the Maintainers operate for this project |
| **Code host** | Where the repository, issues and pull requests live: GitHub today (GOVERNANCE.md §3.14) |
| **PR** | Pull request on GitHub; merge request (MR) on GitLab |

---

## 1. What needs a PRD

**PROC-01** — A change MUST have an Approved PRD before implementation starts
when it adds or changes behavior that a WFL user, a WFL program, or another
system can observe. This includes new or changed syntax, keywords, statements,
standard-library functions, CLI flags, configuration keys, LSP or editor
features, and new website features. When it is unclear whether a change needs
a PRD, treat it as needing one.
*Why:* this is how Brad stays the vision driver while agents work on their own.
*Enforced by:* PR review checklist (`.github/pull_request_template.md`) and
independent review; not machine-checked.

**PROC-02** — These changes do not need a PRD: a fix that makes WFL match
behavior that is already documented or established (an I1 bug), a refactor
that keeps behavior, documentation, and tests. CI, packaging, installer,
configuration-infrastructure and dependency changes keep the issue policy's
rules (I2 Maintainer dispatch; the Dependabot delegation). A PR without a PRD
MUST say `PRD: not needed — <reason>` in its description.
*Why:* bug fixes and maintenance should not wait for a document.
*Enforced by:* PR review checklist.

## 2. Writing a PRD

**PROC-03** — Each feature MUST have exactly one PRD, in its own file at
`Engineering/prds/PRD-NNNN-short-name.md`, copied from
[`PRD-0000-template.md`](../../Engineering/prds/PRD-0000-template.md). `NNNN`
is the next free number; numbers are never reused; the `##` headings and
front matter keys are fixed.
*Why:* a fixed format lets a script and a reviewer check every PRD the same way.
*Enforced by:* `scripts/check_prd_docs.py` in the required **Repository
Hygiene** CI job.

**PROC-04** — Anyone MAY draft a PRD: Brad, a Contributor, a Participant, or a
project-run agent. An agent MAY draft a PRD for a feature it proposes itself,
for example from a gap it found or a pattern in issues. The PR that adds an
agent-proposed PRD MUST carry the label `agent-proposed`, and no more than
**3** agent-proposed PRDs may wait for Brad's review at the same time.
*Why:* agents may suggest ideas, but must not flood the person who decides.
*Enforced by:* honour system (an agent counts open `agent-proposed` PRs before
it opens another); the label does not exist yet (GOVERNANCE.md setup
checklist #5).

**PROC-05** — The "Compatibility" section MUST say whether any existing WFL
program, configuration file, CLI invocation or documented output can behave
differently, and whether the PRD reserves a new word. If anything existing can
break, the PRD describes a **breaking change**: GOVERNANCE.md §3.1 applies,
and Brad merges the implementation himself.
*Why:* backward compatibility is sacred, and reserving an everyday word (as
`start` or `find`) breaks every program that uses it as a name.
*Enforced by:* `scripts/check_prd_docs.py` (the section exists); independent
review (the content).

**PROC-06** — A PRD whose testing risk class is R3 (testing.md §5) MUST
describe its threats in "Security and privacy" or link a threat model.
*Why:* R3 work touches security, concurrency or untrusted input.
*Enforced by:* PRD review checklist (§7).

**PROC-07** — Before a PRD becomes `Approved`, "Open questions" MUST be exactly
`None.`, every requirement MUST use MUST, SHOULD or MAY, and every requirement
MUST be named by at least one acceptance criterion.
*Why:* an agent can only build without asking when nothing is left undecided.
*Enforced by:* `scripts/check_prd_docs.py`.

## 3. Statuses and approval

| Status | Meaning | Allowed next status |
|---|---|---|
| `Draft` | Being written | `In review`; within one PR also `Approved` or `Rejected` |
| `In review` | Ready for Brad | `Approved`, `Rejected`, or back to `Draft` |
| `Approved` | Brad approved; agents may build it | `Implemented`, `Superseded` |
| `Implemented` | Every requirement is merged into `dev` | `Superseded` |
| `Rejected` | Brad decided against it (final) | none |
| `Superseded` | Replaced by a newer PRD (final) | none |

**PROC-08** — Only Brad approves, rejects or supersedes a PRD. To approve, he
sets `status: Approved`, `approved_by: Brad` and `approved_on: YYYY-MM-DD` in
the PRD and approves the PR that carries it. To reject, he sets
`status: Rejected` and leaves the approval fields empty. A project-run agent
MUST NOT set `Approved` or `Rejected`, and MUST NOT fill `approved_by` or
`approved_on`.
*Why:* the approval is the decision that lets agents build without asking.
*Enforced by:* `.github/CODEOWNERS` with code-owner review — not active yet
(GOVERNANCE.md setup checklist #1–#3); `scripts/check_prd_docs.py` checks
that `approved_by` names a PRD approver.

**PROC-09** — A PRD MUST only move between statuses as the table above allows,
and a PRD file MUST NOT be deleted or renamed.
*Why:* the history of decisions stays readable and numbers stay unique.
*Enforced by:* `scripts/check_prd_docs.py --base` on every PR (Repository
Hygiene job).

**PROC-10** — A change of meaning to an Approved or Implemented PRD (a
requirement added, removed or changed in meaning) MUST be a new PRD whose
`supersedes` names the old one; the old PRD's status becomes `Superseded` in
the same PR. A correction that changes no meaning (a typo, a clearer word) MAY
be made in place, with one new line in "Revision history".
*Why:* agents build exactly what Brad approved, not a later edit.
*Enforced by:* `scripts/check_prd_docs.py --base` (requires the revision line;
checks `supersedes`); code-owner review decides whether meaning changed.

**PROC-11** — A feature PR MUST NOT edit its PRD file. When every requirement
is merged into `dev`, an agent opens a separate PR that only sets
`status: Implemented` and adds one "Revision history" line listing the merged
PRs; Brad merges it.
*Why:* PRD files need Brad's review, so feature PRs that touch them could not
merge on their own.
*Enforced by:* code-owner review — not active yet (setup checklist #3).

## 4. Building an approved PRD

**PROC-12** — A project-run agent MAY start building an Approved PRD without
any further dispatch decision. It MAY split the PRD into issues and one
milestone; each issue MUST name the PRD and the `REQ`/`AC` IDs it covers, and
MUST NOT add scope.
*Why:* the approval is the dispatch decision; planning must not grow the feature.
*Enforced by:* independent review of the issues' PRs.

**PROC-13** — Every feature PR MUST name its PRD and the IDs it builds in the
title or description, for example `Implements PRD-0007: REQ-01, REQ-02
(AC-01–AC-03)`.
*Why:* reviewers check the PR against exactly those requirements.
*Enforced by:* PR review checklist.

**PROC-14** — If building a requirement needs behavior that the PRD does not
state, or a requirement turns out to be wrong or impossible, the agent MUST
stop that part and propose a PRD change (§3, PROC-10) instead of building it.
*Why:* this is the fence that keeps agents from inventing features.
*Enforced by:* independent review (the reviewer compares the diff with the PRD).

**PROC-15** — Every acceptance criterion MUST be proven by a Red → Green test
and by an end-to-end test through the real shipped artifact, as
[testing.md](../../testing.md) requires (WFL profile, "End-to-end evidence for
every behavioral change").
*Why:* all code is test-driven and end-to-end tested, whoever writes it.
*Enforced by:* required CI and independent review (existing gates).

Merging a feature PR follows [the issue
policy](issue-policy.md#feature-pull-requests-under-an-approved-prd).

## 5. Bug reports

**PROC-16** — Bugs MUST be reported in the code host's issue tracker with the
bug template ([`.github/ISSUE_TEMPLATE/bug_report.md`](../../.github/ISSUE_TEMPLATE/bug_report.md)).
Severity is one of: `critical` (data loss, a security problem, or WFL cannot
be used), `major` (a main function is broken and there is no workaround), or
`minor` (everything else). A possible security vulnerability MUST NOT be
reported there; it follows [SECURITY.md](../../SECURITY.md) privately.
*Why:* a complete report lets an agent reproduce the bug without asking.
*Enforced by:* the issue template; agent triage (GOVERNANCE.md §3.12).

**PROC-17** — A bug-fix PR MUST say `Fixes #N` and include a regression test
that fails without the fix, plus the end-to-end test that testing.md requires.
*Why:* the same bug must not come back.
*Enforced by:* required CI and independent review.

**PROC-18** — An agent that finds a bug outside its current task MUST file a
bug report with the template and continue its task; it fixes the bug only if
the bug is inside the task, or later as a separate I1 issue.
*Why:* keeps each PR small and reviewable.
*Enforced by:* independent review.

## 6. Records

**PROC-19** — The project MUST NOT keep a file that every change appends to.
Each thing that happens once gets its own file: a PRD in `Engineering/prds/`,
a Dev Diary entry in `History/dev-diary/<year>/YYYY-MM-DD-topic.md`, and test
or release evidence in `Engineering/evidence/YYYY-MM-DD-topic.md`. A record for
a past day is not rewritten; a correction is a new dated addendum
([REPOSITORY_HYGIENE.md](../../REPOSITORY_HYGIENE.md) §1). `CHANGELOG.md` is
the one exception, because it grows once per release, not once per change.
*Why:* a file that grows without limit becomes too large to read, too large
for an agent to load, and a source of merge conflicts.
*Enforced by:* `scripts/check_prd_docs.py` (PRDs); independent review (the rest).

## 7. Checks

Run the checker before opening a PR that touches `Engineering/prds/`:

```bash
python scripts/check_prd_docs.py --base origin/dev
```

It prints one `path:line: message` line per problem and exits with status 1 if
there is any. It checks file names and IDs, the front matter keys and
statuses, dates, the section order, `REQ`/`AC` numbering and coverage, the
approval fields, `supersedes` links, allowed status moves, deleted PRDs, and
the revision line for changed approved requirements. CI runs it in the
**Repository Hygiene** job on Linux and Windows.

### PRD review checklist

The script cannot judge meaning. Before Brad reviews a PRD, an agent other
than its author posts this checklist on the PR, answering each line with
**pass** or **fail** and one sentence:

1. Each requirement describes one behavior and can be tested.
2. Each acceptance criterion has a result that can be observed, with no words
   like "fast" or "easy" without a number.
3. "Non-goals" names at least one thing a reader might otherwise expect.
4. "Compatibility" says clearly whether anything existing can break, including
   newly reserved words.
5. "Security and privacy" describes the risks, or says why there are none.
6. Nothing in the PRD contradicts a rule in the policy files or the
   [19 guiding principles and the No-Unlearning Invariant](../wfl-foundation.md).

---

Last reviewed: 2026-11-20
