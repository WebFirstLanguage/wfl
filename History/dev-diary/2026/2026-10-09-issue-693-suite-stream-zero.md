# 2026-10-09 — Integration-suite :0 stream/websocket leftovers (issue #693)

Thirteenth batch: five leftover files that already bind `:0` (mock
upstream or child-announced websocket port) and have no process-wide
state.

- `http_stream_test.rs` — parser + runtime against a local `127.0.0.1:0`
  mock. Rewritten to `crate::common`.
- `websocket_test.rs` — child `wfl` listens on port 0 and announces
  `listening on port`.
- `dropped_interpret_cleanup_test.rs` / `outbound_stream_ownership_test.rs`
  — mock `:0` upstream; hang-detector timeouts only.
- `outbound_stream_absolute_lifetime_test.rs` — 1s absolute lifetime +
  1.5s wait; `elapsed < 5s` is a hang detector, not a tight bound.

Skipped this batch: `http_stream_paced_test` (`MIN_PARKED_MS = 150` under
parallelism) and every file that still uses `free_tcp_port` or a fixed
port.
