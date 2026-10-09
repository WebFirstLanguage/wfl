# 2026-10-09 — Integration-suite leftovers that stay separate (issue #693)

After the listen-0 batches, eight top-level integration binaries remain.
Each now states why it cannot join `tests/suite/`:

- `password_policy_lifecycle_test` — process-wide configured-hashing admission semaphore
- `concurrent_main_loop_test` — 300ms interleaving bound
- `concurrent_timeout_eval_lock_test` — `elapsed < 1000ms`
- `http_stream_paced_test` — minimum parked-read wall-clock
- `outbound_stream_deadline_test` — 500ms–4s deadline window
- `outbound_stream_open_expiry_test` — 700ms–3s absolute-cap window
- `outbound_stream_reaper_race_test` — 700ms–3s reaper window
- `response_stream_backpressure_test` — 1500ms–9s write-timeout window and a <1500ms early-flush bound

Nothing else at `tests/*.rs` is a safe suite move.
