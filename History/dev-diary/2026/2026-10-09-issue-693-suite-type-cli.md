# 2026-10-09 — Integration-suite type/config/CLI leftovers (issue #693)

Twelfth batch: six leftover files with no process-wide state and no live
port bind move into `tests/suite/`.

- `typechecker_response_stream_join_test.rs` / `_scope_test.rs` /
  `stream_handle_type_test.rs` — type-check only; `listen on port 8080`
  is source text, never a bind.
- `tls_handshake_boundary_test.rs` — rustls record-parser unit tests
  (one real-socket TLS pair on `:0`).
- `web_queue_bound_test.rs` — config load + in-memory mpsc/semaphore
  shed; no listener.
- `file_io_lifecycle_cli_test.rs` — real CLI child, `WFL_GLOBAL_CONFIG_PATH`
  on the child only, 30s hang detector. Rewritten to `crate::common::wfl_exe()`.

None of these names are pinned as `--test` filters in CI or scripts.

Remaining top-level files still need a per-file process-wide / timing /
`free_tcp_port` decision before they move.
