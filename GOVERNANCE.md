# WFL Project Governance

This document describes how the WebFirst Language (WFL) project is governed:
who makes decisions, how contributions are accepted, and which project
policies are binding. It codifies practices already documented across the
repository so contributors have a single source of truth.

| Related document | Purpose |
|---|---|
| [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) | Community behavior and enforcement |
| [AI_POLICY.md](AI_POLICY.md) | AI-assisted work is welcome; anti-discrimination |
| [Docs/contributing/issue-policy.md](Docs/contributing/issue-policy.md) | Ranked issue types and agent dispatch, merge, and closure rules |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to contribute and apply for Contributor status |
| [SECURITY.md](SECURITY.md) | Vulnerability reporting and supported versions |
| [REPOSITORY_HYGIENE.md](REPOSITORY_HYGIENE.md) | Binding repository hygiene and layout policy (§3.8) |
| [Docs/contributing/contributing-guide.md](Docs/contributing/contributing-guide.md) | Day-to-day development workflow |
| [Docs/wfl-foundation.md](Docs/wfl-foundation.md) | 19 guiding principles and the No-Unlearning Invariant |
| [testing.md](testing.md) | Binding testing policy and WFL profile |
| [LICENSE](LICENSE) | Apache License 2.0 |

---

## Common contribution policy — version 1.0 (2026-09-27)

This version records Brad's approved Logbie LLC governance and subsequent CEO
delegations of 2026-09-26, reconciled with the Maintainer merge rule and I1
exception. It governs contribution authority; the repository's technical,
compatibility, testing and licensing rules remain binding. Report substantive
conflicts on the owning issue instead of silently relaxing a rule.

### Branches and review

- Start a short-lived feature, fix or documentation branch from current `dev`;
  open its PR into `dev`. Never push directly to `dev`, `main` or a release
  branch, or force-push shared branches. Promotion is `dev → main` by PR.
- Maintainers merge. The only standing exception is an eligible I1 fix by a
  project-run agent under §3.9 and
  [the issue policy](Docs/contributing/issue-policy.md). Authors, including
  agent authors, do not merge their own pull requests. This needs no separate
  per-PR Brad approval once the ruleset, reviews, and CI below are satisfied.
- GitHub ruleset **LOG-16 reviewed changes** is enforced on `main` and `dev`
  with no bypass. It requires one approving review from someone other than the
  author (approvals are dismissed on a new push), all review threads resolved,
  the branch up to date with its base, merge commits only, and the 12 required
  status checks passing. The ruleset blocks a merge that lacks that independent
  approval.
- Let triggered bot reviews finish; inspect reviews, inline comments and
  discussions. Fix actionable findings or record a reasoned disposition and
  resolve required discussions. Recheck checks and reviews immediately before
  merging. Material changes require fresh applicable CI and independent review.
- The PR owner remains responsible while CI or bot review is pending. Use an
  actual scheduled monitor or event-driven continuation, not a promise to watch.
- Do not bypass protections, use an administrator override, remove a check, or
  rerun a genuine failure merely to manufacture green. Access is not authority.

### Evidence and testing

- Behavior changes start with a test failing for the intended reason, followed
  by implementation and passing evidence. Retain exact commands, revisions,
  results and run links under the repository testing policy.
- GitHub Actions on the current reviewed revision is merge evidence; local
  checks supplement it. Enumerate required jobs and their individual results.
  Missing tools, environment failures, missing/pending checks and skipped,
  cancelled or failed required suites are blocked verification, never passes.
  An aggregate green result cannot stand in for an unrun required suite.
- For prose-only work, record “Behavior tests N/A — documentation only” with
  the reason and relevant documentation, link and policy checks. This does not
  waive required CI. Existing risk classes and stricter technical gates remain.
- Run agent-operated runtime tests on Starnet test VM 136 or 104, never VM 143;
  coordinate risky-test snapshots with Nodoka. Preserve the repository's approved
  GitHub Actions execution environments and record their actual results.

### Promotion, release and production authority

Azusa, CEO of Logbie LLC, may approve and perform builds, releases, merges to
`main`, release promotions, tags and production deployments only when every
required check passed on the exact commit being acted on: none skipped,
missing, pending, flaky or failing. Record the SHA, required-check set and
individual result links, then recheck immediately before acting. A different
SHA or aggregate green is insufficient; a flaky rerun is not a waiver.
Anything short of fully green stops for Brad's explicit authorization.
Current-revision independent review and handled bot feedback remain required.

