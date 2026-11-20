---
id: PRD-0000
title: <short title>
status: Draft
owner: <name, or the agent that drafted it>
created: YYYY-MM-DD
approved_by:
approved_on:
supersedes:
---

# PRD-0000: <short title>

<!-- Copy this file to PRD-NNNN-short-name.md, using the next free number (numbers are never reused). Replace every <...>. Keep every "##" heading, in this order; put extra structure under "###" headings. Rules: Docs/contributing/prd-process.md. Check: python scripts/check_prd_docs.py -->

## Problem

<Who has the problem, what happens today, and why it matters. Link the issue or idea it came from. An agent-proposed PRD says what evidence led to it.>

## Goals

- <An outcome this PRD delivers.>

## Non-goals

- <Something a reader might expect that this PRD does NOT deliver.>

## Requirements

- **REQ-01** — WFL MUST <one behavior that can be tested>.

## Acceptance criteria

- **AC-01** (REQ-01) — Given <state>, when <action>, then <result that can be observed>.

## Compatibility

<Can any existing WFL program, configuration file, CLI invocation, or documented output behave differently? Does this reserve a new word (which breaks programs that use it as a name)? If anything existing can break, this is a breaking change: GOVERNANCE.md section 3.1 applies and Brad merges it.>

## Security and privacy

<The risks and how they are handled, or why there are none. An R3 PRD (testing.md section 5) includes or links a threat model.>

## Test plan

<Risk class (testing.md section 5). For each acceptance criterion: the Red test, and the end-to-end test through the real shipped artifact (release wfl binary, wfl-lsp process, VS Code extension host, or a running website instance).>

## Open questions

None.

## Revision history

- YYYY-MM-DD — Created.
