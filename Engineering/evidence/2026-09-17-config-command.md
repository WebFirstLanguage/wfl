# Configuration command change evidence

## Scope and risk

Risk class: **R3** (configuration input and CLI compatibility). This changes
the WFL executable's configuration wizard, command dispatch, and maintained
documentation. It does not change interpreter syntax, TLS transport, the
configuration reader, package format, or concurrent runtime behavior.

The requested public contract is `wfl config [dir]`. The maintainer explicitly
directed immediate removal of `--init`, overriding the usual retained-alias
migration for this command. Bare `init` gives migration guidance; project
initialization is separate work. Existing script filenames and script arguments
remain usable. No deployment or publication is part of this change.

## Acceptance criteria and regression coverage

| Requirement | Automated evidence |
| --- | --- |
| Current or explicit existing directory, defaults, only `.wflcfg` created | `config_accepts_all_defaults_in_current_directory`, `config_accepts_existing_target_directory_with_spaces` |
| Remove `--init`; actionable bare `init`; canonical help/header | `removed_init_flag_explains_the_configuration_command_without_writes`, `bare_init_explains_the_configuration_command`, `config_help_does_not_start_the_wizard`, `test_generated_config_names_config_command` |
| Reject invalid targets/extra args/flags before writes | `config_rejects_missing_directory_and_file_target`, `config_rejects_extra_arguments_and_operation_flags_without_writes` |
| Preserve existing file on declined/empty confirmation or incomplete input | `config_preserves_existing_file_when_overwrite_is_declined_or_input_ends`, `config_eof_does_not_create_a_partial_configuration` |
| Confirmed overwrite after completed answers | `config_overwrites_only_after_confirmation_and_complete_answers` |
| Optional TLS paths can be skipped and omitted; required fields remain required | `test_optional_tls_settings_accept_blank_input`, `test_optional_tls_prompts_explain_enter_to_skip`, `test_generated_config_omits_unset_tls_settings`, `test_required_setting_without_default_rejects_blank_input` |
| Invalid answers reprompt; explicit TLS paths survive | `config_reprompts_invalid_input_and_preserves_explicit_tls_paths`, `test_generated_config_preserves_explicit_tls_paths` |
| Existing programs named `config`/`init` and script arguments still work | `existing_scripts_named_config_or_init_still_run`, `script_arguments_named_config_or_init_remain_script_arguments` |

The real CLI tests use Cargo's freshly built binary, OS pipes, and temporary
directories. They assert exit status, configuration contents, and absence or
preservation of files. Children have a bounded deadline. Unit tests cover the
lowest useful input-validation and generation boundary; CLI tests cover the
entire user journey. New examples are shell invocations, covered by these CLI
tests; no WFL snippets were changed and no WFL MCP tools were available.

## Red to Green

- Base: `db11db976792c9ec399966a2092d814e57fd64ca`.
- Test-only Red commit: `36f0876b` (ancestor of the implementation).
- `cargo test --lib wfl_config::wizard::tests -- --nocapture`: **8 passed,
  3 failed** before implementation. Failures were optional blank TLS rejection,
  missing skip instructions, and the old generated-file command name.
- `cargo test --test config_command_test -- --nocapture`: **2 passed,
  11 failed** before implementation. New command invocations were treated as
  filenames; `--init` still launched the wizard. Filename/script-argument
  compatibility tests passed on the base.
- The same commands after implementation: **11 unit and 13 CLI tests passed**.
- Raw local outputs: `target/test-artifacts/config-command/{red,green}-{unit,cli}.log`.

The Green implementation and this evidence are committed after the Red commit.
The final commit is identified by Git history rather than a self-referential
hash in this file.

## Broader verification

Verification is in progress; results will be filled in before completion.

## Platform, review, and recovery

Exercised platform: Windows x86-64, Rust/Cargo 1.98.1. Linux CI was not run
locally. No coverage percentage was measured. Independent review is in progress.

Concurrency, protocol fuzzing, crypto, and load tests are not newly applicable:
the change only collects configuration choices and dispatches a command; it
does not alter these runtime boundaries. Existing release-relevant suites are
run for regression coverage.

Recovery: reverting the implementation restores `--init`; the `.wflcfg` format
is unchanged and files generated here remain readable by the preceding version.
Declining overwrite or ending input before completion preserves the existing
file. As before, a filesystem failure during the final write is not an atomic
replacement guarantee; atomic config writes are outside this change.
