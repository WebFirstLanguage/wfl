# Issue policy: agent dispatch and resolution

This policy ranks issues by the authority given to an AI agent **before a fix
starts and before it merges**. It applies to project-run agents and to anyone
dispatching work to them.
It does not restrict a person from using AI while preparing a contribution:
[AI_POLICY.md](../../AI_POLICY.md) applies equally to all contributors.

**Dispatch** means assigning or starting implementation, including a fix PR.
Agents may investigate, reproduce, and propose a scope before dispatch unless
the issue requires private handling. **Autonomous** I1 work needs no human
approval before dispatch. A different eligible agent may merge its PR without
human approval when the review bots and branch rules satisfy every I1 completion
gate below. Authors, including agent authors, must not merge their own PRs.
After the merge, the author agent may close the linked issue with evidence.
This is a narrow, standing delegation from the Maintainers under
[GOVERNANCE.md](../../GOVERNANCE.md); it grants no release or general triage
authority. Closing an issue without a merged fix (for example, as duplicate or
not planned) still requires a Maintainer or delegated Contributor decision.
A second, separate delegation lets agents merge eligible pull requests opened
by Dependabot; see [Dependabot pull requests](#dependabot-pull-requests).

## Ranked issue types

Apply the highest applicable type. These issue types govern **dispatch**;
`R0`–`R3` in [testing.md](../../testing.md#5-change-risk-classes) separately
govern test and review evidence. A low testing risk class does not override a
human decision required below.

| Rank | Issue type | Typical issues | Agent authority before a fix |
|---|---|---|---|
| **I1** | Well-defined bug or routine correction, autonomous | A reproducible internal or user-visible bug whose expected behavior is already established; a prose typo or broken link with an unambiguous correction; a test or fixture correction that preserves intended behavior without weakening a required gate | May investigate, self-dispatch and open a PR. A different eligible agent may merge after the I1 completion gates pass; the author may then close the linked issue with evidence. |
| **I2** | Human review before dispatch | Any new feature or enhancement; a bug that needs a product or design decision; speculative performance work; dependency, CI, packaging, installer, or configuration changes (a pull request opened by Dependabot follows [Dependabot pull requests](#dependabot-pull-requests) instead); unclear acceptance criteria or disputed expected behavior | May triage and propose a fix. A Maintainer must confirm the scope and authorize dispatch in the issue or linked discussion before implementation starts. |
| **I3** | Maintainer-led, restricted | Potential vulnerabilities or secrets; changes to security boundaries or protected data; new or breaking language syntax or semantics; compatibility exceptions, destructive migrations, release controls, registry trust, governance, legal/licensing, Code of Conduct, or access and permission decisions | Do not self-dispatch. A Maintainer chooses the handling channel, decision, scope, and whether to assign any implementation to an agent. Security reports follow [SECURITY.md](../../SECURITY.md) privately. |

I1 applies to bugs when the agent can reproduce the failure, explain the
observed and expected behavior using an existing contract, and state a bounded
acceptance test. A new behavior choice, even when proposed as a bug fix, is I2.
An `R2` or `R3` testing risk class does not by itself prevent autonomous
dispatch of a well-defined bug; the agent must still meet every test and review
gate for that risk class before merge. An I3 security or authority decision
always takes precedence. Executable changes default to `R2` under `testing.md`;
an agent claiming `R1` must explain why no public contract, persistent state,
security boundary, process boundary, or critical journey can be affected. The
required reviewer confirms that claim before merge. An `R0` classification
likewise requires proof that shipped behavior is unchanged.

## I1 completion gates

An eligible agent other than the PR author may merge an I1 PR when **all** of
these conditions hold. The I1 delegation does not permit self-merge. Human
sign-off is unnecessary when qualifying bot review satisfies the required
approvals:

1. The issue still qualifies as I1 after implementation. The PR links the
   issue and records the expected behavior, risk class, acceptance criteria,
   tests, and evidence required by `testing.md`.
2. All required CI checks pass on the final PR head and the latest target branch
   or merge-queue result. Pending, skipped, waived, flaky, and known-failing
   required checks are not passes. The PR's target branch must trigger the
   workflows that produce those checks; a branch rule that requires a check
   does not run it. If a required check has no run for this PR, autonomous
   merge is blocked. Relevant test layers outside CI must also pass when
   `testing.md` requires them.
3. At least one review bot independent of the author agent has examined the
   final diff and evidence, along with any other required review bots. The
   agent addresses each actionable finding with a fix or a documented,
   technically supported response, and no blocking finding or required bot
   approval remains outstanding. A bot that merely repeats the author's claims
   does not satisfy an independent-review requirement. For `R3`, the required
   qualified independent review, including security-focused review when
   applicable, must be complete. If no suitable bot review is available, a
   human reviewer must provide the missing review before merge.
4. Branch protection and required review rules permit the merge without a
   bypass. The agent does not grant itself access, dismiss a required review,
   or weaken CI to make the PR eligible. A comment-only bot review does not
   satisfy a required approving review; if a qualifying bot approval is
   unavailable, obtain the required human approval before merge.

After the merge succeeds, the agent may close the linked issue with the merged
PR and verification evidence. A merge keyword that closes the issue on merge
also satisfies this step. If any gate fails or the change expands beyond I1,
pause and seek a Maintainer decision.

## Dependabot pull requests

Dependabot opens pull requests that update dependencies. No agent writes these
changes, so they need no dispatch decision. A project-run agent may review,
approve, and merge an **eligible** Dependabot pull request without per-PR human
approval. The agent may merge it directly, or it may turn on GitHub auto-merge
so that GitHub merges the pull request once the branch rules pass. This is a
narrow, standing delegation from the Maintainers under
[GOVERNANCE.md](../../GOVERNANCE.md). Like the I1 delegation, it grants no
release, tag, deployment, or general triage authority.

Dependabot opens security updates against the default branch, `main`. An
eligible Dependabot pull request may therefore merge into `main` or `dev`,
whichever branch Dependabot targeted. This is the only exception to
"promotion is `dev → main` by PR" and to the CEO gate for merges to `main`.
A dependency update that reaches `main` this way returns to `dev` through the
normal `main → dev` sync pull request.

### Eligibility

A Dependabot pull request is eligible only when **all** of these hold:

1. **Dependabot made every change.** The pull request author is
   `dependabot[bot]` and Dependabot created every commit on its head. If a
   person or another agent pushes a commit to the branch, the pull request is
   no longer eligible; a Maintainer decides it. Agents must not push to a
   Dependabot branch. An agent may ask Dependabot to rebase or recreate the
   pull request with a `@dependabot` command.
2. **Only dependency files change.** The diff touches only dependency
   manifests and lockfiles for Cargo (`Cargo.toml`, `Cargo.lock`, including
   `fuzz/`) or npm (`package.json`, `package-lock.json`, including
   `vscode-extension/`). The manifest change is limited to the version
   requirement of the dependency being updated.
3. **The update is compatible.** Each directly updated package moves to a
   semver-compatible version: the same major version, and for a `0.x` version
   the same minor version, because `0.y` steps are breaking changes under Cargo
   and npm rules. A major update, or a `0.x` minor update, is never eligible,
   even when it fixes a security advisory.
4. **No new trust is added.** The update adds no new direct dependency,
   removes none, changes no Cargo feature or npm script, and adds no npm
   install script (`preinstall`, `install`, or `postinstall`). Any new
   transitive package uses a licence compatible with Apache-2.0 that already
   appears in the lockfile. A licence change on any updated package is not
   eligible.
5. **The release has aged, unless it fixes an advisory.** The new version was
   published at least 72 hours before the merge, so the ecosystem has time to
   report and yank a compromised or broken release. A security update that
   resolves an open Dependabot alert is exempt from this wait. A yanked or
   deprecated target version is never eligible.
6. **Restricted areas stay with Maintainers.** GitHub Actions, Docker, and
   other non-Cargo, non-npm ecosystems are not eligible, because they change
   workflows, release builds, or container images (see the Brad-reserved
   decisions in [GOVERNANCE.md](../../GOVERNANCE.md)). An update that would
   need a code change, a docs change, a test change, or a compatibility
   exception is not eligible.

### Risk class

A dependency update can change executable behavior, so it is never `R0`. It
is `R2` by default under [testing.md](../../testing.md#5-change-risk-classes).
It is `R3` when the updated package (direct or transitive) implements
cryptography, TLS, authentication, sessions, password hashing, random number
generation for security, subprocess handling, a database driver, or parsing
of untrusted input such as HTTP. An `R3` update is eligible only when it is a
patch update (only the last version number changes) and its security-focused
independent review is complete. `testing.md` does not require an artificial
failing test for dependency maintenance; it does require the compatibility,
security, integration, and end-to-end evidence that CI provides.

### Merge gates

The agent may approve and merge, or turn on auto-merge, only when **all** of
these hold:

1. **The agent reviewed the update itself.** The agent read the full diff,
   the release notes or changelog between the old and new versions, and any
   linked security advisory. Its approving review on the pull request states
   the update type, the risk class, why each eligibility rule holds, and any
   notable change it found. An approval that repeats Dependabot's description
   without that check does not count as independent review.
2. **Required CI passes on the final head.** Every required check passes on
   the exact head commit. This includes Linux and Windows integration, fuzz
   compilation, and the extension dependency security check when npm files
   change. Pending, skipped, cancelled, flaky, and failed required checks are
   not passes. The agent must not rerun a genuine failure to get green. When
   auto-merge is on, GitHub waits for the required checks; if one fails, the
   pull request is not merged and the agent records the failure for a
   Maintainer.
3. **Bot reviews are handled.** Every triggered review bot has finished. The
   agent records a technically supported response to each actionable finding.
   Agents cannot push fixes to a Dependabot branch (eligibility rule 1), so a
   finding that needs a change, or any blocking finding, makes the pull
   request ineligible.
4. **Branch rules pass without a bypass.** The LOG-16 ruleset permits the
   merge. The agent does not grant itself access, dismiss a required review,
   use an administrator override, or weaken a check. If Dependabot rebases the
   branch, the ruleset dismisses the earlier approval, and the agent must
   review the new head again before it approves.
5. **The evidence is recorded.** The approving review, or a pull request
   comment, links the passing CI run for the head commit and lists the
   individual required results.

### When an update is not eligible

If any eligibility rule or gate fails, the agent does not approve or merge.
It leaves one comment that names the failed rule, so a Maintainer can decide.
It may still investigate read-only, for example by reading release notes or
reproducing a CI failure locally. A Dependabot alert for a possible
vulnerability that is **not** yet public follows [SECURITY.md](../../SECURITY.md)
privately, as an I3 matter.

## Triage and escalation

1. Check for a possible vulnerability or exposed secret first. Do not copy its
   details into a public issue or an untrusted tool; use the private process in
   `SECURITY.md`. Treat it as I3 even if the proposed patch looks small.
2. Record the issue type, short rationale, expected outcome, and testing risk
   class in the issue or linked PR. For I2, link the Maintainer's dispatch
   decision. For I3, use the channel selected by the Maintainer.
3. If the issue fits more than one type, choose the higher rank. If facts are
   missing, default to I2; if a sensitive or I3 trigger is plausible, use I3.
   An agent may continue read-only investigation but must pause implementation
   until the required decision is recorded.
4. Reclassify upward and stop the fix if investigation reveals broader behavior,
   compatibility, security, or authority impact. Obtain the new dispatch
   decision before resuming. Do not split an issue merely to evade a gate.
5. For every dispatched fix, follow [testing.md](../../testing.md), the
   documentation and compatibility rules in `GOVERNANCE.md`, and the PR process.
   CI passing is necessary for I1 merge and closure, together with the review
   and evidence gates above. Releases remain Maintainer decisions.

**Examples:** A broken prose link is I1 if its target is obvious. A compiler
diagnostic that contradicts an existing documented rule is I1 when it has a
reproduction and a clear expected message. A request for a new diagnostic is
I2. A parser bug with a documented expected result can be I1 even when its
testing risk is `R3`; an independent review and all `R3` evidence still apply.
A public issue that appears to expose a credential is I3 and moves to private
handling.
