# HTTP response controls evidence

Risk: R3 (HTTP protocol behavior, response limits, streaming and compatibility).
Base: `cb1dadaad96939a4450a6eb2b3a6a51678035b7f`.

## Acceptance criteria

- Existing requests follow redirects without source changes.
- The contextual phrase `without following redirects` returns the original
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
on a syntax error. The Green regression will select the new no-follow option
explicitly, while a separate compatibility test retains the default following
behavior. The test's `finally` stops its fixture process on assertion failure.

The Red test-only commit is retained as an ancestor of the implementation.
Green commands, results and commit identification will be recorded after the
implementation is tested. No passing result is claimed here.