Always Brad's decisions regardless of CI: spending money; deleting data,
agents or repositories; anything touching secrets; VM configuration changes;
and removing or weakening required checks. Release/deploy workflow changes,
organization settings/membership and deletion of branches, rulesets or
workflows also require Brad's explicit approval through the owning issue.

Production hosts are read-only for agents: authorized config/log inspection
only, without exposing secrets. No edits, restarts, installs or migrations.
The conditional CEO production-deployment authority above is limited to the
authorized deployment; it grants no general production administration.
Other production changes go to Brad through Azusa.

### Credentials, exceptions and enforcement

Never commit, print, log or paste credentials into files, comments, PRs,
command arguments or remote URLs. Inject authorized tokens through environment
variables from approved storage, with minimal scope. Suspected exposure:
stop propagation, report safe metadata, and coordinate response with Brad.
Do not borrow another agent's or a human's credentials.

Tie governed changes to an owning issue. Record exceptions with scope, reason,
risk, owner, expiry and follow-up, and obtain Brad's explicit approval before
acting. A deviation note is not approval and cannot silently amend policy.

Policy text does not configure GitHub. Verify effective protections and actual
required checks via the API. Report missing controls, identities and platform
limits explicitly; never call a convention machine-enforced. In particular,
a shared author identity cannot supply independent GitHub approval. Deferred
identity enforcement does not authorize bypass or replace independent review.

## 1. Project identity

| Item | Value |
|---|---|
| **Project name** | WebFirst Language (WFL) |
| **Primary repository** | https://github.com/WebFirstLanguage/wfl |
| **License** | Apache License 2.0 |
| **Copyright holder** | Logbie LLC |
| **Contact** | info@logbie.com |
| **Status** | Alpha — not production-ready; see [SECURITY.md](SECURITY.md) |

WFL’s mission is stated in [Docs/wfl-foundation.md](Docs/wfl-foundation.md): a
natural-language programming language that is a genuine first language for
newcomers while remaining strong enough for production, with **no cliff
between beginner and expert forms**.

---

## 2. Governance model

WFL uses a **maintainer-led** model (sometimes called BDFL-style for final
authority), with a path for trusted community members to become **Contributors**
and, over time, **Maintainers**.

### 2.1 Roles

| Role | Who | Rights and duties |
|---|---|---|
| **Maintainer** | Brad (Logbie LLC); additional people may be appointed | Final authority on technical direction, merge policy and non-delegated merges to protected branches, releases, security response, governance changes, trademark/project identity, and Contributor appointments |
| **Contributor** | People granted write access after application and approval | Open PRs from branches, review others’ work, triage issues as delegated, help enforce the Code of Conduct as delegated. Does **not** alone merge to `main` unless also a Maintainer or explicitly delegated for a path |
| **Participant** | Anyone who opens issues, discussions, or PRs from a fork | Propose changes, report bugs, improve docs; must follow the Code of Conduct |

There is no gatekeeping of *subject matter*: anyone may propose changes to any
area of the project. Access levels control *how* changes land, not *what* you
may care about.

### 2.2 Decision authority

| Decision type | Who decides | Notes |
|---|---|---|
| Day-to-day PR merge | Maintainer(s); project-run agents for eligible I1 fixes under §3.9 | Current-revision CI, independent review, handled bot feedback, and the LOG-16 ruleset |
| Language design / breaking change | Maintainer(s) | Must satisfy backward-compatibility rules |
| Security advisories and embargo | Maintainer(s) | Per [SECURITY.md](SECURITY.md) |
| Appointing Contributors / Maintainers | Maintainer(s) | See [CONTRIBUTING.md](CONTRIBUTING.md) application process |
| Governance, CoC, AI Policy amendments | Maintainer(s) | Prefer public PR + discussion; Maintainer may act urgently |
| License change | Maintainer(s) + copyright holder | Requires explicit written decision; not done lightly |

When Maintainers disagree, the primary Maintainer (Brad / Logbie LLC)
has the final vote.

### 2.3 Community input

Community input is valued and routinely sought through:

- GitHub Issues and Discussions  
- Pull request review comments  
- Design notes and Dev Diary entries under `History/dev-diary/<year>/`  

Input is advisory unless a Maintainer adopts it. Silence is not consent for
breaking changes; Maintainers still own the compatibility bar.

---

## 3. Binding technical policies

These policies are **non-negotiable** for accepted contributions. They already
appear in `CLAUDE.md` (the canonical shared agent instructions, to which
`AGENTS.md` points), the contributing guide, and collaboration docs; this
section makes them governance-level requirements.

