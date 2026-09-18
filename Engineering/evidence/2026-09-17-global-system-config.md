# Global WFL setup verification

## Contract and risk

`wfl config` is system setup. It accepts no directory argument, writes the
existing global configuration path (`C:\wfl\config` on Windows,
`/etc/wfl/wfl.cfg` elsewhere), and honors `WFL_GLOBAL_CONFIG_PATH`. It neither
creates nor changes project `.wflcfg` files. It always dispatches setup even
when the working directory contains a file called `config`; explicit paths
such as `wfl ./config` still execute programs.

Risk: **R3**, configuration persistence and CLI compatibility. The maintainer
explicitly specified this final command contract and removal of obsolete
command references, including documentation. The archive comparison was
updated with its manifest checksum. No host-wide configuration was changed
during verification: tests set the global-path override inside temporary
directories.

## Test-first evidence

- Base: `d13030c1`.
- Test-only Red commit: `512d665c`, preceding implementation.
- Green implementation commit: `f3f10ba8`.
- `cargo test --test config_command_test`: **4 passed, 10 failed** before
  implementation. The command still wrote local configuration, accepted a
  directory, and deferred to a same-named file.
- `cargo test --lib wfl_config::wizard::tests::test_generate_file`: **1 passed,
  1 failed** before implementation; missing parent directories prevented saving.
- The global-default CLI test also independently failed its updated banner
  assertion before implementation (`red-banner.log`).

## Acceptance coverage

| Behavior | Regression tests |
| --- | --- |
| Global destination with defaults, including spaces in its parent path | `config_accepts_all_defaults_in_global_configuration` |
| Project configuration remains byte-for-byte unchanged | `config_preserves_local_configuration` |
| Setup works when cwd contains a file called `config` | `config_command_works_when_current_directory_contains_config_file` |
| Help documents global setup without a directory argument or writes | `config_help_does_not_start_the_wizard` |
| Positional arguments, extra flags rejected before wizard/writes | `config_rejects_directory_and_file_arguments`, `config_rejects_extra_arguments_and_operation_flags_without_writes` |
| Overwrite refusal and EOF preserve existing global file | `config_preserves_existing_file_when_overwrite_is_declined_or_input_ends` |
| Overwrite requires confirmation and complete input | `config_overwrites_only_after_confirmation_and_complete_answers` |
| EOF leaves neither configuration nor new parent directories | `config_eof_does_not_create_a_partial_configuration` |
| Invalid input reprompts; explicit TLS paths persist | `config_reprompts_invalid_input_and_preserves_explicit_tls_paths` |
| Explicit program paths and script arguments still execute | `explicit_program_paths_still_run`, `script_arguments_named_config_remain_script_arguments` |
| Relative global-path override works | `global_configuration_path_can_be_relative` |
| Missing parents created at final save; blocked parent preserved | `test_generate_file_creates_missing_parent_directories`, `test_generate_file_preserves_file_blocking_parent_directory`, `global_configuration_write_failure_preserves_existing_files` |

Existing wizard unit tests continue to cover omitted optional TLS paths,
required values, defaults, and validation of booleans, numbers, and IPs.

## Results

- `cargo test --lib`: **697 passed, 0 failed, 6 existing ignored tests**.
- `cargo test --test config_command_test --test cli_help_version_flags_test
  --test transpiler_sunset_test`: **23 passed**, including all 14 configuration
  CLI tests.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- `cargo build --release`: passed.
- Direct release CLI smoke: global configuration created in a nested path,
  optional TLS keys omitted, local `.wflcfg` preserved, directory argument
  rejected without modifying global contents.
- Static and working-tree hygiene checks: passed, including archive checksum.
- Independent read-only review approved implementation, coverage, and docs;
  no actionable defects were found.
- `python -X utf8 scripts/validate_docs_examples.py --ci --force --report`:
  **34 passed**.
- Canonical WFL program stage using the fresh release binary and complete
  fixture snapshot: **142 passed, 0 failed, 24 existing canonical skips**.
  All 217 fixture files and the copied subprocess binary were verified against
  the current checkout. Runner assertions/timeouts were unchanged; only absolute
  discovery/binary paths were adjusted to keep outputs under `target/`.
- Tracked-content search found no obsolete command spelling or directory-form
  setup invocation.

Raw outputs: `target/test-artifacts/system-config/`.

## Limits and recovery

Windows x86-64 was exercised; Linux CI and coverage percentages were not.
The earlier complete workspace run had unrelated file-I/O timing failures,
with baseline comparisons recorded in
[the initial evidence](2026-09-17-config-command.md). This follow-up does not
change those runtime paths or claim that the full presubmit is green.
No WFL snippets changed; CLI examples are exercised through the real binary.

Cancelling or ending input before save preserves existing configuration and
does not create directories. A blocked parent path is reported without
altering that file. The configuration format and reader are unchanged;
existing global and local files remain readable. As before, failures during
the final file write do not have an atomic replacement guarantee. No merge,
publication, or system configuration installation is part of this work.
