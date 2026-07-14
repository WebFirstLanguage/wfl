---
title: "WFL Docker Playground — Decision Log"
kind: spec
---

# Decision log

## 2026-07-14 — Existing playground replacement

- The current `wfl.fyi/playground` experience is a curated placeholder.
- This initiative replaces that placeholder with the full sandboxed playground; it is not a greenfield route and does not create a second coexisting playground product.
- Replacement, deployment, and rollback mechanics remain downstream core-flow and technical-planning concerns.

## 2026-07-14 — Product purpose and Docker-control boundary

- The playground is intentionally a WFL dogfooding showcase: visitors can use the native WFL experience without installing a runtime, while WFL powers the public backend and coordinates execution.
- Docker is the selected outer containment mechanism for untrusted visitor programs. The security design must assume intentionally malicious submissions, not only accidental mistakes.
- WFL owns the public API and execution workflow.
- A narrowly scoped privileged helper—rather than the internet-facing WFL process—will exclusively control Docker lifecycle operations.
- Direct Docker control from WFL is not required to satisfy the dogfooding goal.
- “No runtime installation” is a product goal, but is not by itself evidence that Docker is the only viable execution substrate.

## 2026-07-14 — Meaning of “real WFL interpreter”

- Playground execution must preserve WFL semantic fidelity and relevant CLI-compatible diagnostics, output, and outcomes.
- Each sandbox runs the official native Linux WFL executable rather than a reimplementation or browser port.
- The execution environment uses a pinned, reproducible WFL version and discloses that version with each result.
- Authenticity does not imply unrestricted capabilities: the official interpreter runs under an explicit sandbox capability profile.
- Dangerous native capabilities may be sandbox-scoped or unavailable without invalidating the “real WFL interpreter” promise.

## 2026-07-14 — v1 execution experiences and adaptive usage

- V1 includes all four execution experiences: run-to-completion programs; lint/analyze/type-check/test tooling; interactive REPL sessions; and temporary WFL web-server/listen previews.
- Every execution always has hard safety ceilings for resources and lifetime, even when the service is otherwise idle.
- Anonymous visitors receive a rolling active-compute allowance that is generous or effectively uncapped when spare capacity exists and contracts as demand rises.
- REPL and server-preview sessions use separate idle limits; idle wall time is distinguished from active computation.
- Admission fails safely through global concurrency limits, queueing, or rejection when capacity is exhausted.
- Fairness may combine IP/IP-prefix signals with an anonymous browser token; the token is not an account or authentication.
- The user experience must explain current allowance, remaining active time, capacity-based tightening, and retry availability.
- Exact quota values and enforcement mechanisms are intentionally deferred to technical and operational planning.

## 2026-07-14 — Ephemeral project-folder workspace

- Each anonymous session receives an isolated, ephemeral workspace that may be populated by importing a complete local project folder.
- Imported content is reconstructed as sanitized regular files inside the workspace; the playground does not recreate arbitrary local filesystem semantics.
- Import processing normalizes paths into the workspace and rejects absolute paths, traversal, symlinks, hardlinks, device nodes, sockets, named pipes, and other special entries.
- Imported executable permissions are ignored.
- Imported `.wflcfg` files are rejected or ignored; a sealed playground configuration remains authoritative and cannot be widened by visitor content.
- Import and workspace quotas cover total bytes, individual file size, file count, directory depth, filename length, and upload duration.
- WFL programs may create files within the workspace, and permitted artifacts may be inspected or downloaded under output limits.
- The entire workspace is destroyed when its anonymous session expires; no cross-session persistence is provided.

## 2026-07-14 — WFL-centered control plane

- WFL owns the public API, interactive client connections, anonymous-session behavior, adaptive allowance policy, admission and queueing decisions, run/session state, execution-mode transitions, result streaming, preview coordination, and the frontend-facing status/error contract.
- WFL requests sandbox creation, cancellation, and termination but does not hold direct Docker authority.
- A narrow privileged helper exclusively accesses Docker, revalidates privileged requests, selects the pinned interpreter image, applies resource/filesystem/network restrictions, creates and terminates containers, reaps abandoned work, and returns bounded events and artifacts.
- The helper enforces sandbox policy but does not become the product-policy or scheduling brain.
- Standard infrastructure concerns such as TLS termination, host firewalling, and external monitoring may use appropriate non-WFL infrastructure without weakening the WFL dogfooding goal.

## 2026-07-14 — Initial guest capability profile

