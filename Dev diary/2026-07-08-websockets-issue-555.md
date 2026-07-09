# Dev Diary — 2026-07-08 — WebSocket support (issue #555)

## Summary

Implemented event-handler WebSocket support in the interpreter, one of the four
aspirational features tracked in issue #555. WFL programs can now run a real
WebSocket server with natural-language syntax:

```wfl
listen for websockets on port 8080 as chat_server

on websocket connect to chat_server as connection:
    send websocket message "Welcome!" to connection
end on

on websocket message from chat_server as incoming:
    broadcast websocket message body of incoming to chat_server
end on

on websocket disconnect from chat_server as connection:
    display "left: " with id of connection
end on

wait for 3600 seconds
close server chat_server
```

## Design decision: event-handler model on a pull-based architecture

The aspirational `TestPrograms/web_server_websocket_test.wfl` was written against
an **event-handler** model (`on websocket connect ... end on`). The existing HTTP
server, by contrast, is **pull-based**: warp runs in background tokio tasks and
pushes requests over an `mpsc` channel, and WFL code calls `wait for request` to
pull one. That design exists because the interpreter is single-threaded (`Rc` /
`RefCell`, `!Send`) — warp tasks cannot call back into it.

To honour the event-handler syntax without breaking that invariant, events are
**pumped into the interpreter while the program sits in a `wait for <duration>`**.
warp's per-connection tasks send `connect` / `message` / `disconnect` events over
a channel; during a wait, the interpreter drains that channel and runs the
matching registered handler block via `execute_block`. Everything stays on one
thread. A real server keeps itself alive (and dispatching) with a long wait or a
main loop.

## What was added

- **Lexer/parser:** no new keywords (kept `message`, which is a common variable
  name, as an identifier). New statements: `listen for websockets ...`,
  `on websocket connect|message|disconnect to|from <server> as <bind>: ... end on`,
  `send websocket message <msg> to <target>`, `broadcast websocket message <msg>
  to <server>`. `close server` now also closes a WebSocket server.
- **AST:** `ListenWebSocketStatement`, `WebSocketHandlerStatement`,
  `SendWebSocketMessageStatement`, `BroadcastWebSocketMessageStatement`, plus a
  `WsHandlerEvent` enum.
- **Interpreter:** `WflWebSocketServer` (event receiver + connection registry +
  handler set), a `handle_ws_connection` task (splits the socket, forwards
  inbound frames as events, drains a per-connection outbound channel), the event
  pump hooked into `WaitForDurationStatement`, and handler dispatch that binds a
  connection/message object.
- **Analyzer/typechecker/transpiler:** binding + property-name scoping so
  `id of conn` / `body of msg` resolve; the JS transpiler rejects WS statements
  (interpreter-only, like databases).
- **Dependencies:** `futures-util` (to split warp's WS stream/sink);
  `tokio-tungstenite` as a dev-dependency for the client test.

## Two problems worth recording

1. **Merged identifiers vs. the message operand.** The lexer merges adjacent
   identifiers into one token, so `send websocket message body of msg` lexes the
   command words *and* the leading `body` into a single identifier, stranding
   `of msg`. WFL's own convention is that command words are keywords (which don't
   merge), but `message` is too common a variable name to reserve. The parser
   therefore strips the known command prefix and parses the remaining operand
   robustly: a leading string literal parses as a full expression; `<field> of
   <object>` is reconstructed as a property read; a bare (multi-word) variable is
   taken as-is; anything more complex asks the user to store it in a variable
   first.

2. **`property of object` did not actually work at runtime.** `body of request`
   appears throughout the (CI-skipped) HTTP demos, but `X of Y` parses as a call
   `X(Y)` and the interpreter had no object-field fallback — so it errored with
   "undefined variable". Added a narrow fallback in `FunctionCall` evaluation:
   when the callee is a bare name that is not a real function and the single
   argument is an object carrying that key, return the field. This makes
   `body of msg`, `id of conn`, and the existing `method of request` /
   `body of request` forms all read as property access.

## Tests

- `tests/websocket_test.rs`: end-to-end with a real `tokio-tungstenite` client —
  connect + greet + echo, and broadcast to multiple clients.
- `TestPrograms/websocket_echo_server.wfl`: CI-safe (self-contained, no external
  client) coverage of parse → analyze → listen → register → dispatch → close.

## Not done (still tracked in #555)

The original `web_server_websocket_test.wfl` remains CI-skipped: beyond the core
handled here, it also exercises comma-separated `create list with a, b, c`,
`get websocket stats`, `connect websocket client` (server-as-its-own-client
simulation), and connection-limit enforcement — each a separate concern the issue
itself scopes as its own work. The core real-time messaging feature is complete
and verified end-to-end.
