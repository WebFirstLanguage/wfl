# 2026-10-09 — Integration-suite concurrent listen-0 leftovers (issue #693)

Four concurrent-loop files leave `free_tcp_port` and publish the bound
socket after `listen on port 0`, matching the suite helper from #771.

- `concurrent_stream_ownership_test.rs`
- `concurrent_recursion_depth_test.rs`
- `concurrent_module_loading_test.rs`
- `concurrent_execute_capture_test.rs`

Readiness is `wait_for_published_web_server`, not a bare TCP connect.
No tight elapsed-time assertions in this set.

Skipped: `concurrent_main_loop_test` (300ms interleaving bounds) and
`concurrent_timeout_eval_lock_test` (`elapsed < 1000ms`).
