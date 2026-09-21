# rustls TLS 1.3 record-boundary remediation

## Assessment and scope

- Risk class: **R3** (cryptography, untrusted network input, protocol guarantees).
- Advisory: RUSTSEC-2026-0285 / upstream GHSA-2mjx-qc3c-rqvc.
- Baseline: `23c1a4577da68d853fa30c49a17773427471eca4` (WFL 26.9.16).
- Red test-only ancestor: `e02d72d14b1ee0d56606e47c16515fb8905fe142`.
- The fix is the subsequent commit changing Cargo.toml and both Cargo.lock files;
  the private pull request records its exact SHA and validation status.

The baseline resolves rustls 0.23.42. `cargo tree --locked -i rustls` shows
reqwest 0.13.4 / hyper-rustls 0.27.9, tokio-rustls 0.26.4, sqlx-core 0.9.0,
and WFL sharing that version. The standalone fuzz workspace has the same paths.

WFL's outbound HTTPS client in `src/interpreter/mod.rs` uses reqwest's rustls
backend. Secured listeners in `src/interpreter/tls.rs` use tokio-rustls with
TLS 1.2/1.3 enabled and no client authentication. PostgreSQL and MySQL/MariaDB
connections in `src/interpreter/database.rs` delegate TLS negotiation to SQLx
when the connection settings/peer enable it. SQLite and plaintext HTTP do not
exercise TLS handshakes. SQLx protocol reachability is established from the
dependency graph and call sites; live PostgreSQL/MySQL TLS services were not
tested in this environment.

