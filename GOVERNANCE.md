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
| [Docs/contributing/prd-process.md](Docs/contributing/prd-process.md) | How a feature goes from idea to an Approved PRD, and how agents build it |
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
  branch, or force-push shared branches. Promotion is `dev → main` by PR,
  except that an eligible Dependabot pull request may merge into the branch
  Dependabot targeted, including `main` (§3.10). Project-run agents may open
  and merge the promotion PR under §3.12.
- Maintainers merge. There are four standing exceptions for project-run
  agents: an eligible I1 fix (§3.9), an eligible Dependabot pull request
  (§3.10), a feature pull request that implements an Approved PRD (§3.11),
  and the `dev → main` promotion pull request (§3.12), as defined in
  [the issue policy](Docs/contributing/issue-policy.md). Authors, including
  agent authors, do not merge their own pull requests. This needs no separate
  per-PR Brad approval once the ruleset, reviews, and CI below are satisfied.
  For an agent-authored PR, a different eligible agent or an authorized
  Maintainer must perform the merge.
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
There are two exceptions. The §3.10 Dependabot delegation lets a project-run
agent merge an eligible Dependabot pull request into `main` under the same
exact-commit, fully-green rule. The §3.12 promotion delegation lets
project-run agents open and merge the `dev → main` promotion pull request
under the same rule; because the scheduled nightly workflow builds from
`main`, the next nightly build then publishes that code, as it already does
today. Neither exception grants tag, non-nightly release, release-promotion or
WFL deployment authority, which stays with Azusa's gate above.

Always Brad's decisions regardless of CI: spending money; deleting data,
agents or repositories; anything touching secrets; VM configuration changes
(the routine website-host maintenance in §3.13 is the only exception); and
removing or weakening required checks. Release/deploy workflow changes,
organization settings/membership and deletion of branches, rulesets or
workflows also require Brad's explicit approval through the owning issue.

Production hosts are read-only for agents: authorized config/log inspection
only, without exposing secrets. No edits, restarts, installs or migrations.
The one exception is the project website host, within the limits of §3.13.
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
| Day-to-day PR merge | Maintainer(s); project-run agents for eligible I1 fixes (§3.9), eligible Dependabot updates (§3.10), features under an Approved PRD (§3.11) and the `dev → main` promotion (§3.12) | Current-revision CI, independent review, handled bot feedback, and the LOG-16 ruleset |
| Which features WFL gets (PRD approval) | Brad only | The vision decision; §3.11 and [the PRD process](Docs/contributing/prd-process.md) |
| Language design / breaking change | Maintainer(s) | Must satisfy backward-compatibility rules. New, non-breaking syntax may be built by agents under an Approved PRD (§3.11); a breaking change is always merged by Brad |
| Issue triage, planning, duplicate closure | Project-run agents within §3.12; Brad for "not planned" | §3.12 |
| Website content, deploys, routine host maintenance | Project-run agents within §3.13; Brad for everything else on the host | §3.13 |
| Moving the code host (for example to GitLab) | Brad | §3.14 |
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
pass. New features need an Approved PRD (§3.11); unclear behavior needs a
Maintainer's dispatch decision. This delegation does not extend to releases or
security response; issue triage is delegated separately under §3.12. Human
contributors may use AI under `AI_POLICY.md` regardless of issue type.

### 3.10 Agent merges of Dependabot updates

