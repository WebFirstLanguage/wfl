# WFL Project Governance

This document describes how the WebFirst Language (WFL) project is governed:
who makes decisions, how contributions are accepted, and which project
policies are binding. It codifies practices already documented across the
repository so contributors have a single source of truth.

| Related document | Purpose |
|---|---|
| [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) | Community behavior and enforcement |
| [AI_POLICY.md](AI_POLICY.md) | AI-assisted work is welcome; anti-discrimination |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to contribute and apply for Contributor status |
| [SECURITY.md](SECURITY.md) | Vulnerability reporting and supported versions |
| [Docs/development/contributing-guide.md](Docs/development/contributing-guide.md) | Day-to-day development workflow |
| [Docs/wfl-foundation.md](Docs/wfl-foundation.md) | 19 guiding principles and the No-Unlearning Invariant |
| [LICENSE](LICENSE) | Apache License 2.0 |

---

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
| **Maintainer** | Brad (Logbie LLC); additional people may be appointed | Final authority on technical direction, merges to protected branches, releases, security response, governance changes, trademark/project identity, and Contributor appointments |
| **Contributor** | People granted write access after application and approval | Open PRs from branches, review others’ work, triage issues as delegated, help enforce the Code of Conduct as delegated. Does **not** alone merge to `main` unless also a Maintainer or explicitly delegated for a path |
| **Participant** | Anyone who opens issues, discussions, or PRs from a fork | Propose changes, report bugs, improve docs; must follow the Code of Conduct |

There is no gatekeeping of *subject matter*: anyone may propose changes to any
area of the project. Access levels control *how* changes land, not *what* you
may care about.

### 2.2 Decision authority

| Decision type | Who decides | Notes |
|---|---|---|
| Day-to-day PR merge | Maintainer(s) | Based on review, CI, and project policies below |
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
- Design notes, Dev Diary entries, and package-design ADRs under `wflpkg/` and `Dev diary/`  

Input is advisory unless a Maintainer adopts it. Silence is not consent for
breaking changes; Maintainers still own the compatibility bar.

---

## 3. Binding technical policies

These policies are **non-negotiable** for accepted contributions. They already
appear in `AGENTS.md`, `CLAUDE.md`, the contributing guide, and collaboration
docs; this section makes them governance-level requirements.

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
- A `Dev diary/` entry for non-trivial features or behavior changes  
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
[Docs/development/contributing-guide.md](Docs/development/contributing-guide.md)
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
- Maintainers cut releases; Contributors do not publish project releases unless
  explicitly delegated.

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
   impact (template in the collaboration guide).  
5. Address review feedback. AI-assisted work is welcome; the human author is
   accountable (see [AI_POLICY.md](AI_POLICY.md)).  
6. Maintainer merges when checks and policies are satisfied.

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
| Package / registry designs (`wflpkg/`, future hub) | Maintainers; supply-chain and trust-root decisions are Maintainer-only |
| Domain and brand references | Logbie LLC |
| Signing keys, release credentials | Maintainers only |

Design documents under `wflpkg/` may describe future registry **governance
risk** (longevity, key custody, transparency logs). Those designs do not
transfer authority away from Maintainers unless this document is amended.

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
