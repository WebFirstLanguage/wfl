# Issue policy: agent dispatch and resolution

This policy ranks issues by the authority given to an AI agent **before a fix
starts and before it merges**. It applies to project-run agents and to anyone
dispatching work to them.
It does not restrict a person from using AI while preparing a contribution:
[AI_POLICY.md](../../AI_POLICY.md) applies equally to all contributors.

**Dispatch** means assigning or starting implementation, including a fix PR.
Agents may investigate, reproduce, and propose a scope before dispatch unless
the issue requires private handling. **Autonomous** I1 work needs no human
approval before dispatch. It may merge and close without human approval when
the review bots and branch rules satisfy every I1 completion gate below.
This is a narrow, standing delegation from the Maintainers under
[GOVERNANCE.md](../../GOVERNANCE.md); it grants no release or general triage
authority. Closing an issue without a merged fix (for example, as duplicate or
not planned) still requires a Maintainer or delegated Contributor decision.

## Ranked issue types

Apply the highest applicable type. These issue types govern **dispatch**;
`R0`–`R3` in [testing.md](../../testing.md#5-change-risk-classes) separately
govern test and review evidence. A low testing risk class does not override a
human decision required below.

| Rank | Issue type | Typical issues | Agent authority before a fix |
|---|---|---|---|
| **I1** | Well-defined bug or routine correction, autonomous | A reproducible internal or user-visible bug whose expected behavior is already established; a prose typo or broken link with an unambiguous correction; a test or fixture correction that preserves intended behavior without weakening a required gate | May investigate, self-dispatch, open a PR, then merge and close the linked issue after the I1 completion gates pass. |
| **I2** | Human review before dispatch | Any new feature or enhancement; a bug that needs a product or design decision; speculative performance work; dependency, CI, packaging, installer, or configuration changes; unclear acceptance criteria or disputed expected behavior | May triage and propose a fix. A Maintainer must confirm the scope and authorize dispatch in the issue or linked discussion before implementation starts. |
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

An agent may merge an I1 PR and close its linked issue when **all** of these
conditions hold. Human sign-off is unnecessary when qualifying bot review
satisfies the required approvals:

1. The issue still qualifies as I1 after implementation. The PR links the
   issue and records the expected behavior, risk class, acceptance criteria,
   tests, and evidence required by `testing.md`.
2. All required CI checks pass on the final PR head and the latest target branch
   or merge-queue result. Pending, skipped, waived, flaky, and known-failing
   required checks are not passes. Relevant test layers outside CI must also
   pass when `testing.md` requires them.
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
