# Outbound HTTP Client (Method/Headers/Body) and Standard Crypto (Issue #558)

**Date:** 2026-07-03

## What Changed

Issue #558 reported that a pure-WFL service cannot integrate with real
third-party APIs (the motivating example is Stripe): the outbound HTTP
client was GET-only with no way to set the method, headers, or a request
body — and there was no standard SHA-256 or HMAC-SHA256 for verifying
webhook signatures (WFLHASH cannot interoperate with what Stripe/GitHub
sign).

### 1. Extended `open url` statement

The existing forms keep working unchanged (they still parse to
`HttpGetStatement`):

```wfl
open url at "https://example.com" and read content as response
open url at "https://example.com" as response
```

New optional clauses, joined by `and`/`with` in any order, parse to the new
`HttpRequestStatement`:

```wfl
open url at "https://api.stripe.com/v1/checkout/sessions"
    with method "POST"
    and headers request_headers
    and body "mode=payment&line_items[0][price]=price_123"
    and read response as resp
```

- `method` — any HTTP method as text (default `GET`), validated at runtime.
- `headers` — a map (`Value::Object`) of header name → value.
- `body` — text request body; `with` concatenation works inside the clause
  (`body "amount=" with amount`), with lookahead so a `with`/`and` that
  introduces the next clause terminates the value instead.
- `read response as` binds a full response object: `status` (Number), `ok`
  (Boolean, 2xx), `body` (Text), `headers` (map with lowercase names).
  `read content as` keeps binding just the body text. Non-2xx statuses are
  data, not runtime errors; network failures still raise catchable errors.

Implementation notes:

- The lexer merges consecutive identifiers into multi-word names, so
  `and headers auth_headers` arrives as the single token
  `Identifier("headers auth_headers")`. The clause parser matches both the
  bare keyword and the merged form and splits off the remainder as the
  variable reference.
- Clauses may continue on following lines; end-of-line tokens are skipped
  only when a `and`/`with` connector follows, so a statement that ends
  without its `read`/`as` terminator still errors.
- `IoClient::http_request` drives reqwest with
  `Method::from_bytes`/headers/body and returns
  `(status, headers, body_text)`.

### 2. Standard crypto builtins

- `sha256 of <text>` — FIPS 180-4 SHA-256, lowercase hex (64 chars).
- `hmac_sha256 of <message> and <key>` — RFC 2104 HMAC-SHA256, lowercase
  hex. This is the webhook-verification primitive for Stripe/GitHub/Slack.

The `hmac` crate was already a transitive dependency via `hkdf`, so this
adds no new external code. Registered in `stdlib/crypto.rs`, `builtins.rs`
(names + arity), `typechecker/mod.rs` (return type), and
`stdlib/typechecker.rs` (parameter types).

### 3. Supporting fixes found along the way

- **`create map` was unusable multiline**: `parse_map_creation` never
  skipped end-of-line tokens, so any map with entries on their own lines
  failed to parse. Fixed; also added string-literal keys
  (`"Content-Type" is "..."`) since HTTP header names aren't valid
  identifiers, and registered the created variable in the analyzer (it was
  previously reported as undefined at every use site).
- **Property access on objects**: `Expression::PropertyAccess` only worked
  on container instances; it now also reads `Value::Object` keys, so
  `resp.body`, `resp.ok`, `resp.headers` work. The dot-property parser
  additionally accepts the `status` keyword as a property name so
  `resp.status` parses (it lexes as `KeywordStatus`, not an identifier).
  The type checker allows property access on `Map`/`Unknown`/`Any` types
  instead of erroring.

## Tests

- `tests/sha256_hmac_test.rs` — 9 tests against standard vectors (empty
  string, "abc", RFC-style HMAC vectors, Stripe-style signed payload).
- `tests/http_request_parser_test.rs` — 12 tests: backward compatibility of
  both GET forms, clause combinations, merged-identifier handling,
  multiline statements, duplicate-clause and missing-terminator errors.
- `tests/http_request_runtime_test.rs` — 6 tests against a local one-shot
  TCP server (offline-safe): method/headers/body arrive on the wire,
  response object fields and dot access, 404 handled as data, invalid
  method errors.
- `TestPrograms/crypto_standard_test.wfl` — end-to-end vectors plus the
  webhook signature verification pattern.

## Follow-ups / Not Done

- The issue's minor note about `open file at <action result>` type-checker
  warnings is a separate inference gap (same family as #551/#553) and was
  not addressed here.
- No timeout clause yet; requests use reqwest defaults.
- Binary request/response bodies are out of scope (text only).
