---
title: "WFL Docker Playground — Epic Brief"
kind: spec
---

# WFL Docker Playground

## Summary

Build an install-free playground on **wfl.fyi** where visitors can explore WFL using the official native Linux interpreter inside isolated, ephemeral sessions. The playground is also a deliberate WFL dogfooding showcase: WFL owns the public product experience and execution workflow while a narrow privileged boundary enforces Docker containment.

The service is an anonymous, best-effort public beta. It favors authentic WFL behavior, exploration, privacy, and fail-closed operation over persistence, unrestricted native access, or guaranteed availability.

## Context and problem

The existing **wfl.fyi/playground** is a curated placeholder. It demonstrates WFL examples but does not provide the intended arbitrary-project experience. This initiative replaces that placeholder with the full sandboxed playground rather than introducing a separate or coexisting product.

Without the replacement, visitors still need to install and configure a local runtime to explore their own WFL programs. Curated examples can demonstrate syntax, but they cannot provide the feedback loop of importing, editing, running, inspecting, testing, interacting with, and serving a visitor-authored WFL project.

Removing that installation barrier creates a second problem: the service must execute untrusted native programs from anonymous internet visitors. The product therefore needs to feel like real WFL without allowing visitor programs to inherit the host's authority, reach unrelated systems, retain cross-session state, or consume unbounded shared capacity.

This initiative matters both as an onboarding experience and as evidence that WFL can power a serious interactive service of its own.

## Who is affected

| Audience or system | Need or responsibility |
| --- | --- |
| New and curious visitors | Try meaningful WFL programs immediately without installing a runtime. |
| Existing WFL users | Import and explore projects in an environment that behaves like a pinned official WFL release. |
| WFL maintainers | Demonstrate the language honestly while preserving backward compatibility, documentation quality, and release discipline. |
| Playground operator | Keep anonymous execution bounded, observable, recoverable, and affordable without promising production-grade availability. |
| wfl.fyi | Present the supported playground experience; the execution backend is not a general public API. |

## Product promise

The playground will provide:

- The official, pinned native Linux WFL executable, with its version disclosed to the visitor.
- WFL semantic fidelity and relevant CLI-compatible diagnostics, output, and outcomes.
- Run-to-completion programs, language tooling and tests, interactive REPL sessions, and temporary web-server previews.
- Ephemeral full-project-folder exploration through a sanitized session workspace.
- Clear feedback when a capability is restricted, a limit is reached, capacity tightens, or work cannot be admitted.
- No retention of visitor code, inputs, workspace content, output, or artifacts beyond the active session.

“Real WFL” does not mean unrestricted WFL. The official interpreter operates under a visible playground capability profile designed for anonymous execution.

## Scope boundaries

| In scope for v1 | Explicitly outside v1 |
| --- | --- |
| Replacement of the current curated `/playground` placeholder on wfl.fyi | A second coexisting playground experience |
| WFL-centered public API, session behavior, admission, queueing, run state, streaming, and preview coordination | Direct Docker authority in the internet-facing WFL process |
| Narrow privileged helper that enforces Docker lifecycle and sandbox policy | Accounts, authenticated user workspaces, saved history, or cross-session persistence |
| Workspace-confined filesystem behavior and SQLite | Visitor subprocess execution and external PostgreSQL/MySQL connections |
| Temporary bounded server and WebSocket previews | Directly exposed host ports or persistent application hosting |
| No guest outbound network at launch | General-purpose or documented public execution API |
| A deliberate future seam for curated outbound destinations | Implementing curated egress in v1 |
| Fixed pre-provisioned capacity with adaptive allowances and load shedding | Automatic paid scaling in v1 |
| A future capacity-provider seam | Designing or implementing the future autoscaling model now |
| Short-lived wfl.fyi website-flow tokens and demand-triggered anonymous challenges | Treating tokens, CORS, or origin checks as user authentication |

## Operating stance

The playground launches as a **best-effort public beta** with no uptime, queue-time, or execution-availability guarantee. It remains within fixed capacity and self-regulates through hard per-execution ceilings, anonymous active-compute allowances, idle limits, global concurrency controls, queueing, rejection, and fail-closed shutdown.

Ordinary disconnections receive a brief reconnect grace. Cancellation, expiry, quota or safety violations, crash recovery, and grace-window expiry lead to bounded teardown of containers, routes, tokens, child processes, and workspace content. Unknown or orphaned resources are removed during reconciliation.

## Privacy stance

Visitor source, stdin, files, stdout/stderr, and generated artifacts are session-only data. They are not retained for debugging, analytics, or product history. Operational records are limited to the minimum short-lived metadata needed for capacity, provenance, security response, and abuse controls. Visitor content enters a support record only through explicit visitor submission.

## Launch quality bar

Resource limits and a best-effort label do not reduce the verification standard. Public launch is blocked until the internal release gate passes:

- Mandatory TDD with failing tests before implementation.
- End-to-end coverage across the WFL control plane, privileged-helper seam, Docker lifecycle, token and admission behavior, every execution mode, previews, workspace import, resource enforcement, cancellation, reconnect grace, crash/restart reconciliation, teardown, and privacy behavior.
- Adversarial and negative coverage for isolation, malformed inputs, token misuse, resource exhaustion, orphan cleanup, and fail-closed behavior.
- A written threat model, pinned interpreter/image provenance checks, verification that visitor content does not enter persistent logs or backups, and a rehearsed kill switch plus rebuild procedure.

An independent security review is desirable if resources later permit it, but is not a v1 prerequisite. The project explicitly accepts the residual risk of relying on rigorous internal verification.

## Success looks like

- A visitor can open wfl.fyi, import or create a WFL project, use each supported execution mode, and understand both WFL results and playground-imposed restrictions without installing anything.
- The experience truthfully identifies the WFL version and capability profile used for each run.
- Anonymous demand cannot force unbounded execution or automatic spending; the service degrades transparently and fails closed.
- Session termination reliably removes visitor content and execution resources without cross-session leakage.
- WFL visibly owns the product orchestration while privileged Docker authority remains isolated behind a narrow enforcement boundary.

## Durable context

Detailed settled decisions and their rationale are preserved in the sibling **WFL Docker Playground — Decision Log** artifact. Product journeys and technical mechanisms remain intentionally deferred to core-flow and technical planning.