- Native filesystem behavior is confined to the ephemeral session workspace and its quotas.
- Visitor subprocess execution is disabled.
- SQLite is available inside the session workspace; external PostgreSQL and MySQL connections are unavailable in v1.
- WFL listen/WebSocket programs may use temporary, bounded preview routing but never directly expose host ports.
- Guest environment variables contain only fixed, non-secret playground values.
- Time and randomness retain native behavior.
- Generated regular-file artifacts may be inspected or downloaded under type and size limits.
- Host and internal services are unreachable.
- Unsupported capabilities return explicit playground restriction errors rather than silently pretending to succeed.
- V1 launches with no guest outbound network access (N1).
- The capability model and control-plane contract must permit a later, deliberate move to curated destinations (N2) without redesign, but N2 is not implemented as a hidden or dormant bypass. Enabling it requires an explicit policy change and review.

## 2026-07-14 — Public-beta posture, fixed capacity, and caller gate

- The playground is a public, best-effort beta with no uptime, queue-time, or execution-availability guarantee.
- The service self-regulates through adaptive allowances, queueing, rejection, and fail-closed shutdown when safety or capacity thresholds are crossed.
- V1 uses fixed, pre-provisioned capacity (R1) and does not implement automatic paid scaling.
- The architecture must retain a clean capacity-provider seam so a later cost-capped scaling model is not made prohibitively difficult, but no R2 autoscaling behavior is planned or built in v1.
- The only supported caller is the official playground on wfl.fyi; v1 is not a documented or supported public execution API.
- Execution requires short-lived, signed tokens scoped to the anonymous website session and permitted operation.
- Normal anonymous visitors receive website-flow tokens (T1). When demand or abuse increases, token issuance may require an anonymous human challenge (T2).
- Tokens, browser-origin checks, IP/IP-prefix signals, and anonymous session signals work together with quota and admission controls. They are not user authentication and cannot prove a visitor's identity.
- Direct clients remain hostile inputs even though they are unsupported; CORS, hidden endpoints, and origin checks are defense-in-depth rather than the abuse-control boundary.

## 2026-07-14 — Privacy-first, session-only content

- Visitor source, stdin, workspace files, stdout/stderr, and generated artifacts exist only for the active anonymous session and are destroyed with it.
- The service does not retain visitor content for debugging, analytics, or product history.
- Operational records are limited to the minimum short-lived metadata needed for capacity management, limit enforcement, interpreter/image provenance, security response, and abuse prevention.
- IP-derived information may be processed transiently for quota and abuse controls; retained identifiers must be minimized, short-lived, access-controlled, and excluded from ordinary backups where practical.
- Visitor code or output may enter a support record only when the visitor explicitly submits it as part of an error report.
- The privacy behavior must be disclosed clearly to visitors without implying that in-session processing does not occur.

## 2026-07-14 — Disconnect grace and fail-closed teardown

- Ordinary browser or network disconnection enters a short, fixed reconnect-grace window (L2).
- Only the same anonymous session may reclaim work during that grace window; the exact duration is deferred to core-flow and technical planning.
- When the grace window expires, the session, containers, preview routes, tokens, workspaces, child processes, and in-session content are destroyed.
- Explicit cancellation, quota exhaustion, safety-limit violations, and session expiry trigger teardown without relying on the reconnect window.
- Backend/helper crashes and host restarts require fail-closed reconciliation that terminates and removes unrecognized or orphaned resources.
- Teardown has a bounded completion requirement and escalates from cooperative cancellation to enforced termination when necessary.

## 2026-07-14 — Internal-only public-launch verification gate

- Public launch uses an internal-verification-only posture (V1) because resources for an independent external review or staged private program are not available.
- This choice explicitly accepts the residual risk that internal testing cannot prove the absence of container escapes or design flaws that an independent reviewer might find.
- Mandatory TDD applies to every feature and security control: failing tests precede implementation.
- End-to-end integration testing is mandatory across the public WFL control plane, privileged-helper boundary, Docker lifecycle, token/admission flow, every supported execution mode, preview routing, workspace import, resource enforcement, cancellation, disconnect grace, crash/restart reconciliation, teardown, and privacy behavior.
- Adversarial and negative testing must cover guest-to-host, guest-to-control-plane, guest-to-guest, internal-network isolation, malformed imports, resource exhaustion, token misuse, orphan cleanup, and fail-closed behavior.
- Launch evidence includes a written threat model; pinned interpreter/image provenance and vulnerability checks; verification that visitor content is absent from persistent logs/backups; and a rehearsed kill switch plus clean rebuild/recovery procedure.
- The anonymous public beta does not launch until all internal release gates pass. Failing or unverified controls block launch rather than becoming accepted post-launch TODOs.
- An independent review remains desirable if resources later become available, but is not a v1 prerequisite.
