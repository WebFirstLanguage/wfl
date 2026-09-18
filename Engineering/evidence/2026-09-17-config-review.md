# PR #731 review verification

## Scope and risk

Risk **R3**: global configuration replacement, CLI compatibility, and resource
limit configuration. Current `main` (`e133ddbd`) was integrated in `6ca3cfd6`.
The review fixes retain the explicitly requested global `wfl config` command
with no directory argument. They add the missing `outbound_stream_max_seconds`
registration and protect existing configuration from incomplete saves.

The documentation index now describes global and project configuration, and
the affected production functions and test helpers have API documentation.
The dated [diary addendum](../../History/dev-diary/2026/2026-09-17-config-command.md)
records the user's explicit scope decisions concerning command compatibility
and the archived terminology correction; those decisions are preserved.

## Red to Green

`0831ebdf` first separated serialization from file writing without changing
behavior; all 13 existing wizard unit tests passed. Test-only commit `8c7b31eb`
then retained these observed failures before implementation:

- Atomic-save component tests: **3 failed**. Partial output was published,
  an existing reader saw truncated/replaced contents, and direct writing
  prevented the test from exercising a final replacement failure.
- Stream-setting checker tests: **3 failed**, because the key was unregistered.
- Stream-value wizard test: **1 failed**, because the unsigned runtime range
  was rejected by signed integer parsing.
- CLI tests: **10 passed, 6 failed**, including missing default, missing prompt,
  confirmed overwrite, and invalid-answer reprompt coverage.

Tests use real temporary files and the actual CLI. The component save tests
inject write/flush failures through a writer that forwards bytes to a real file;
they are not claims of a physical disk-full test. Replacement failure is induced
by creating a destination directory after the temporary contents are written.

Green implementation: **`3e391455`**. Later evidence-only updates do not change
the tested source. Independent read-only review approved this commit, inspected
the configuration unit/CLI result logs, and found no actionable defects.

## Acceptance coverage

| Behavior | Test |
| --- | --- |
| Partial write/flush errors preserve existing bytes or absence; no temporary files remain | `test_config_save_failures_preserve_destination_and_clean_temporary_files` |
| Successful replacement leaves old readers unchanged and publishes complete contents | `test_config_save_replaces_file_without_changing_existing_readers` |
| Failed rename preserves the destination directory and removes temporary output | `test_config_save_cleans_temporary_file_when_replacement_fails` |
| Read-only global file remains unchanged and CLI returns failure | `config_preserves_read_only_global_configuration` |
| Unix creation mode, existing mode, and symlink behavior | `test_config_save_new_file_permissions_match_normal_creation`, `test_config_save_preserves_symlink_and_target_permissions`, `test_config_save_rejects_dangling_symlink_without_creating_target` |
| Stream setting defaults to 300; checker accepts 60/0 and repairs invalid input | `test_outbound_stream_lifetime_is_registered_with_runtime_default`, `test_outbound_stream_lifetime_values_survive_check_and_fix`, `test_outbound_stream_lifetime_rejects_negative_and_fixes_to_default` |
| CLI prompts, creates and overwrites using 60/0, and reprompts negative input | `config_prompts_and_writes_outbound_stream_lifetime_on_create_and_overwrite`, `config_reprompts_negative_outbound_stream_lifetime` |
| Wizard accepts the runtime's u64 range, rejects negative/overflow/noninteger values | `test_validate_outbound_stream_lifetime_accepts_unsigned_range_only` |

## Targeted results

- `cargo test --lib wfl_config::`: **28 passed** on Windows.
- `cargo test --test config_command_test --test cli_help_version_flags_test
  --test transpiler_sunset_test`: **26 passed**, including 17 configuration tests.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- `cargo build --release`: passed. Verified release SHA-256:
  `ca96e3a593e85f1921529abedd55aa0fe38e1615fb4367d1fc8d038c71fb04c3`.
- `python -X utf8 scripts/validate_docs_examples.py --ci --force --report`:
  **36 passed**.
- Canonical WFL program stage: **144 passed, 0 failed, 24 existing skips**
  (168 programs discovered). The stage was extracted verbatim from
  `scripts/run_integration_tests.ps1`, with only the binary-path variable
  supplied before it. All 233 program fixtures were verified against the
  current checkout in a fresh snapshot under `target/`; the copied release
  binary matched the hash above. No assertions, timeouts, or skips changed.
- `python scripts/check_repo_hygiene.py --mode static` and `--mode working-tree`:
  passed before the broader workspace run.

Raw logs live under `target/test-artifacts/config-review/` and are not tracked.

## Complete workspace result

`cargo test --all --no-fail-fast` on the integrated `3e391455` implementation
finished with **2,240 passed, 4 failed, 27 existing ignored**, exit 101.
The only failed target was the unchanged
`file_io_error_handling_test`: six tests passed and these four timed out:

- `test_disk_full_simulation`
- `test_double_close_file_error`
- `test_use_closed_file_handle_error`
- `test_concurrent_access_same_file_error`

The concurrent and performance file-I/O targets each passed all seven tests
in this run. These new results supersede the earlier seven-failure count for
the current candidate; they do not establish the cause of the remaining
timeouts or retroactively invalidate the earlier failures. The two additional
error-handling timeouts were not subjected to a new baseline comparison.
The full presubmit remains **not green** and its review thread remains open,
with current results tracked in issue #732. Tests were not retried, relaxed,
ignored, or skipped to obtain a passing result.

Failed existing tests left four named fixture files in the checkout. Their
paths were verified against `tests/file_io_error_handling_test.rs` and removed
after all test processes completed; this is the already tracked test-cleanup
problem, not a configuration-save artifact.

## Implementation and recovery

Each save uses a uniquely created sibling file. Rendering, flush, and `sync_all`
must succeed before `std::fs::rename` publishes it. A temporary-path guard removes
the file on failure. Normal file attributes and Unix creation permissions are
used; existing Unix permission bits are copied. The standard-library rename
supports replacement with an existing Windows reader open, which is covered by
the component regression.

Existing symlinks are resolved and their target is replaced, preserving the
link. Dangling links, non-regular targets, and read-only targets fail before
saving. Configuration format and reader remain unchanged. Cancellation before
save retains the previous behavior. Ownership and custom ACL retention are not
guaranteed, nor is power-loss durability of the parent directory entry.

Windows x86-64 is the locally exercised platform. Unix-specific permission and
symlink tests require Linux/macOS CI; coverage percentages were not measured.
No new runtime concurrency, protocol, or crypto implementation was introduced.
Earlier file-I/O failures remain tracked separately in
[issue #732](https://github.com/WebFirstLanguage/wfl/issues/732); creating that
issue does not turn a failed required check into a pass.
