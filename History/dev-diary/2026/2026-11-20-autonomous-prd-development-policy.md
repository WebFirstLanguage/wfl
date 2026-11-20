# 2026-11-20 — Autonomous development within approved PRDs

Brad wants WFL development to run on its own, with him as the vision driver:
he decides which features WFL gets, and agents do the rest, including website
management and day-to-day project execution. Bots must not make up features,
but they may fix bugs from issues, and all code must stay test-driven and
end-to-end tested.

Before this change, every new feature was I2: Brad had to approve the start
of the work, and a human had to merge it. New syntax was I3. Agents had no
general triage authority, production hosts (including the website) were
read-only for agents, and the rules named GitHub features only.

## What changed

- **PRD process** (`Docs/contributing/prd-process.md`, rules `PROC-01`–`PROC-19`).
  A feature is any change a user, a program or another system can observe.
  It needs a PRD in `Engineering/prds/PRD-NNNN-short-name.md` with a fixed
  format. Anyone may draft one, and agents may propose their own ideas
  (label `agent-proposed`, at most 3 waiting at once). Only Brad approves.
- **GOVERNANCE.md §3.11–§3.14** (rules `GOV-01`–`GOV-17`). An Approved PRD is
  the dispatch decision. A different agent may merge the feature PR at any
  risk class after the gates pass. Agents may plan approved work, triage
  issues, close duplicates with a link, and merge the `dev → main` promotion at
  the exact-commit fully-green gate. They may deploy and maintain the website
  (wfl.fyi) with a rollback point, a smoke test and automatic rollback. A
  mapping table lists what a move to GitLab would need.
- **testing.md TEST-01.** Every behavioral change needs Red → Green evidence
  and an end-to-end test through the real shipped artifact, at every risk
  class, for people and agents alike.
- **Issue policy.** New section "Feature pull requests under an approved PRD"
  with six completion gates; the I2 and I3 rows now point to it.
- **Tooling.** `scripts/check_prd_docs.py` (built Red → Green, evidence in
  `Engineering/evidence/2026-11-20-prd-checker.md`) runs in the required
  Repository Hygiene job. New `.github/CODEOWNERS`, bug and feature-idea issue
  templates, a PRD line in the PR template, and an agent rule block in
  `CLAUDE.md`.

## Why the rule is shaped this way

- **The approval is the only human step for a feature.** Everything after it
  (planning, building, review, merge) uses the gates that already govern I1
  bug fixes, so autonomy adds no new kind of trust.
- **Proposing is not deciding.** Agents may suggest features, because they see
  gaps and patterns in issues, but the approval fields and the code-owner
  review belong to Brad.
- **Breaking changes stay with Brad.** Backward compatibility is sacred, and
  reserving a new everyday word breaks programs that use it as a name. Every
  PRD therefore has a "Compatibility" section.
- **Feature PRs do not touch their PRD.** PRD files need Brad's review, so a
  separate small PR marks a PRD `Implemented`.
- **Website rules fail closed.** Until `wfl-web` has CI, branch protection and
  a deploy procedure, agents may not merge or deploy it.
- **GitLab is not a drop-in replacement.** On GitLab Free, required approvals,
  approval rules and code-owner approval are Premium features, so a move needs
  Premium or a replacement CI check before the delegations are safe.

## What this does not do

Policy text does not configure GitHub. Agents still connect to GitHub as
`logbie`, which is Brad's account, so an approval from that account cannot
prove a human approved. GOVERNANCE.md §13 lists the setup steps (agent
identities, code-owner review, the label, and the `wfl-web` work) in the order
they must happen.