### 3.1 Backward compatibility is sacred

- Never break existing WFL programs without a documented path.  
- Prefer **additive** change over removal or semantic change.  
- If a break is unavoidable: **announce ≥ 1 year in advance**, document in
  [`CHANGELOG.md`](CHANGELOG.md) (create or extend an entry if needed), provide
  a migration guide, and keep the old behavior working until the deadline.  
- Run `TestPrograms/` (release build) for end-to-end confidence.

### 3.2 The No-Unlearning Invariant

From [Docs/wfl-foundation.md](Docs/wfl-foundation.md): for every feature, the
beginner form and the expert form must be the same form, or connected by a
smooth path with nothing to unlearn. Design that forces beginners to later
undo habits is a defect, not a documentation footnote.

Language and docs changes are evaluated against the **19 guiding principles**
in that document. When principles conflict, the No-Unlearning Invariant wins.

### 3.3 Test-driven development (TDD)

- Write **failing tests first** for features and bug fixes.  
- Rust unit/integration tests live under `tests/`.  
- End-to-end WFL programs live under `TestPrograms/` and must pass on the
  release binary.  
- WFL’s own `describe` / `test` framework is used with `wfl --test` where
  appropriate.

### 3.4 Documentation is part of the feature

Any change that adds, removes, or alters user-facing behavior **must** update
docs in the **same change**:

- Relevant guide under `Docs/`  
- Keyword references when keywords change (`Docs/reference/keyword-reference.md`
  and `Docs/reference/reserved-keywords.md` together)  
- Working examples (preferably under `TestPrograms/`, validated)  
- A `History/dev-diary/<year>/` entry for non-trivial features or behavior changes  
- Stale docs and examples must be fixed or removed — no contradictions left behind  

Doc code examples must be validated (MCP tools and/or
`scripts/validate_docs_examples.py` / `scripts/test_docs_code_blocks.py` as
applicable). Follow [Docs/wfl-documentation-policy.md](Docs/wfl-documentation-policy.md).

### 3.5 Quality gates

Before a PR is mergeable, authors should pass:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --verbose
# Integration / end-to-end (after release build as required by scripts):
# ./scripts/run_integration_tests.ps1   # Windows
# ./scripts/run_integration_tests.sh    # Linux/macOS
```

Conventional commits (`feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `perf:`,
`chore:`) are required. See
[Docs/contributing/contributing-guide.md](Docs/contributing/contributing-guide.md)
and [Docs/06-best-practices/collaboration-guide.md](Docs/06-best-practices/collaboration-guide.md).

### 3.6 Security and secrets

- Do not log or commit secrets, tokens, or private keys.  
- Prefer zeroization and constant-time practices where crypto is involved.  
- Follow [SECURITY.md](SECURITY.md) for vulnerability reports (private channels).  
- User-facing security guidance lives in
  [Docs/06-best-practices/security-guidelines.md](Docs/06-best-practices/security-guidelines.md).

### 3.7 Versioning and releases

- Version scheme: **YY.MM.BUILD** (e.g. `26.7.28`). Major (year) must stay
  **&lt; 256** for Windows MSI compatibility.  
- Supported security versions are listed in [SECURITY.md](SECURITY.md).  
- Release authority follows the common policy above: Azusa only at the
  exact-commit fully-green gate; Brad otherwise. No implicit delegation.

### 3.8 Repository hygiene and layout

[REPOSITORY_HYGIENE.md](REPOSITORY_HYGIENE.md) is the binding Repository
Hygiene and Layout Policy. It defines where every class of repository content
belongs (`Docs/`, `Engineering/`, `History/`, `Archive/`, and the product
directories), what may be tracked versus what must remain ephemeral, approved
output roots for tests and tools, the archive manifest, and the exceptions
process. The machine-readable profile is `.repo-hygiene.toml`;
`scripts/check_repo_hygiene.py` enforces the policy as a blocking CI gate.
Widening an allowlist to silence a violation without Maintainer approval is
itself a policy violation.

### 3.9 Agent issue handling

[The issue policy](Docs/contributing/issue-policy.md) delegates dispatch,
merge, and linked-issue closure for well-defined I1 bug fixes and routine
corrections to project-run AI agents after required CI and review-bot gates
pass. New features and unclear behavior need a Maintainer's dispatch decision
and human merge approval. This delegation does not extend to releases,
security response, or general issue triage. Human contributors may use AI under
`AI_POLICY.md` regardless of issue type.