The regression confirms acceptance of an invalid TLS 1.3 encryption boundary,
not an authentication bypass or a completed attacker-controlled handshake.
The [upstream advisory](https://github.com/rustls/rustls/security/advisories/GHSA-2mjx-qc3c-rqvc)
states that the handshake transcript remains authenticated. Its Moderate/5.3
severity is retained; this work does not establish a deployment-specific score.

## Change

Require rustls `^0.23.45`, so fresh resolution cannot select the affected 0.23
versions. Both lockfiles resolve exactly:

| Package | Before | After |
|---|---|---|
| rustls | 0.23.42 | 0.23.45 |
| rustls-webpki | 0.103.13 | 0.103.15 |
| aws-lc-rs | 1.17.1 | 1.18.1 |
| aws-lc-sys | 0.42.0 | 0.45.0 |

The companion changes are required by rustls 0.23.45's updated dependency
requirements (aws-lc-rs >=1.18 and webpki >=0.103.14). No other package was
updated. No WFL syntax, TLS configuration, provider selection, or data format
changes. Existing installed binaries must be rebuilt/replaced to receive the fix.

## Acceptance criteria and Red → Green

`tests/tls_handshake_boundary_test.rs` exercises real rustls connections with
fresh test certificates, without mocking the record parser:

- `rejects_plaintext_encrypted_extensions_coalesced_with_server_hello`: append
  a complete EncryptedExtensions to a genuine ServerHello record. On 0.23.42,
  `process_new_packets` returned successful `IoState`; the required rejection
  assertion failed. The valid-handshake control passed in the same run
  (one pass, one failure, exit 101). On 0.23.45 the malformed record produces
  `PeerMisbehaved(KeyEpochWithPendingFragment)` and exposes no plaintext.
- `accepts_correctly_framed_tls13_and_authenticated_application_data`: trust
  the generated certificate, complete both sides of TLS 1.3, and verify the
  exact encrypted application payload after decryption.
- `rejects_coalesced_record_at_every_transport_split`: exhaustive two-chunk
  transport partition property for the generated malformed record, covering
  record-header and handshake-data splits. The fixture is generated from real
  connections rather than storing ephemeral keys or random record bytes.
- `wfl_https_rejects_coalesced_record_over_a_real_socket`: execute WFL's
  outbound HTTPS POST against a loopback TCP peer sending the malformed record.
  Require a fatal TLS `unexpected_message` alert, a WFL request error, no
  response variable, and bounded completion. EOF/timeout alone is not success.

The first two tests were committed before changing dependencies. The additional
transport/property tests broaden the same regression after the fix. A reserved
WFL identifier in the added socket fixture was corrected before its passing run;
that parse failure is not counted as Red evidence.

## Validation

Local environment: Windows x86-64, rustc 1.98.1, cargo-audit 0.22.2.
`RUST_MIN_STACK=8388608` for the full Windows suites, as required by the runner.
Build output was reused under an external Cargo target directory; the newly
built release executable was copied into this checkout's `target/release/`
for the repository scripts. No installed system WFL executable was substituted.
Raw logs and audits are under `target/reports/rustls-advisory/` (ephemeral).

- `cargo test --locked --test tls_handshake_boundary_test`: 4 passed after fix.
- `cargo test --locked --test web_server_tls_test`: 13 passed. Retains TLS 1.2,
  TLS 1.3, ALPN h2/HTTP/1.1, stalled-handshake isolation, listener shutdown,
  malformed/mismatched keys, bind failures, redirect and configuration coverage.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: passed.
- `cargo check --locked --manifest-path fuzz/Cargo.toml`: passed. This is a
  compile check, not a fuzz campaign. The exhaustive record-partition test is
  the focused property test for this patch.
- `cargo build --locked --release`: passed.
- `python scripts/validate_docs_examples.py --ci --force`: 36 passed.
- `cargo test --locked --all --verbose`: 2,418 passed, zero failed, 27 ignored,
  including workspace and doctests. Existing ignored tests/doctests remain unchanged; none were added
  or disabled to obtain this result.
- `./scripts/run_web_tests.ps1`: 3/3 passed, including release-binary HTTPS and
  redirect behavior; no TLS case skipped.
- `target/release/wfl --execution-timeout 330 --test
  tests/fixtures/cli_budget/long-run.test.wfl`: 1 passed (305-second test).
- `python scripts/check_repo_hygiene.py --mode static`: passed.
- `./scripts/run_integration_tests.ps1 -TestOnly`: passed Rust integration
  suites and 164 release-binary WFL programs, zero failures. The runner's 24
  pre-existing exclusions/directives are unchanged (specialized web/TLS cases
  were exercised separately above); no new skips were introduced.

Windows runner setup failures were diagnosed before the successful web run:
the inherited environment had conflicting case variants of PATH; the installed
OpenSSL default referenced a nonexistent config; and sandboxed Schannel failed
with `No credentials are available in the security package`. A child-process
environment with a single PATH and the installed OpenSSL configuration, followed
by running the unchanged web script outside the sandbox, resolved these setup
problems. The original OpenSSL/Schannel failure logs were retained. No TLS
assertion, timeout, certificate check policy, or production setting was weakened.
An additional release-binary probe verified HTTP 200 and the expected body with
certificate-validating Python/OpenSSL clients restricted separately to TLS 1.2
and TLS 1.3. It also exposed the exact Schannel error used in the diagnosis.

Baseline and patched `cargo audit --json` reports were inspected. After the fix,
RUSTSEC-2026-0285 is absent from both root and fuzz reports. Both audits still
exit 1 for pre-existing RUSTSEC-2026-0258 in h2 0.3.27 and 0.4.15. These findings
are not ignored, remediated, or claimed clean by this narrowly scoped patch.

## Review, residual risk and recovery

Independent security/maintainer review and supported-platform CI remain merge
gates; local tests do not substitute for that review or Linux validation.
No numerical coverage percentage was collected; acceptance coverage is the
explicit boundary/property/socket mapping above, supplemented by existing suites.
No release, public disclosure, advisory closure, or default-branch merge is
performed as part of preparing this fix. The unchanged h2 findings need separate
remediation/disposition before claiming a clean overall dependency audit.

No schema, persisted state, authorization policy, or credential change is made,
so migration, tenant authorization, secret-redaction, and data restore tests are
not newly triggered. Existing workspace HTTP lifecycle/budget/streaming tests
cover higher-level resource, cancellation, and shutdown behavior; this patch
adds no new queues or async ownership mechanism.

Recovery is to revert the dependency commit and rebuild/redeploy, which restores
the affected rustls version and is therefore not a security-safe steady state.
Prefer forward repair within patched rustls versions if compatibility problems
are discovered. No runtime data migration or external state rollback is needed.
