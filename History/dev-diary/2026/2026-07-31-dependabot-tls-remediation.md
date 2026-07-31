# Dependabot remediation: secured listeners on Rustls 0.23

**Date:** 2026-07-31
**Risk class:** R3 (TLS boundary, protocol negotiation, concurrency, lifecycle)

## Why this changed

WFL's secured web listener still used Warp 0.3.7's optional TLS adapter. That
adapter pinned Tokio-Rustls 0.25, Rustls 0.22, and Rustls-WebPKI 0.102.8 even
though the rest of WFL already used the patched Rustls 0.23 /
Rustls-WebPKI 0.103 line. The duplicate legacy chain produced four Dependabot
alerts in the root lockfile and the same four alerts in the fuzz lockfile.

Warp remains at 0.3.7 for its routing/filter API. Only its `tls` feature was
removed. WFL now supplies the small transport adapter that serves the unchanged
Warp filters through Tokio-Rustls 0.26 and Rustls 0.23.

## Compatibility decisions

- TLS configuration uses an explicit ring crypto provider, so embedders and
  tests do not depend on a process-global provider having been installed first.
- TLS 1.2 and TLS 1.3 remain enabled.
- ALPN still prefers HTTP/2 and falls back to HTTP/1.1.
- A TLS handshake is driven by Hyper's per-connection task rather than the
  listener accept loop. A silent or malformed client therefore cannot block
  unrelated connections.
- Hyper's executor-spawned connection tasks are tracked without retaining
  completed tasks. Dropping or aborting WFL's existing server task cancels all
  accepted connections, including an established idle connection and a client
  stalled before its ClientHello.
- The accepted peer address is copied into the Hyper request extensions before
  the Warp filter runs. The shared request filter prefers Warp's normal remote
  address and falls back to that extension on the secured path, preserving
  `client_ip`.
- Certificate and key files are loaded once before binding. Rustls 0.23 also
  verifies that the private key matches the end-entity certificate, producing
  an actionable runtime error instead of starting a listener with an invalid
  pair.

Plain HTTP, redirect listeners, standalone WebSocket listeners, language syntax,
and configuration precedence were not changed.

The migration does not add a separate application-level cap for pre-HTTP TLS
connections. The tracker retains only currently active Hyper tasks and remains
bounded by the operating system's accepted-connection resources; adding a
configurable handshake limit or timeout is a separate hardening change.

## Other alert fixes

The VS Code extension lockfile was regenerated with:

- `js-yaml` overridden to 4.3.0 for GHSA-52cp-r559-cp3m;
- the affected nested `brace-expansion` copy updated past 5.0.7 for
  GHSA-mh99-v99m-4gvg;
- the lockfile's product version synchronized to the current repository version.

## Test evidence

The test-first revision added real-socket coverage for occupied ports, a stalled
TLS handshake followed by healthy clients, TLS 1.2 and 1.3, HTTP/2 ALPN and
HTTP/1.1 fallback, peer-address propagation, malformed keys, certificate/key
mismatch, and listener shutdown.

On the pre-change revision, eleven TLS tests passed and the new mismatch test
failed because the invalid pair was accepted. The first transport draft made
all twelve pass. Independent R3 review then required stronger lifecycle and
port-zero evidence. The lifecycle test failed Red because an established TLS
connection remained open after `close server`; after tracked task cancellation,
all thirteen tests passed, including a ClientHello-stalled connection, immediate
address reuse, and actual ephemeral-port reporting. Both root and fuzz
dependency trees now contain only Rustls-WebPKI 0.103.13; Cargo cannot resolve
the removed 0.102.8 package from either lockfile.

The final independent review also exercised a tracked task that panics. Before
the RAII registration guard, the active-task count remained at one after
unwinding; afterward it returns to zero on normal completion, cancellation, and
panic. The reviewer approved the resulting race handling and lifecycle evidence
with no remaining actionable findings.

The broader Green verification was:

- `cargo fmt --all -- --check`;
- `cargo clippy --all-targets --all-features -- -D warnings`;
- `cargo test --all -j 2` (the unrestricted parallel compile exceeded this
  Windows host's paging-file limit; two jobs completed the same suite);
- `cargo build --release -j 2`;
- `cargo check --manifest-path fuzz/Cargo.toml --all-targets`;
- 130 ordinary `TestPrograms` cases with the integration runner's timeout,
  expected-failure, `--test`, exclusion, and `CI-SKIP` rules;
- `scripts/run_web_tests.ps1` (2/2; its optional OpenSSL-generated TLS fixture
  was skipped because OpenSSL is unavailable, while the Rust TLS target ran
  all 13 real-socket cases);
- `python scripts/validate_docs_examples.py` (19/19);
- `npm ci --ignore-scripts` and `npm run compile`.

The extension's existing `npm run lint` command cannot find an ESLint
configuration or a matching `src` target, so its `pretest` stops before the
test runner. Invoking the test runner directly produced 24 passing, 2 pending,
and one pre-existing Windows line-ending assertion failure. Static repository
hygiene also reports the user-owned staged `.worktrees` entries and archive
hash drift caused by this checkout's CRLF-normalized files; none of those paths
is part of this remediation.
