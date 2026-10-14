# 2026-10-09 — Integration-suite remaining listen-0 leftovers (issue #693)

Six leftover files drop `free_tcp_port` / fixed port 8241 and publish the
bound socket after `listen on port 0`.

- `concurrent_disconnect_burst_test.rs`
- `concurrent_disconnect_paths_burst_test.rs`
- `dropped_interpret_server_cleanup_test.rs`
- `http_server_streaming_test.rs`
- `response_expression_disconnect_runtime_test.rs`
- `web_admission_reopens_after_timeout_test.rs` (was port 8241)

Readiness is `wait_for_published_web_server`, not a bare TCP connect or a
fixed sleep. No tight elapsed-time assertions in this set.

Still separate: `password_policy_lifecycle_test` (process-wide hashing
pool), `concurrent_main_loop_test` (300ms interleaving bounds),
`concurrent_timeout_eval_lock_test` (`elapsed < 1000ms`),
`http_stream_paced_test`, `outbound_stream_deadline_test`,
`outbound_stream_open_expiry_test`, `outbound_stream_reaper_race_test`,
and `response_stream_backpressure_test` (tight wall-clock proofs).
