# 2026-07-29 — outbound POSTs died on reused keep-alive connections

## Symptom

A WFL app proxying an SSE chat endpoint (Rin) failed roughly 3 runs in 100 of
its end-to-end suite, and only under CI-style concurrency — a single container
alone passed 20/20. The app reported:

```
anthropic proxy (http) ended early: Failed to send HTTP POST request:
        error sending request for url (http://127.0.0.1:18999/v1/messages)
```

The mock upstream recorded every request it received, and it had recorded
nothing: the server never saw the POST. The API key was read correctly and the
proxy did try to send. The outbound POST simply never left.

The controlled experiment that identified it (4 parallel containers × 25 runs,
one variable changed, WFL 26.7.57):

| Variant | Runs | Failures |
|---|---|---|
| stock mock (Node's default 5s `keepAliveTimeout`) | 100 | 3 |
| mock with `keepAliveTimeout = 120000` | 100 | 0 |

The failing step sat between two upstream calls, producing local replies for
several seconds with no upstream traffic. Under load the idle gap crossed 5
seconds, Node closed the socket, and WFL's next POST went out on a dead one.

## Root cause

Two independent gaps in `src/interpreter/mod.rs`, both in the shared
`IoClient`:

**1. The connection pool was never configured.**

```rust
reqwest::Client::builder().build()   // no pool tuning at all
```

Nothing set `pool_idle_timeout`, so reqwest's default 90-second idle window
applied. Peers are far less patient — Node closes idle keep-alive sockets at 5
seconds, most proxies between 5 and 15 — so WFL would happily hand out a socket
the peer had abandoned 85 seconds ago.

**2. Neither send site could recover.**

```rust
request.send().await.map_err(|e| {
    HttpClientError::Request(format!("Failed to send HTTP {method} request: {e}"))
})
```

`open_http_stream` (backing `stream response`) and `send_http_request` (the
buffered `read response`/`read content` path) both turned a send failure
straight into a runtime error. hyper will not silently replay a non-idempotent
request — that is not its call to make — so a POST written onto an
already-closed socket failed permanently instead of reconnecting.

The concurrency dependence was a red herring about the mechanism: contention
only widened the idle gap. The defect needs nothing but an idle period, which is
why it is production-relevant — the same client talks to `api.anthropic.com`,
where a first chat message after an idle spell could 500 rather than reconnect.

## Fix

`pool_idle_timeout` is now 3 seconds (`HTTP_POOL_IDLE_TIMEOUT_SECONDS`), well
under the common 5-second floor. hyper checks expiry when it takes a connection
*out* of the pool, so this is not merely a background sweep: a connection idle
longer than that is never handed out, whatever the runtime is doing. Reuse still
covers back-to-back requests, where the handshake saving actually is.

That narrows the race but cannot close it — the peer can always close between
the pool's check and the write — so both send sites now go through
`IoClient::send_with_reuse_retry`, which re-sends once on a fresh connection
when the connection was lost before any of the response arrived *and* repeating
the request is safe. The retry is bounded at one attempt (a peer that is genuinely
gone must surface promptly, not be replayed in a loop), it never fires once a
response head is in hand (a 500 is the program's to handle), it declines to retry
a body that cannot be cloned, and it runs inside the existing timeout/budget
wrapper so it cannot extend a request past its deadline.

### What counts as a lost connection

The first cut keyed on `reqwest::Error::is_request()`, and review caught that
this is too broad: it is the general "the request phase failed" bucket, so it
also replayed a POST that a peer had answered with an unparseable reply — a peer
that demonstrably handled the request — and burned a pointless second attempt on
a refused connection. The predicate is the entire safety argument for re-sending
something that may not be idempotent, so it now tests the specific evidence.

Rather than guess at the error shapes, they were measured against a raw TCP peer:

| Failure | Classification |
|---|---|
| keep-alive socket closed with no response | `hyper::Error::is_incomplete_message` |
| non-HTTP reply | `hyper::Error::is_parse` |
| nothing listening | `is_connect`, io `ConnectionRefused` |

`connection_was_lost` walks the cause chain for the first of those, plus an I/O
error whose kind says the socket was reset, aborted, broken, or read to an
unexpected end (the same event as some platforms surface it). Everything else is
reported to the program as it stands. This needs `hyper` as a direct dependency
to downcast; it is already in the tree via reqwest and resolves to the same 1.x
instance.

### The retry future has to be boxed

CI caught a second-order cost on Windows: `execute_file_test`'s depth guard —
a WFL file that executes itself — died with `STATUS_STACK_OVERFLOW`, on a test
that had nothing to do with HTTP.

The mechanism is that an inline `async fn` becomes part of the state machine of
everything that awaits it. Holding a request builder, its replay clone, and an
in-flight `send()` inline embedded all of that in every statement that can make
a request, and an `execute file` chain nests one such state machine per level.
Measured on Linux by shrinking the test thread's stack with `RUST_MIN_STACK`,
running the same depth-guard test:

| Build | 2048 KiB | 1920 KiB | 1792 KiB |
|---|---|---|---|
| before this change | ok | ok | overflow |
| inline retry future | ok | **overflow** | overflow |
| boxed retry future | ok | ok | overflow |

So the inline form cost about 150 KiB of headroom on that recursion, and Windows
test threads sit just about there. `Box::pin` puts the retry's state on the heap
and restores the previous threshold exactly. One allocation per outbound request
is not measurable next to a network round trip.

Worth remembering generally: the stack cost of an `async fn` is paid by every
caller that awaits it inline, and recursion multiplies it.

### A lost connection is not enough on its own

Review then pushed back a second time, and rightly: a lost connection says the
*caller* saw no response head, not that the upstream saw nothing. A connection
can also be lost after a server read and committed the body, and nothing on this
side distinguishes that from a socket the peer abandoned before the request
arrived. The tests' own upstream reads a dropped request in full before closing,
which is that ambiguity in miniature — so the narrowed predicate alone would
still let WFL submit the same charge twice.

`request_may_be_replayed` is the second gate. A re-send now needs the connection
to have been lost *and* the request to carry a promise it can be repeated:

* an idempotent method per RFC 9110 §9.2.2 (`GET`, `HEAD`, `PUT`, `DELETE`,
  `OPTIONS`, `TRACE`), or
* an `Idempotency-Key` / `X-Idempotency-Key` header — the caller stating the
  upstream deduplicates repeats of this exact request.

A bare `POST` or `PATCH` gets no re-send; its protection is the 3-second pool
window, which is what made these failures routine in the first place. Building the
replay clone up front doubles as the check that the body can be reproduced, so a
streaming body drops out of eligibility there too.

## Testing

R3 (lifecycle, streaming, concurrency). Red first:
`tests/http_connection_reuse_retry_test.rs`, commit `545e4c4`, 3 of 4 failing —
two of them with the reported message verbatim, `Failed to send HTTP POST
request: error sending request for url (...)`, reproduced inside WFL with no
reference to the reporting app.

The upstream in those tests is a raw TCP server rather than a timer, so the
"peer closed the keep-alive socket" moment is deterministic: it answers normally
except for the request numbers it is told to drop, where it closes the socket
with no response — precisely what a client sees when it writes to a socket the
peer had already decided to close. Coverage:

* buffered `read response` recovers for a keyed POST, and the retry lands on a
  fresh connection;
* `stream response` recovers for a keyed POST (the path the reported failure
  took);
* an idempotent `GET` through `read content` recovers with no key at all;
* an upstream that closes *every* request is attempted exactly twice and then
  raises — the retry is bounded, not a loop;
* an answered request is sent exactly once — no replay behind the program's
  back;
* a peer that replies with something unparseable is not replayed either, even
  with a key permitting it: the peer handled the request, so the failure is real
  but the delivery must stay single;
* the negative half of the replay contract: an unkeyed `POST` against an upstream
  that reads it in full and then closes reaches the server **exactly once** and
  surfaces the failure, rather than being silently submitted twice;
* the reported real-world shape: a peer whose keep-alive window is shorter than
  the gap the program leaves between calls still yields a reply. The gap is
  longer than our own pool window, since an unkeyed POST has to survive this
  case without any re-send — it pins the pool defence specifically.

Red for the added replay contract is the test commit preceding the change: with
the old predicate, `an_unkeyed_post_is_never_replayed` observed 2 deliveries
where it requires 1.

Stability, since the defect this replaces was itself intermittent: 90 runs green
(30 sequential + 60 across 6 concurrent copies), 0 failures.

## Residual risk

An unkeyed `POST` onto a socket the peer closed inside the 3-second window still
surfaces as a hard error, because a lost connection is not decidable from the
client: it cannot be told apart from a delivered request whose response was lost.
That is the deliberate trade — a spurious, catchable failure beats a duplicated
charge — and it is narrower than it sounds, since the pool window already removes
the case that made this routine. Callers who want that last case recovered opt in
with an idempotency key, the same contract the upstream APIs (Stripe and friends)
already ask for.

The key is taken at the caller's word: WFL cannot verify the upstream actually
deduplicates on it. A program that sends the header to a service which ignores it
gets at-least-once delivery for that request, which is documented alongside the
feature in `Docs/04-advanced-features/interoperability.md`.
