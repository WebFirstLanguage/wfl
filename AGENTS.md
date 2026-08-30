# Repository Guidelines

**Canonical agent instructions live in [`CLAUDE.md`](CLAUDE.md) — read that file first.**

`CLAUDE.md` is the single source of truth for shared agent guidance: project
structure and architecture, build/test/dev commands, coding style, the binding
Testing and Repository-Hygiene policies, documentation rules, LSP workflow, and
the **Cursor Cloud specific instructions** (Rust ≥ 1.94 toolchain requirement and
the `run_integration_tests.sh` / `run_web_tests.sh` end-to-end flows).

This file intentionally carries no policy of its own, so `AGENTS.md` and
`CLAUDE.md` can never drift — update `CLAUDE.md` and everything stays in sync.

Binding policy referenced throughout lives at the repository root: `GOVERNANCE.md`,
`CODE_OF_CONDUCT.md`, `AI_POLICY.md`, `CONTRIBUTING.md`, `SECURITY.md`,
`testing.md`, and `REPOSITORY_HYGIENE.md`.
