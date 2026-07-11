# AI Policy — Inclusion, Not Discrimination

## Summary

**WFL was built with AI assistance. AI-assisted contributions are welcome.**

The WebFirst Language project does **not** discriminate against contributors,
reviewers, or Maintainers for using generative AI, coding agents, or similar
tools. We evaluate **the quality of the contribution**, not the purity of the
tooling that produced it.

---

## 1. Why this policy exists

Many open-source communities have informal or explicit bias against
AI-assisted work. That bias is incompatible with how WFL is developed and with
our foundation principle of accessibility for beginners and experts alike.

This policy:

1. States that AI use is a **first-class, legitimate** way to contribute.  
2. Forbids harassment or rejection **solely** because AI was involved.  
3. Still holds humans accountable for correctness, licensing, and project
   policies ([GOVERNANCE.md](GOVERNANCE.md)).

---

## 2. What is allowed

You **may**:

- Use AI to draft code, tests, docs, commit messages, and review comments  
- Use AI agents (including multi-step coding agents) in your workflow  
- Disclose AI use voluntarily (disclosure is appreciated, never shamed)  
- Build tooling that helps people write or learn WFL with AI  

You are **encouraged** to:

- Understand the change you submit well enough to explain and defend it  
- Run the project’s quality gates yourself (fmt, clippy, tests, TestPrograms)  
- Keep secrets out of prompts and out of the tree  
- Prefer small, reviewable PRs over opaque bulk dumps  

---

## 3. What is required (accountability)

AI is a tool; **you** are the contributor of record.

| Requirement | Detail |
|---|---|
| **Correctness** | Code must work; tests must pass; TDD remains mandatory |
| **License** | You must have the right to submit the work under Apache-2.0 |
| **No plagiarism laundering** | Do not paste copyrighted material you cannot relicense |
| **Project policies** | Backward compatibility, docs-with-features, style, security — all still apply |
| **Honest communication** | Do not invent “passing CI” or “reviewed by X” claims |

If an AI-generated contribution introduces bugs, license problems, or policy
violations, the **human author** (and any reviewing Maintainer who merges it)
bears responsibility the same as for hand-written work.

---

## 4. What is forbidden (discrimination)

The following are **Code of Conduct violations** when aimed at AI use itself:

- Rejecting a PR, issue, or applicant **only** because AI was used  
- Harassment, mockery, or gatekeeping of the form “real programmers don’t use AI”  
- Demanding that contributors “prove” they wrote every character by hand  
- Treating disclosed AI use as evidence of bad faith or incompetence  

### What is *not* discrimination

Maintainers and reviewers **may** still:

- Request changes for quality, design fit, tests, docs, or compatibility  
- Reject work that is incorrect, unsafe, incomprehensible, or out of scope  
- Ask for clarification or a simpler design  
- Require that the human author can explain behavior and trade-offs  
- Limit volume (e.g. flood of low-effort automated PRs) as spam or disruption  

**Quality review is not anti-AI bias.** Applying a *higher* bar *only* because
AI was mentioned, or a *lower* bar that skips review because “the bot did it,”
is unfair. Apply the **same bar** to all contributions.

---

## 5. Disclosure (optional but appreciated)

You may note AI assistance in the PR description, for example:

```markdown
## Notes
- Drafted with assistance from <tool/model>; I reviewed, tested, and edited.
```

Disclosure helps reviewers ask better questions. **Lack of disclosure is not
grounds for rejection** under this policy.

Maintainers will not require watermarking, AI-only labels on commits, or
invasive “proof of human” rituals.

---

## 6. AI in project infrastructure

Maintainers may use AI for triage, draft reviews, documentation drafts,
security research assistance, and automation, subject to:

- No pasting private vulnerability details into untrusted third-party tools
  when that would violate [SECURITY.md](SECURITY.md) handling  
- No committing secrets  
- Human sign-off on merges and releases  

---

## 7. Relationship to other policies

| Document | Relationship |
|---|---|
| [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) | AI discrimination is a conduct violation |
| [GOVERNANCE.md](GOVERNANCE.md) | Same technical policies for all contributors |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution and Contributor application process |
| [SECURITY.md](SECURITY.md) | Security reporting remains private and careful |

---

## 8. Amendments

Changes to this policy follow the amendment process in
[GOVERNANCE.md §10](GOVERNANCE.md#10-amending-this-document).

---

**Effective:** 2026-07-10  
**Contact:** info@logbie.com  
**Spirit:** Tools change. Responsibility and kindness do not.
