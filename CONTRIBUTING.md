# Contributing to WFL

Thank you for your interest in WebFirst Language (WFL). This document is the
root entry point for contribution policy. Day-to-day workflow detail lives in
the development guide; **project authority and community rules** live in the
governance suite below.

| Document | Read this for |
|---|---|
| [GOVERNANCE.md](GOVERNANCE.md) | Who decides what; binding technical policies |
| [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) | Community standards |
| [AI_POLICY.md](AI_POLICY.md) | AI-assisted work is welcome |
| [SECURITY.md](SECURITY.md) | Private vulnerability reporting |
| [Docs/development/contributing-guide.md](Docs/development/contributing-guide.md) | Fork, TDD, fmt/clippy/test, docs validation |
| [Docs/06-best-practices/collaboration-guide.md](Docs/06-best-practices/collaboration-guide.md) | PR template, reviews, commits |
| [Docs/wfl-foundation.md](Docs/wfl-foundation.md) | Design principles |

By participating, you agree to the Code of Conduct and AI Policy.

---

## Ways to contribute (no special status required)

You do **not** need to be a formal Contributor to help. From a fork you can:

- Fix bugs and improve error messages  
- Add tests (`tests/`, `TestPrograms/`)  
- Improve documentation and examples  
- Propose features via Issues / Discussions  
- Improve LSP, VS Code extension, packaging, or scripts  

### Quick start

1. Fork https://github.com/WebFirstLanguage/wfl  
2. Create a branch: `git checkout -b feature/my-change`  
3. Follow TDD and quality gates in the
   [contributing guide](Docs/development/contributing-guide.md)  
4. Open a pull request with a clear description  

### Non-negotiable project rules (summary)

- **Backward compatibility is sacred** — do not break existing WFL programs  
- **TDD** — failing tests first  
- **Docs ship with the feature** — same PR  
- **Quality gates:** `cargo fmt`, `cargo clippy -D warnings`, `cargo test`  
- **Conventional commits** (`feat:`, `fix:`, `docs:`, …)  
- **AI is welcome** — you remain accountable for the result  

Full policy: [GOVERNANCE.md §3](GOVERNANCE.md#3-binding-technical-policies).

---

## Becoming a Contributor

**Contributor** is a trusted role with elevated project access (for example
repository collaborator permissions), granted by Maintainers after application.
It is optional: most people contribute successfully via fork + PR forever.

### Who should apply

Consider applying if you:

- Have made (or are preparing) meaningful contributions to WFL  
- Agree with the foundation principles and governance policies  
- Want to help with review, triage, or ongoing maintenance  
- Can interact respectfully per the Code of Conduct  

You do **not** need professional credentials, a certain job title, or to avoid
AI tools ([AI_POLICY.md](AI_POLICY.md)).

### Criteria Maintainers consider

Applications are reviewed holistically. Typical signals:

| Signal | Why it matters |
|---|---|
| Quality of past PRs/issues | Judgment, communication, follow-through |
| Respect for compatibility and tests | Protects users and the alpha → stable path |
| Alignment with WFL design principles | Natural language, no-unlearning, clarity |
| CoC compliance | Trust for elevated access |
| Sustained interest | Access is a responsibility, not a trophy |

There is no fixed PR count. One excellent, careful contribution can outweigh
many noisy ones. Maintainers may also invite people without a formal
application.

### How to apply

1. **Open a GitHub Discussion** in the
   [WebFirstLanguage/wfl](https://github.com/WebFirstLanguage/wfl) repository  
   (category: *Ideas* or *General*), **or** email **info@logbie.com** with
   subject: `WFL Contributor Application`.  
2. Use the template below.  
3. Wait for Maintainer review. We aim to respond within **14 days**; complex
   cases may take longer.  
4. If approved, you will receive collaborator access (or equivalent) and a
   short welcome note on expectations.  
5. If not approved yet, Maintainers will say what would help (e.g. more review
   cycles, a specific area of ownership). You may re-apply later.

**Privacy:** GitHub Discussions are **public**. Do **not** put a private email
address, phone number, or other non-public contact details in a Discussion
application. Use your public GitHub identity there. If Maintainers need a
private contact path, apply (or follow up) by email to **info@logbie.com**
instead.

### Application template

```markdown
# WFL Contributor Application

## Identity
- Name (or preferred handle):
- GitHub username:
- Time zone (optional):
- Preferred private contact: (EMAIL APPLICATIONS ONLY — leave blank on public Discussions)

## Motivation
Why do you want Contributor status on WFL?

## Background
Briefly: programming experience, languages, open source, teaching, etc.
(AI-assisted work is fine to mention — see AI_POLICY.md.)

## Contributions to date
Links to PRs, issues, docs, discussions, or external write-ups about WFL.

## Areas of interest
(e.g. parser, stdlib, docs, LSP, security, packaging, community)

## Agreements
I have read and agree to follow:
- [ ] GOVERNANCE.md
- [ ] CODE_OF_CONDUCT.md
- [ ] AI_POLICY.md
- [ ] SECURITY.md (especially private reporting for vulns)
- [ ] Project contribution workflow (TDD, docs-with-features, compatibility)

## Access
What access do you believe you need? (e.g. triage, push to non-main branches)
Anything you explicitly do *not* want responsibility for?

## Anything else
```

### What Contributor status is not

- It is **not** required to submit PRs  
- It is **not** automatic co-ownership of the trademark or company assets  
- It is **not** a promise of merge rights to `main` (Maintainers merge unless
  delegated)  
- It may be **revoked** for CoC violations, security abuse, or prolonged
  unavailability with risky credentials left active  

---

## Security vulnerabilities

**Do not** file public issues for security bugs. Follow [SECURITY.md](SECURITY.md).

---

## Questions

- GitHub Discussions: https://github.com/WebFirstLanguage/wfl/discussions  
- Email: info@logbie.com  
- Draft PRs are welcome when you want early design feedback  

**Thank you for helping build a language that reads like English.**
