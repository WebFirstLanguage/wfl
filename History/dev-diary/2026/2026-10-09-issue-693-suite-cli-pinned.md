# 2026-10-09 — Integration-suite pinned names, CLI bound, rustyline (issue #693)

Eleventh batch: the non-Group-A leftovers move into `tests/suite/`.

Pinned `--test` names now use the suite filter form in the same change:
`cargo test --test suite --verbose -- split_functionality::` and
`cargo test --test suite --verbose -- database_test::` (cargo `--verbose`
before `--`; the YAML `run` value is quoted so `::` is not a mapping
indicator). Updated CI (`.github/workflows/ci.yml`), both integration
scripts, `Docs/04-advanced-features/databases.md`, and the rustyline
helper path in `CLAUDE.md`. Historical `History/` /
`Engineering/evidence/` / `Archive/` records are left as-is.

`split_functionality` now drives `crate::common::wfl_exe()` (the cargo
bin) so `cargo test --test suite` does not require a prior
`cargo build --release`.

`database_transaction_cli_test` stays in the suite: the 15s process
bound had ~14.8s headroom under 4 concurrent copies plus a 64-thread
web-module load (max 192ms / 32 runs).

`config_command_test` and `init_command_test` set `TERM=dumb` only on
the child `Command` (no process-global rustyline/TERM state).

## Remaining top-level (33) — design call for Brad

Group A last, as planned. `password_policy_lifecycle_test` is the one
file that *must* stay a separate binary if the others ever fold in: it
pins a process-wide Tokio blocking-pool limit.

Concurrent / §11.3 (real sockets, shared interpreter, races):
`concurrent_disconnect_burst_test`,
`concurrent_disconnect_paths_burst_test`,
`concurrent_execute_capture_test`, `concurrent_main_loop_test`,
`concurrent_module_loading_test`, `concurrent_prehead_prune_race_test`,
`concurrent_recursion_depth_test`, `concurrent_stream_ownership_test`,
`concurrent_timeout_eval_lock_test`.

Stream-named siblings (keep with Group A so they stay one binary
family): `http_server_streaming_test`, `http_stream_paced_test`,
`http_stream_test`, `outbound_stream_absolute_lifetime_test`,
`outbound_stream_deadline_test`, `outbound_stream_disconnect_test`,
`outbound_stream_head_disconnect_test`,
`outbound_stream_open_expiry_test`, `outbound_stream_ownership_test`,
`outbound_stream_reaper_race_test`,
`response_expression_disconnect_runtime_test`,
`response_stream_backpressure_test`, `stream_handle_type_test`,
`typechecker_response_stream_join_test`,
`typechecker_response_stream_scope_test`,
`wait_line_pre_response_disconnect_test`.

Lifecycle / admission / transport (R3, often ports or drop/cleanup):
`dropped_interpret_cleanup_test`,
`dropped_interpret_server_cleanup_test`,
`file_io_lifecycle_cli_test`, `tls_handshake_boundary_test`,
`web_admission_reopens_after_timeout_test`, `web_queue_bound_test`,
`websocket_test`.

Isolation (must stay a separate executable):
`password_policy_lifecycle_test` — process-wide hashing admission and
`max_blocking_threads(1)`.
