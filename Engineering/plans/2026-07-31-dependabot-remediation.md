# Dependabot Alert Remediation Plan

> **Date:** 2026-07-31
> **Owner:** WFL maintainers
> **Branch:** `warden/dependabot-remediation`
> **Base:** `438780ae038608783c54fdc661273f040c4c58dd`
> **Risk:** **R3** — the Rust remediation replaces TLS transport plumbing and
> therefore touches a security boundary, protocol negotiation, concurrency, and
> server lifecycle. The npm lockfile refresh is dependency maintenance.

## Goal

Resolve all ten Dependabot alerts open on 2026-07-31 without changing WFL
language syntax or the public behavior of plain HTTP, secured HTTP, redirects,
WebSockets, request metadata, or `close server`.

The open-alert baseline is:

- `Cargo.lock`: four `rustls-webpki` advisories through
  `warp 0.3.7 -> tokio-rustls 0.25 -> rustls 0.22 -> rustls-webpki 0.102.8`.
- `fuzz/Cargo.lock`: the same four advisories through the path dependency on
  WFL.
- `vscode-extension/package-lock.json`: `js-yaml 4.2.0` and the
  `brace-expansion 5.0.7` copy nested below `minimatch 9.0.7`.

## Compatibility constraints

- Keep Warp 0.3.7 for the existing routing/filter API; do not mix a Warp
  migration into this security change.
- Remove only Warp's legacy `tls` feature and serve the same Warp filters over
  Tokio-Rustls 0.26 / Rustls 0.23.
- Preserve TLS 1.2 and TLS 1.3, HTTP/2 and HTTP/1.1 ALPN, peer IP reporting,
  port-zero address reporting, actionable certificate/key errors, handshake
  isolation, and server shutdown semantics.
- Keep plain HTTP, redirect, and standalone WebSocket listeners on their
  current Warp server path.
- Preserve existing user-owned staged worktree entries; stage and commit only
  files named by this plan.

## Evidence model

Most of this work is behavior-preserving dependency maintenance under
`testing.md` §6.3, so artificial behavioral failures are not appropriate. The
new characterization suite found one genuine pre-existing failure: Warp's
Rustls 0.22 configuration accepted a certificate paired with an unrelated
private key when the listener started. The replacement must reject that invalid
pair before binding, so that criterion follows full R3 Red → Green chronology.
Independent review then found a second genuine failure in the first replacement
draft: aborting the outer Hyper server left accepted connection tasks alive.
The lifecycle test was committed while Red before tracked cancellation was
added. The other TLS characterizations establish a passing pre-change baseline.
The security acceptance criteria also have an observable failing baseline: the
ten live alerts and both lockfiles' inverse dependency path to
`rustls-webpki 0.102.8`.

After the change, run the same suite and prove the mismatched pair is rejected
and the vulnerable package is absent from both lockfiles. Retain command output
and commit IDs in the pull request. An independent agent that did not author the
implementation must review the final diff and evidence.

## Tasks

### 1. Characterize the secured-listener contract

Add real-socket integration coverage in `tests/web_server_tls_test.rs` for:

- occupied-port errors returned synchronously and with useful context;
- a stalled or invalid TLS client not blocking a subsequent valid client;
- request `client_ip` surviving the custom TLS transport;
- HTTP/2 ALPN plus HTTP/1.1 fallback;
- malformed key and certificate/key mismatch errors;
- `close server` cancelling an established idle connection and a ClientHello-
  stalled connection, followed by immediate address reuse;
- a port-zero secured listener reporting its actual ephemeral address.

Run the complete existing TLS integration test target on the base and record
the passing result before production changes.

### 2. Remove the legacy Rustls dependency chain

- Change `Cargo.toml` so Warp no longer enables its `tls` feature.
- Add Tokio-Rustls 0.26 with the ring provider and TLS 1.2 enabled; reuse the
  existing direct Rustls 0.23 and Rustls-PEMFile 2 dependencies.
- Implement the secured listener with an explicit ring-backed
  `rustls::ServerConfig`.
- Accept handshakes concurrently and serve the unchanged Warp filter through
  Warp's matching Hyper 0.14 re-export.
- Track Hyper's executor-spawned connection tasks and cancel them when the WFL
  server task is aborted or otherwise ends.
- Inject the accepted peer `SocketAddr` into request extensions and make the
  shared request filter fall back to that extension only for this custom path.
- Keep Hyper's server lifecycle under the existing WFL task handle so aborting
  it stops accepts, then cancel every tracked accepted-connection task after
  WFL's existing 50 ms response-flush allowance.
- Return certificate, key, key-mismatch, and bind failures as WFL runtime
  errors rather than panics.

Regenerate both `Cargo.lock` and `fuzz/Cargo.lock`. Prove that neither contains
`rustls-webpki 0.102.8`, `rustls 0.22`, or `tokio-rustls 0.25`.

### 3. Remediate the extension lockfile

- Raise the existing `js-yaml` override to 4.3.0.
- Regenerate `vscode-extension/package-lock.json` with npm rather than editing
  integrity metadata manually.
- Allow the semver-compatible nested `brace-expansion` update to 5.0.8.
- Confirm the lockfile's product version agrees with root `Cargo.toml`.
- Run `npm ci`, compile, lint, and the extension tests.

### 4. Broaden and record

Run:

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo build --release
scripts/run_integration_tests.ps1
scripts/run_web_tests.ps1
python scripts/validate_docs_examples.py
python scripts/check_repo_hygiene.py --mode static
```

Also compile the fuzz workspace and run dependency-tree checks for both Rust
lockfiles. Document the implementation and evidence in a dated Dev Diary entry.

### 5. Publish without merging

- Commit the passing characterization baseline separately from the production
  remediation.
- Request independent R3 review and address actionable findings.
- Push `warden/dependabot-remediation`, open a normal pull request, and do not
  merge it.
- Re-query Dependabot after GitHub processes the pushed manifests. If alert
  rescanning is still pending, report that explicitly rather than claiming the
  dashboard is already clear.