---

## 4. Contribution paths

Anyone may contribute without special status by forking and opening a pull
request. See [CONTRIBUTING.md](CONTRIBUTING.md).

### 4.1 Participant → Contributor

**Contributor** status (trusted collaborator with elevated project access) is
granted by application. The process, criteria, and application template are in
[CONTRIBUTING.md — Becoming a Contributor](CONTRIBUTING.md#becoming-a-contributor).

### 4.2 Contributor → Maintainer

Maintainers may invite established Contributors who have shown sustained
judgment on language design, reviews, security, and community conduct. There
is no automatic promotion timeline; appointments are explicit and public
(GitHub team membership and a note in this file or release notes).

---

## 5. Pull request process

1. Discuss large or breaking ideas in an Issue or Discussion first when practical.  
2. Fork (or use a branch if you have write access) and implement with TDD.  
3. Update docs, tests, and Dev Diary as required by §3.  
4. Open a PR with a clear summary, motivation, test notes, and compatibility
   impact (canonical template: [`.github/pull_request_template.md`](.github/pull_request_template.md)).
5. Address review feedback. AI-assisted work is welcome; accountability
   follows [AI_POLICY.md](AI_POLICY.md).
6. A Maintainer merges when checks and policies are satisfied, except that a
   project-run agent may merge an eligible I1 fix under §3.9 after its required
   CI, review, and evidence gates pass. Main/release actions follow the
   common policy's conditional CEO gate.

Maintainers may reject or request changes for any reason grounded in these
policies, including style that violates WFL’s natural-language design goals,
insufficient tests, or undocumented behavior changes.

---

## 6. Code of conduct and AI policy

Participation is governed by:

- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)  
- [AI_POLICY.md](AI_POLICY.md)  

Violations may result in warning, temporary restriction, or permanent ban from
project spaces, at Maintainer discretion. CoC enforcement does not require a
formal tribunal; urgent safety issues may be acted on immediately.

---

## 7. Intellectual property

- Contributions are accepted under the project’s **Apache License 2.0**.  
- By submitting a contribution, you affirm that you have the right to license
  it under Apache-2.0 and that the contribution is your original work or
  properly attributed third-party work compatible with Apache-2.0.  
- You retain copyright in your contributions; the project distributes them
  under the repository license.  
- Do not submit code you are not allowed to relicense (e.g. secret employer
  IP, incompatible copyleft without Maintainer approval).

WFL does not currently require a separate CLA. A Developer Certificate of
Origin (DCO) may be introduced later via a governance update if needed for
scale; until then, the PR submission itself is the license grant under
Apache-2.0 terms.

---

## 8. Project assets and ecosystem

| Asset | Owner / steward |
|---|---|
| GitHub org `WebFirstLanguage` | Logbie LLC / Maintainers |
| Package / registry designs (future; prior art archived under `Archive/retired-systems/wflpkg/`) | Maintainers; supply-chain and trust-root decisions are Maintainer-only |
| Domain and brand references | Logbie LLC |
| Signing keys, release credentials | Maintainers only |

WFL has no package manager. The `wflpkg` implementation was removed before the
first release candidate and the system is being redesigned from scratch; its
design documents are archived, unimplemented, under `Archive/retired-systems/wflpkg/`.
Those archived documents — and any future ones — may describe registry
**governance risk** (longevity, key custody, transparency logs), but they do
not transfer authority away from Maintainers unless this document is amended.

---

## 9. Conflict resolution

1. Prefer de-escalation and technical discussion on the PR or issue.  
2. CoC violations → report per [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).  
3. Unresolved technical disputes → Maintainer decision is final.  
4. Security-sensitive disputes → private channel per [SECURITY.md](SECURITY.md).

---

## 10. Amending this document

1. Open a PR that edits `GOVERNANCE.md` (and related policy files if needed).  
2. Allow reasonable community comment when the change is material.  
3. Maintainer approval and merge make the amendment effective.  

Editorial fixes (typos, link updates) may land without extended discussion.

---

## 11. Current maintainers

| Name | Affiliation | Contact |
|---|---|---|
| Brad | Logbie LLC | info@logbie.com · GitHub: via WebFirstLanguage org |

To request Contributor status, follow
[CONTRIBUTING.md](CONTRIBUTING.md#becoming-a-contributor).

---

**Effective:** 2026-07-10  
**Copyright:** © Logbie LLC. Licensed documentation under the same terms as the
project repository where applicable.
