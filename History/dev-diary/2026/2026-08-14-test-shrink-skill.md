# 2026-08-14 — test-shrink skill and orchestration agents

## What

Added an on-demand Claude Code project skill, `test-shrink`
(`.claude/skills/test-shrink/SKILL.md`), plus three reusable subagent
definitions it orchestrates:

- `.claude/agents/research-agent.md` — read-only scout that measures and ranks
  shrink candidates (may web-search current Rust practices).
- `.claude/agents/code-monkey.md` — implements exactly one scoped change and
  verifies it with targeted tests.
- `.claude/agents/code-reviewer.md` — gatekeeper returning KEEP / REVISE /
  REJECT, with coverage preservation as its non-negotiable question.

## Why

The Rust test suite (~146 files, ~43k lines) generates a large volume of debug
data and duplicated harness code. The maintainer wants an occasional
maintenance pass that shrinks test size and output as far as possible while
every Rust test and every `TestPrograms/` program keeps passing.

## How it works

The skill runs a loop: green-baseline preflight → measure (source lines,
fixture size, captured output bytes, on-disk artifacts, wall time) → parallel
research agents produce a ranked worklist → serial shrink loop (code-monkey
implements one candidate, code-reviewer verdicts the diff, full gate verifies,
keep-as-one-commit or revert) → wrap-up with before/after metrics, hygiene
check, and a fresh Dev Diary entry per pass.

Guardrails are drawn from the binding Logbie Testing Policy: no weakened
assertions, no deleted coverage, no manufactured green, real boundaries stay
real, and maintainer-decision items (e.g. `[profile.release] debug = true`,
CI config) are reported as proposals rather than applied.

## Research playbook (same-day follow-up)

Distilled the maintainer-supplied research on Rust compiler/interpreter test
architecture (matklad's testing/build essays, rustc dev-guide best practices,
rust-analyzer and pydantic-monty case studies) into
`.claude/skills/test-shrink/references/rust-test-optimization.md`. Key content:

- **Test-binary proliferation:** each of the ~146 files under `tests/` is its
  own statically-linked binary with debug info — identified as the primary
  source of the suite's disk/debug-data bloat; the playbook documents the
  incremental single-binary consolidation migration.
- **`check`-driver pattern** for deduplicating harness code without coupling
  tests to compiler internals.
- **`[profile.test]` tuning** promoted to an ordinary candidate (release
  profile stays untouchable); snapshot-crate adoption, `cargo-nextest`, and
  linker changes remain maintainer proposals.
- **rustc hygiene rules:** minimal fixtures, descriptive names, suppressing
  unrelated warning noise.

SKILL.md's research phase now points agents at the playbook with a priority
order, and treats its claims as hypotheses to verify and measure.

## Notes

- No product code changed; this is tooling/process only.
- The agents are written to be reusable by other orchestration loops, not
  just test-shrink.
