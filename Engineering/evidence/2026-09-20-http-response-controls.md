# HTTP response controls evidence

Risk: R3 (HTTP protocol behavior, response limits, streaming and compatibility).
Base: `cb1dadaad96939a4450a6eb2b3a6a51678035b7f`.

## Acceptance criteria

- Existing requests follow redirects without source changes.
- The contextual clause `and without following redirects` returns the original
  status, body and headers for buffered and streaming requests.
- Existing scalar `headers` access remains compatible; additive `header_values`
  exposes every response-header value without joining cookie fields.
- No-follow requests keep the existing request budget, cancellation, body-size
  limit and streaming lifetime checks.
- WFL owns every new scenario, fixture, assertion and process cleanup. The
  existing recursive TestPrograms runners discover the new suite on both
  platforms. No external network service is used.

## Red, before implementation

On 2026-09-20, the downloaded Windows WFL 26.9.12 nightly binary ran:

```text
target/release/wfl.exe --test TestPrograms/http_redirects/redirects.test.wfl
Total: 1; Passed: 0; Failed: 1
the login redirect exposes its own status and cookie
Expected 200 to equal 302
```

The test uses syntax accepted by the baseline and an actual ephemeral-port WFL
server. It reproduces loss of the original redirect response; it does not rely
on a syntax error. The Green regression selects the new no-follow option
explicitly, while a separate compatibility test retains the default following
behavior. The test's `finally` stops its fixture process on assertion failure.

The Red test-only commit `66034730` is retained as an ancestor of the
implementation.

## Additional compatibility regression

The first full `cargo test --all --locked` exposed a source-fixer interaction:
when a program declares a variable named `without following redirects`, the
name-wide snake-case rewrite also changed the identically spelled contextual
clause. The existing source-preservation corpus test rejected the resulting
program. A new WFL regression independently reproduced the same behavior:
7 of 8 cases passed and the fixer case returned exit 2 instead of 0.

The fixer now treats the spelling as protected when a parsed HTTP statement
uses it as a clause, following its existing conservative handling of names
that also appear in public API positions. Other local names remain fixable.
The WFL regression checks both preservation and unrelated name fixing, then
parses the fixed file through the real CLI.

## Green validation

Local tuple: Windows x86-64; Rust/Cargo 1.98.1; WFL reports 26.9.12.

- `cargo build --release --locked` passed, followed by
  `target/release/wfl.exe --test TestPrograms/http_redirects/redirects.test.wfl`:
  **8 passed, 0 failed**.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- `cargo clippy --all-targets --all-features -- -D warnings` passed.
- `cargo test --all --locked` passed: 2,414 passed and 27 existing ignored tests
  across 175 result records, including workspace integration and doc tests.
  The existing HTTP request/parser, request budget, retry, stream lifetime,
  disconnect, ownership, and reaper-race suites all passed.
- `cargo check --locked --manifest-path fuzz/Cargo.toml` passed. Valid,
  duplicate, and incomplete WFL request seeds are retained under
  `fuzz/seeds/fuzz_parser/`; no sustained fuzz campaign is claimed.
- `scripts/run_web_tests.ps1` passed its two available scenarios. Its TLS
  scenario was skipped by the existing runner because OpenSSL is unavailable
  on this host; the Rust TLS tests were included in the passing cargo suite.
- `scripts/run_integration_tests.ps1 -TestOnly` passed its Rust integration
  stage and the recursively discovered WFL programs: **145 passed, 0 failed,
  24 existing skips**, including `PASS redirects.test.wfl`. One existing
  pre-response-line disconnect test took several minutes to finish its shutdown
  on this run; it completed without intervention. The same test passed promptly
  in the full workspace run. No timeout was suppressed or test changed.
- The first integration-runner launch rejected this host's conflicting
  inherited `Path` and `PATH` entries before tests. A fresh child environment
  with one combined Path allowed the existing runner to execute; it reproduced
  the same fixer regression. No repository environment or runner policy was
  changed.

An independent source review by the separate runtime-capabilities agent
reported no blocking findings. It checked policy isolation, default behavior,
repeated headers, budgets and stream cleanup, contextual parsing, and the
recursive fixer's name protection. The implementation author ran the checks
above. This technical review does not constitute maintainer approval, and
remote CI verification remains the parent change's responsibility.
