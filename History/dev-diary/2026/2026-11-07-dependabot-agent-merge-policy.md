# 2026-11-07 — Let agents merge eligible Dependabot updates

Dependabot pull requests kept waiting for a human merge, even for small,
compatible patch updates such as `rustls` 0.23.45 → 0.23.46 (#805, #806) and
`brace-expansion` 1.1.18 → 1.1.21 (#775). The governance suite had no rule for
them. The issue policy listed "dependency … changes" as I2 (Maintainer approval
before dispatch). `GOVERNANCE.md` said the I1 fix was the *only* standing agent
merge exception. Merges into `main` were limited to Brad, or Azusa at the
exact-commit gate. That last point matters because Dependabot sends security
updates to the default branch, `main`, and the repository has no
`.github/dependabot.yml` to point version updates elsewhere.

## What changed

- `Docs/contributing/issue-policy.md` gained **Dependabot pull requests**, a
  second standing delegation. It sets out eligibility, risk class, merge gates,
  and what to do when an update is not eligible. The I2 row now points
  Dependabot pull requests to that section.
- `GOVERNANCE.md` gained **§3.10 Agent merges of Dependabot updates**. The
  common policy, the decision table (§2.2), and the PR process (§5) now name
  two standing exceptions instead of one. The `dev → main` promotion rule and
  the CEO `main` gate each note the Dependabot exception, and that it grants no
  build, release, tag, or deployment authority.
- `AI_POLICY.md`, `CLAUDE.md`, and
  `Docs/06-best-practices/collaboration-guide.md` now mention both delegations.

## Why the rule is shaped this way

- **Dependabot is the author.** The "authors do not merge their own pull
  requests" rule is met, so the reviewing agent can approve and merge. The
  approval must rest on the agent's own reading of the diff, the release notes,
  and any advisory. Repeating Dependabot's description does not count.
- **Semver-compatible only.** Cargo and npm treat a `0.x` minor step as
  breaking, so the rule bans `0.x` minor updates as well as major updates.
  Backward compatibility is sacred, and a breaking dependency change can break
  WFL programs.
- **No pushes to Dependabot branches.** If anyone else adds a commit, the
  change is no longer Dependabot's alone, and the independence argument fails.
- **72-hour wait for non-security updates.** This gives the ecosystem time to
  report and yank a compromised or broken release. Security updates for an open
  alert skip the wait, because waiting leaves a known vulnerability in place.
- **Security-sensitive packages are `R3` and patch-only.** This matches
  `testing.md`, which classes cryptography, TLS, authentication, and
  untrusted-input boundaries as `R3`.
- **Cargo and npm only.** GitHub Actions and Docker updates change workflows,
  release builds, and images, which `GOVERNANCE.md` reserves for Brad's
  approval.

## What this does not do

Policy text does not configure GitHub. Agent auto-merge needs the repository
setting **Allow auto-merge** on, and the LOG-16 ruleset still requires an
approving review from someone other than the author. This change adds no
`.github/dependabot.yml`, adds no workflow, and changes no ruleset.

Behavior tests N/A — documentation only (governance prose; no executable
behavior changed).