[The issue policy](Docs/contributing/issue-policy.md#dependabot-pull-requests)
delegates review, approval and merge of **eligible** Dependabot pull requests
to project-run AI agents, without per-PR human approval. An agent may merge
directly or turn on GitHub auto-merge. Because Dependabot, not the agent,
authored the change, the reviewing agent's approval is the independent review;
it must rest on the agent's own reading of the diff, release notes and any
advisory, not on Dependabot's description.

Eligible means, in short: Dependabot made every commit; only Cargo or npm
manifests and lockfiles change; every update is semver-compatible (no major
update, and no `0.x` minor update); no new direct dependency, feature, install
script or licence change; the release is at least 72 hours old unless it fixes
an open Dependabot alert; and every required check, bot review and LOG-16 rule
passes on the exact head commit without a bypass. Updates to cryptography, TLS,
authentication or untrusted-input packages are `R3` and eligible only as patch
updates. GitHub Actions, Docker and other ecosystems stay with Maintainers,
because they change workflows, release builds or images.

An eligible pull request may merge into `main` or `dev`, whichever Dependabot
targeted; changes reaching `main` return to `dev` by the normal sync pull
request. Anything not eligible waits for a Maintainer. This delegation grants
no build, release, tag, deployment or production authority, and the Maintainers
remain accountable for it and may revoke it.

### Rules with IDs (§3.11–§3.14 and §10)

The rules below carry IDs (`GOV-NN`) so pull requests, reviews, checks and the
agent rule block in `CLAUDE.md` can cite them. MUST, MUST NOT, SHOULD and MAY
in these rules have their RFC 2119 meaning. IDs are never reused; a retired
rule's ID is not given to a new rule. Each rule's *Enforced by* line says what
enforces it **today**; "not set up yet" points to the setup checklist in §13.

### 3.11 Autonomous development under approved PRDs

Brad is the **vision driver**: he decides which features WFL gets. Project-run
agents do the engineering within the bounds of a PRD he approved. The process
and the PRD format are in [the PRD process](Docs/contributing/prd-process.md).

**GOV-01** — Only Brad approves, rejects or supersedes a PRD. A project-run
agent MUST NOT set a PRD to `Approved` or `Rejected`, or fill its approval
fields. Agents MAY draft and propose PRDs, including for ideas of their own.
*Why:* agents may suggest features but must never decide them.
*Enforced by:* `.github/CODEOWNERS` with code-owner review — not set up yet
(setup checklist #1–#3); `scripts/check_prd_docs.py` checks the approver name
(Repository Hygiene job, exists).

**GOV-02** — A project-run agent MAY start a feature only when the feature's
PRD is `Approved`; the approval is the dispatch decision, and no further
per-feature approval is needed. An agent MUST NOT start building a feature
that has no Approved PRD.
*Why:* this removes Brad from every step except the vision decision.
*Enforced by:* PR review checklist (`.github/pull_request_template.md`) and
independent review.

**GOV-03** — An Approved PRD authorizes only the behavior it states in its
requirements and acceptance criteria. Work that needs more MUST stop and go
back to Brad as a PRD change.
*Why:* this is the fence that keeps agents from making up features.
*Enforced by:* independent review against the PRD (issue policy, feature
completion gates).

**GOV-04** — A different eligible agent MAY merge a feature PR that implements
an Approved PRD, at any testing risk class, when every completion gate in
[the issue policy](Docs/contributing/issue-policy.md#feature-pull-requests-under-an-approved-prd)
passes; an R3 PR also needs a completed security-focused independent review.
The author agent MUST NOT merge its own PR.
*Why:* full autonomy for approved work, with the same gates as I1 bug fixes.
*Enforced by:* LOG-16 ruleset (exists: independent approval, required checks,
resolved threads); separate agent identities — not set up yet (setup
checklist #1).

**GOV-05** — Even with an Approved PRD, these stay Brad's decisions: merging a
breaking change or a compatibility exception (§3.1); security vulnerabilities
and secrets (SECURITY.md); tags and releases (Azusa's gate in the common
policy); legal and licensing; access and permissions; spending money; changes
to the policy files (§10); and moving the code host (§3.14).
*Why:* these are the decisions where a mistake cannot be undone by a revert.
*Enforced by:* the issue policy's I3 type and independent review;
code-owner review for policy files — not set up yet (setup checklist #3).

**GOV-06** — Autonomous work MUST meet the same test rules as any other work:
Red → Green evidence and end-to-end tests through the real shipped artifact
([testing.md](testing.md), WFL profile, "End-to-end evidence for every
behavioral change").
*Why:* all code stays test-driven and end-to-end tested, whoever writes it.
*Enforced by:* required CI checks and independent review (exist).

### 3.12 Agent project execution

Project-run agents run day-to-day project execution. These delegations grant no
tag, release or deployment authority.

**GOV-07** — A project-run agent MAY split an Approved PRD into issues and one
milestone, label them, assign agents and track progress. Each issue MUST name
the PRD and the `REQ`/`AC` IDs it covers and MUST NOT add scope.
*Why:* planning is engineering work; scope is Brad's.
*Enforced by:* honour system and independent review of the resulting PRs.

**GOV-08** — A project-run agent MAY triage incoming issues: classify them as
I1, I2 or I3, add labels and the testing risk class, ask the reporter for
missing reproduction details, and dispatch I1 bugs. A possible vulnerability or
exposed secret MUST go to the private process in SECURITY.md and MUST NOT be
discussed in the public issue.
*Why:* triage keeps the backlog ready for agents without waiting for Brad.
*Enforced by:* honour system.

**GOV-09** — A project-run agent MAY close an issue as a duplicate only with a
link to the original issue. Closing as "not planned", "won't fix" or "invalid"
MUST stay with Brad or a delegated Contributor.
*Why:* a duplicate loses nothing; "not planned" is a product decision.
*Enforced by:* honour system.

**GOV-10** — A project-run agent MAY open the `dev → main` promotion PR, and a
different eligible agent MAY merge it, only when every required check passed on
the exact head commit (none skipped, missing, pending, flaky or failing), the
LOG-16 rules pass without a bypass, and the PR records the head SHA and the
individual required-check results. Because the nightly build runs from
`main`, a promotion is published by the next nightly; tags and non-nightly
releases stay with Azusa's gate.
*Why:* keeps `main` current without a human step, under the same exact-commit
rule Azusa uses.
*Enforced by:* LOG-16 ruleset on `main` (exists); separate agent identities —
not set up yet (setup checklist #1).

### 3.13 Website operations

The project website is **wfl.fyi**, built from
[`WebFirstLanguage/wfl-web`](https://github.com/WebFirstLanguage/wfl-web) and
served from the website host; `webfirstlanguage.com` redirects to it. These
rules are the only exception to "production hosts are read-only for agents".

**GOV-11** — Website code, theme and content change through PRs in `wfl-web`.
A different eligible agent MAY merge a `wfl-web` PR after its required CI and
an independent review pass; a new website feature needs an Approved PRD
(GOV-02). Agents MUST NOT merge `wfl-web` PRs while `wfl-web` has no required
CI and branch protection.
*Why:* the website is code and follows the same rules as WFL itself.
*Enforced by:* `wfl-web` branch protection and CI — not set up yet (setup
checklist #6); until then this rule fails closed.

**GOV-12** — A project-run agent MAY deploy the exact merged `wfl-web` commit to
the website host. Before each deploy it MUST have a rollback point (a host
snapshot, or the previous release kept on disk). After the deploy it MUST run
the smoke test. If the smoke test fails it MUST roll back automatically and
open an issue. Without a rollback point, the agent MUST NOT deploy.
*Why:* a broken site is the project's public face; recovery must be automatic.
*Enforced by:* the website deploy procedure — not set up yet (setup checklist
#7); until then this rule fails closed.

**GOV-13** — A project-run agent MAY publish news posts and release notes on
the website about work that has shipped (merged and published in a nightly or
a release). A post MUST describe only shipped behavior and MUST label any
future work as planned.
*Why:* the honest-docs rule applies to the website too.
*Enforced by:* honour system and the documentation honesty policy (`CLAUDE.md`).

**GOV-14** — A project-run agent MAY do routine maintenance on the website host:
apply operating-system security updates, and upgrade the WFL runtime to a
**published** WFL release (never an unreleased build). The same rollback point,
smoke test and automatic rollback as GOV-12 apply. Everything else on the host
stays Brad's: firewall, SSH, user accounts, TLS certificates, DNS, Cloudflare,
disk or plan changes, and anything that costs money.
*Why:* keeps the site patched without giving agents general administration.
*Enforced by:* the website maintenance procedure — not set up yet (setup
checklist #7); honour system for the excluded areas.

**GOV-15** — Credentials for website deploys, maintenance and CMS publishing
MUST be injected from approved storage. Agents MUST NOT print, copy, create or
rotate them; rotation is Brad's.
*Why:* the common policy's credential rules apply to website automation.
*Enforced by:* honour system; credential injection — not set up yet (setup
checklist #8).

### 3.14 Code-host portability

**GOV-16** — Policy rules describe the control, not the product: "code host",
"PR/MR", "required checks". GitHub is the code host today. Moving to another
code host, for example a self-hosted GitLab instance used only for WFL, is
Brad's decision. Before a move, every control in the table below MUST be
enforced on the new host, or replaced by a CI check that Brad approved; until
then, the old host stays authoritative.
*Why:* the agent delegations are only safe while the controls behind them work.
*Enforced by:* Maintainer review of the move (no move is planned).

| Control the policy relies on | GitHub (today) | GitLab equivalent | Note for a move |
|---|---|---|---|
| No direct pushes to `dev`/`main` | LOG-16 ruleset | Protected branches | Available in GitLab Free |
| Required checks pass on the exact head | Ruleset required status checks | "Pipelines must succeed" | Free; Windows jobs need a Windows runner |
| One independent approval, dismissed on a new push | Ruleset | Approval rules, "Prevent approval by author", "Remove approvals when commits are added" | **Premium or Ultimate only**; GitLab Free has optional approvals |
| Brad's review on PRDs and policy files | `.github/CODEOWNERS` + code-owner review | `CODEOWNERS` + code-owner approval on protected branches | **Premium or Ultimate only** |
| All review threads resolved | Ruleset | "All threads must be resolved" | Free |
| CI | GitHub Actions | GitLab CI/CD | Workflows must be rewritten |
| Dependency updates | Dependabot (§3.10) | Renovate | §3.10 must be re-approved for Renovate |
| Private vulnerability reports | Private vulnerability reporting | Confidential issues | SECURITY.md must be updated |

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
   project-run agent may merge an eligible I1 fix under §3.9, an eligible
   Dependabot pull request under §3.10, a feature pull request under an
   Approved PRD under §3.11, or the `dev → main` promotion under §3.12, after
   its required CI, review, and evidence gates pass. Other main/release actions
   follow the common policy's conditional CEO gate.

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

**GOV-17** — A project-run agent MAY propose a change to a policy file in a
PR, but MUST NOT approve or merge it. Only Brad approves and merges changes to
`GOVERNANCE.md`, `CODE_OF_CONDUCT.md`, `AI_POLICY.md`, `CONTRIBUTING.md`,
`SECURITY.md`, `testing.md`, `REPOSITORY_HYGIENE.md`, `.repo-hygiene.toml`,
`Docs/contributing/issue-policy.md`, `Docs/contributing/prd-process.md`,
`.github/CODEOWNERS`, `AGENTS.md` and `CLAUDE.md`.
*Why:* agents work inside the rules; they do not set them.
*Enforced by:* `.github/CODEOWNERS` with code-owner review — not set up yet
(setup checklist #1–#3).

---

## 11. Current maintainers

| Name | Affiliation | Contact |
|---|---|---|
| Brad | Logbie LLC | info@logbie.com · GitHub: via WebFirstLanguage org |

To request Contributor status, follow
[CONTRIBUTING.md](CONTRIBUTING.md#becoming-a-contributor).

---

## 12. Open items

None.

---

## 13. Setup checklist

Rules marked "not set up yet" depend on these steps. Until a step is done, the
rule that depends on it is a promise, not a control.

1. **Give each project-run agent its own code-host identity** (a GitHub App or
   a machine user), and use the `logbie` account only for Brad. Today Claude
   Code sessions connect to GitHub as `logbie`, including the session that
   wrote this policy change. While agents share Brad's account, an approval
   from `logbie` cannot prove that Brad approved (GOV-01, GOV-04, GOV-10,
   GOV-17).
2. **Merge `.github/CODEOWNERS`** into `dev` and `main`. It names `@logbie`
   as the required reviewer for `Engineering/prds/` and the policy files.
3. **Turn on "Require review from Code Owners"** in the LOG-16 ruleset for
   `main` and `dev`, **after** step 1. If it is turned on first, a PR that an
   agent opened as `logbie` and that touches a PRD or policy file cannot get the
   required code-owner approval, because an author cannot approve their own PR.
4. **Confirm that both Repository Hygiene checks are required** in LOG-16. The
   PRD format check is a step inside that job, so it blocks a merge only while
   the job is a required check.
5. **Create the label `agent-proposed`** in the repository (PROC-04).
6. **`wfl-web`: add CI and branch protection.** CI checks the site's WFL code
   and runs tests against a running instance of the site; branch protection on
   its default branch requires that CI and one independent approval (GOV-11).
7. **`wfl-web`: write the deploy and maintenance procedure**, as a runbook or a
   deploy workflow: rollback point, smoke-test routes, automatic rollback, and
   an issue on failure (GOV-12, GOV-14). A deploy workflow is a release/deploy
   workflow change, so it needs Brad's explicit approval.
8. **Store the website deploy and CMS credentials in approved storage** and
   inject them into the agent environment; Brad owns rotation (GOV-15).

---

**Effective:** 2026-07-10  
**Last reviewed:** 2026-11-20  
**Copyright:** © Logbie LLC. Licensed documentation under the same terms as the
project repository where applicable.
