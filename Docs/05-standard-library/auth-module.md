# Managed authentication

WFL's authentication helpers provide a bounded session store, session rotation
and revocation, a request CSRF guard, secure session cookies, and an account
attempt limiter. They are built in; no import is needed. Password verification
uses the [crypto module](crypto-module.md#password-hashing).

Invoke these helpers with `of`, as in `session_cookie of token`, or with an
explicit call, `call session_cookie with token`. Bare `session_cookie with
suffix` keeps the ordinary concatenation grammar for existing variables.
Existing variables, constants, and user actions may use these helper names;
their declarations take precedence over the default helpers in that scope.

Create each store once before your request loop and pass its handle to actions
that need it. Handles share state across concurrent handlers and isolated
modules. They cannot be constructed from text, inspected as objects, or
serialized. Dropping the last handle releases its state.

Set `execution_logging = false` when handling real credentials. Passwords and
issued tokens are ordinary WFL text values; execution traces and diagnostic
snapshots can contain them. Opaque store handles hide their contents, but WFL
does not automatically redact secret text returned to your program.

**Storage lifetime:** these stores live in one running program's memory.
Restarting the program logs out all sessions and resets attempt counters.
Separate processes have separate state. Applications requiring shared or
durable state must use a shared backend; these helpers do not provide one.
Retain proxy-level request limits, especially when running multiple workers.

## Session API

| Function | Arguments | Result |
|---|---|---|
| `create_session_store` | absolute lifetime seconds, idle seconds, capacity | Opaque store |
| `session_create` | store, account key | Record with `id`, `csrf_token`, `account` |
| `session_lookup` | store, session ID | Account text, or `nothing` |
| `session_rotate` | store, session ID | New session record, or `nothing` |
| `session_revoke` | store, session ID | `yes` if a live session was removed |
| `session_revoke_account` | store, account key | Number of live sessions removed |
| `session_cookie` | session ID | Complete `Set-Cookie` value |
| `session_csrf_guard` | store, request object | `yes` only if the request passes |

Lifetime values are whole seconds from 1 to 2,592,000 (30 days). Idle lifetime
cannot exceed absolute lifetime. Capacity is a whole number from 1 to 100,000.
Account keys contain 1 to 1,024 UTF-8 bytes. Invalid policies, types, arities or
account keys raise an error; a full session store raises an error without
evicting live sessions. Expired entries are reclaimed on subsequent operations.

Each issued session has independent 256-bit random ID and CSRF tokens, encoded
as 64 lowercase hexadecimal characters. The store retains their SHA-256
digests. Tokens presented by a client never create sessions. Unknown, malformed,
revoked and expired IDs return `nothing` on lookup or rotation.

Successful lookups and CSRF checks refresh the idle deadline. Rejected requests
do not refresh it. Absolute lifetime is measured from initial issuance using a
monotonic clock; neither activity nor rotation extends it. Rotation replaces
both tokens and invalidates the previous session immediately, with no overlap.
Only one simultaneous rotation of an old ID can succeed. If random generation
fails, the previous session remains valid until its existing deadline.

Issue a session only after verifying credentials. Rotate after reauthentication
or privilege changes, and revoke all account sessions after password resets or
account disablement. Rotation retains the account key. Authorization remains
the application's responsibility: look up current account permissions when
handling protected operations.

```wfl
store sessions as create_session_store of 3600 and 900 and 10000
store attempts as create_account_rate_limiter of 5 and 60 and 10000
store allowed as account_rate_limit_allow of attempts and "account-42"
check if allowed:
    // Issue only after the application's credential check succeeds.
    store issued as session_create of sessions and "account-42"
    store cookie as session_cookie of issued["id"]
    check if contains of cookie and "Secure; HttpOnly; SameSite=Strict":
        display "Session cookie prepared"
    end check
    store rotated as session_rotate of sessions and issued["id"]
    store account as session_lookup of sessions and rotated["id"]
    check if account is equal to "account-42":
        store revoked as session_revoke_account of sessions and account
        display "Sessions revoked: " with revoked
    end check
end check
```

The executable example is
[`managed_auth.wfl`](../../TestPrograms/docs_examples/stdlib_functions/managed_auth.wfl).

## CSRF guard in the request loop

Call `session_csrf_guard` before any protected handler performs work. It is an
explicit middleware predicate: your request loop sends a denial response when
it returns `no`; WFL does not install a global guard automatically.

The guard reads only the `__Host-wfl_session` cookie and the `X-CSRF-Token`
header. It requires a valid session for every method. `GET`, `HEAD` and
`OPTIONS` do not require a CSRF token; all other methods require the token
issued with that particular session. Unsafe operations must use an unsafe
method. Token comparison uses constant-time digest equality.

Missing, wrong, expired or revoked credentials produce `no`. Duplicate session
cookies, repeated physical Cookie/CSRF headers, invalid header encoding, cookie
headers exceeding 8,192 bytes, and malformed tokens are rejected. The request's
`ambiguous_auth_headers` flag carries the transport's duplicate/encoding
decision to the guard. Pass the original request object to preserve it.
`execute file ... with req` also forwards this boolean to the executed page;
include it when reconstructing request context there. Older caller-created
contexts without the flag receive `no`, while a present nonboolean flag is
rejected before the page runs.

```wfl
// CI-SKIP: request-loop fragment; requires a listener and authenticated client
main loop concurrently:
    wait for request comes in on server as req
    store permitted as session_csrf_guard of sessions and req
    check if permitted:
        respond to req with "Protected handler"
    otherwise:
        respond to req with "Forbidden" and status 403
    end check
end loop
```

`session_cookie` returns
`__Host-wfl_session=<id>; Path=/; Secure; HttpOnly; SameSite=Strict`. Send it as
`Set-Cookie` over HTTPS. Deliver the matching CSRF token in the authenticated
page or response body for same-origin code to place in `X-CSRF-Token`; never
put it in a URL. Send authentication responses with `Cache-Control: no-store`.
After revoking at logout, clear the browser cookie with
`__Host-wfl_session=; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=0`.
These browser protections complement server-side validation.

## Account attempt limits

`create_account_rate_limiter of attempts and seconds and capacity` creates a
fixed-window limiter. Attempts and capacity are whole numbers from 1 to
100,000; window seconds range from 1 to 2,592,000. Call
`account_rate_limit_allow of limiter and account_key` **before password
verification** for every attempt. It consumes one attempt and returns `yes`, or
returns `no` when the account's allowance is exhausted. Successful logins also
consume attempts; there is no automatic reset on success.

The first admitted attempt starts the account's window. Rejected attempts do
not prolong it. At expiry the key's allowance resets. A full limiter denies
new keys without evicting existing keys, so changing usernames cannot clear a
blocked account. Use the same canonical account key as your account lookup,
including consistent case/Unicode normalization. Apply the policy to nonexistent
accounts too, and return a uniform login failure response. The limiter does
not normalize identities or perform account discovery.

Fixed windows can admit twice the allowance around a boundary. Account limits
can also deny legitimate users targeted by repeated attempts. Choose policy
values for your workload and combine them with proxy request limits and account
recovery. For validated IP identity, see
[trusted proxies](../04-advanced-features/web-servers.md).

The design uses the server-side expiry/rotation and synchronizer-token patterns
described in [OWASP session management](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)
and [CSRF prevention](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html).
