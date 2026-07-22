# Dev Diary — 2026-07-22 — Streamed server responses

## Context

Follow-on to the same-day outbound response streaming work. This adds the
**server** half (item 3 of the five-capability request): a WFL handler can now
send a response whose body is produced progressively — status/headers first,
then body pieces — which is what a browser chat UI needs to read newline-
delimited JSON off a `fetch()` response as it arrives.

## What shipped

```wfl
start streaming response to req with status 200 and content type "application/x-ndjson" as out
write line json_text to out    // frames a line (newline appended)
write chunk raw_bytes to out    // raw bytes/text, verbatim
flush out                       // advisory
close out                       // ends the response body
```

`start streaming response` returns immediately after sending the head and binds
a stream handle (`{ _server_stream, status }`). Combined with the client-side
`stream response as upstream` + `wait for next line`, a handler can proxy a slow
upstream to the browser line-by-line without buffering either side.

## Design & mechanism

- **Reply payload is now an enum.** The per-request `oneshot` carries a
  `HandlerReply` = `Buffered(WflHttpResponse)` | `Streaming { status,
  content_type, headers, body: mpsc::Receiver<Vec<u8>> }`. `respond` sends
  `Buffered`; `start streaming response` sends `Streaming` with the receiving end
  of a bounded body channel.
- **Transport reply became `Body`-typed.** The warp route's final closure now
  returns `Response<warp::hyper::Body>`. Key simplification: warp's `.recover()`
  unifies reply types via `Either`, so only that one closure changed — the five
  `Response<Vec<u8>>` helper functions and `handle_overloaded` were left alone;
  the closure wraps their returns with `.map(Body::from)`. The streaming arm
  builds `Body::wrap_stream(futures_util::stream::unfold(rx, ...))` — no new
  dependency.
- **Backpressure & disconnect.** The body channel is bounded
  (`RESPONSE_STREAM_BUFFER = 64`), so a slow client backpressures the handler's
  `write` (it awaits a free slot). When the client disconnects, hyper drops the
  body, dropping the receiver; the handler's next `write` then fails with a
  catchable error — that is how a browser disconnect propagates to the handler
  (which can then `close` the upstream it is proxying). `close` (or handler exit)
  drops the sender, ending the response.
- **`start` is a keyword, `streaming`/`flush`/`line`/`chunk` are identifiers.**
  `start streaming response` dispatches on `Token::KeywordStart`; `flush <out>`
  and `write line|chunk <value> to <out>` handle the lexer's identifier-merging
  (`flush out`, `line payload`) the same way the websocket-message statements do.
- **`close` unified.** `close <x>` now closes a file (`Text`), a client upstream
  (`_stream`), or a server response stream (`_server_stream`).

## Files

- Types + transport + exec: `src/interpreter/mod.rs` (`HandlerReply`,
  `Body`-typed closure, `server_response_streams` map, three statement arms,
  `close` extension).
- AST: `StartStreamingResponseStatement`, `StreamWriteStatement`,
  `FlushStreamStatement`.
- Parser: `parse_start_streaming_response`, `parse_flush_stream`
  (`src/parser/stmt/web.rs`), `write line|chunk` branch
  (`src/parser/stmt/io.rs`), `KeywordStart`/`flush` dispatch (`parser/mod.rs`).
- Analyzer/typechecker/transpiler arms.
- Docs: `Docs/04-advanced-features/web-servers.md` ("Streaming a response",
  incl. an upstream-proxy example) + validated example
  `TestPrograms/docs_examples/web_servers/streaming_response.wfl`.

## Tests

`tests/http_server_streaming_test.rs` — parser tests for all four statements,
plus end-to-end runtime tests that stand up a WFL streaming server and read it
back with reqwest: status/headers arrive with `write line` framing
(`alpha\nbeta\ngamma\n`), and `write chunk` is verbatim (`onetwo`). The existing
web-server suites (query/binary/content-length/queue-bound/admission) still pass
after the transport reply-type change; `fmt`, `clippy -D warnings`, and the 618
lib tests are green.

## Still open

- **Item 4 — concurrent handlers** (`main loop concurrently:`), Phase 1 of the
  locked concurrency plan: today a streamed handler runs to completion before the
  next request is served, so per-request isolation between a slow stream and
  other requests still awaits Phase 1.
