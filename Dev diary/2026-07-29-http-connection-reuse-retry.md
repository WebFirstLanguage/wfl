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
when `reqwest::Error::is_request()` holds. That predicate means no response head
arrived, so the caller observed nothing and a re-send cannot duplicate anything
it could have acted on. The retry is bounded at one attempt (a peer that is
genuinely gone must surface promptly, not be replayed in a loop), it never fires
once a response head is in hand (a 500 is the program's to handle), it declines
to retry a body that cannot be cloned, and it runs inside the existing
timeout/budget wrapper so it cannot extend a request past its deadline.

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

* buffered `read response` recovers, and the retry lands on a fresh connection;
* `stream response` recovers (the path the reported failure took);
* an upstream that closes *every* request is attempted exactly twice and then
  raises — the retry is bounded, not a loop;
* an answered request is sent exactly once — no replay behind the program's
  back;
* the reported real-world shape: a peer whose keep-alive window is shorter than
  the gap the program leaves between calls still yields a reply. This one is
  time-based, so it deliberately does not pin *which* recovery ran — the pool
  may drop the expired socket, or hand it over and let the re-send fix it.

Stability, since the defect this replaces was itself intermittent: 90 runs green
(30 sequential + 60 across 6 concurrent copies), 0 failures.

## Residual risk

`is_request()` also covers a request that was fully written and then lost
without a response — a server that dies mid-request rather than an idle socket.
Re-sending there could reach a server that had already acted on the first copy.
The pool timeout makes the stale-socket case, which is the common one, rare
enough that the retry is mostly a backstop; the alternative (leave it) means
programs keep seeing spurious hard failures for requests no server ever saw. If
a caller ever needs "never re-send under any circumstance", that belongs in an
explicit per-request opt-out rather than in the default.
