# Inspecting original HTTP responses

Authentication tests need to observe a login endpoint's original status,
Location, and cookie headers. The HTTP client previously followed redirects
unconditionally, hiding that response behind the destination's response.

The existing `open url` form now accepts `and without following redirects`
before its content, response, or stream terminator. Default requests still
follow redirects. The marker is contextual and reserves no new words. This
keeps the same request vocabulary as users move from a basic fetch to explicit
authentication and streaming workflows, in line with the No-Unlearning
Invariant.

Full buffered and streaming responses also expose `header_values`, a map of
lowercase header names to lists of text values. Existing scalar `headers`
access stays compatible. Multiple Set-Cookie fields are retained separately
instead of joining values into an invalid cookie field.

Two lazily initialized connection pools isolate redirect policies without
mutating shared per-client settings. Both policies use the same existing
request retry rules, execution budgets, bounded body readers, streaming
lifetime, and cleanup paths.

The full source-fixer regression corpus caught a variable/marker spelling
collision. The fixer's conservative name protection now covers this contextual
clause as well, and a WFL CLI regression verifies that ordinary lint fixes
preserve the request while still fixing an unrelated local name.

Every new scenario, fixture, assertion, and driver is WFL. A WFL loopback peer
uses an ephemeral port, and its owning WFL test stops the process in `finally`.
The existing recursive integration runners discover the test suite directly.
The Red test-only ancestor `66034730` reproduced a `200` where the original
redirect's `302` was required, using syntax valid before this implementation.

The [evidence record](../../../Engineering/evidence/2026-09-20-http-response-controls.md)
contains validation results and review limits. The
[HTTP client guide](../../../Docs/04-advanced-features/interoperability.md#inspecting-a-redirect-response)
documents the additive API and links the executable regression.
