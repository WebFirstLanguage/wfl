# 2026-10-09 — Integration-suite disconnect listen-0 leftovers (issue #693)

Four proxy/disconnect files drop `free_tcp_port` and publish the bound
socket after `listen on port 0`. Mock upstreams already bind `:0`.

- `outbound_stream_disconnect_test.rs`
- `outbound_stream_head_disconnect_test.rs`
- `wait_line_pre_response_disconnect_test.rs`
- `concurrent_prehead_prune_race_test.rs` — `elapsed < 3s` is a hang
  detector against a 4s idle timeout, not a tight bound.
