# Configuration command change evidence

This record covers the initial implementation and verification. The final
public command is `wfl config` with no directory argument: it writes global
configuration and leaves project `.wflcfg` files unchanged. The initial checks
below predate that final scope refinement and do not substitute for its
separate verification.

## Scope and risk

Risk class: **R3** (configuration input and CLI compatibility). This changes
the WFL executable's configuration wizard, command dispatch, and maintained
documentation. It does not change interpreter syntax, TLS transport, the
configuration reader, package format, or concurrent runtime behavior.

The initial implementation established command dispatch and optional wizard
inputs. Its tests and results are retained below as evidence for that stage.
No deployment or publication was part of this change.

## Acceptance criteria and regression coverage

| Requirement | Automated evidence |
| --- | --- |
| Initial defaults and configuration-file generation | `config_accepts_all_defaults_in_current_directory`, `config_accepts_existing_target_directory_with_spaces` |
| Canonical help and generated-file header | `config_help_does_not_start_the_wizard`, `test_generated_config_names_config_command` |
| Reject invalid targets/extra args/flags before writes | `config_rejects_missing_directory_and_file_target`, `config_rejects_extra_arguments_and_operation_flags_without_writes` |
| Preserve existing file on declined/empty confirmation or incomplete input | `config_preserves_existing_file_when_overwrite_is_declined_or_input_ends`, `config_eof_does_not_create_a_partial_configuration` |
| Confirmed overwrite after completed answers | `config_overwrites_only_after_confirmation_and_complete_answers` |
| Optional TLS paths can be skipped and omitted; required fields remain required | `test_optional_tls_settings_accept_blank_input`, `test_optional_tls_prompts_explain_enter_to_skip`, `test_generated_config_omits_unset_tls_settings`, `test_required_setting_without_default_rejects_blank_input` |
| Invalid answers reprompt; explicit TLS paths survive | `config_reprompts_invalid_input_and_preserves_explicit_tls_paths`, `test_generated_config_preserves_explicit_tls_paths` |
| Initial filename and script-argument compatibility checks | `existing_scripts_named_config_or_init_still_run`, `script_arguments_named_config_or_init_remain_script_arguments` |

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
  filenames; the wizard still used its previous entry point. Filename/script-argument
  compatibility tests passed on the base.
- The same commands after implementation: **11 unit and 13 CLI tests passed**.
- Raw local outputs: `target/test-artifacts/config-command/{red,green}-{unit,cli}.log`.

Green implementation commit: `5d7c6e52`, after the Red commit. Later evidence-only
commits record verification without changing that implementation.

## Broader verification

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | Passed |
| `cargo build --release` | Passed |
| Focused wizard and real CLI regressions | 11 unit + 13 CLI passed |
| Direct release-binary `wfl config` smoke in a temporary directory | Passed: defaults generated, TLS paths omitted |
| `cargo test --all` | Failed at existing file-I/O concurrent tests |
| `cargo test --all --no-fail-fast` | 2,152 passed, 7 failed, 27 existing ignored tests; all remaining targets and doc tests were exercised |
| `pwsh -NoProfile -File scripts/run_integration_tests.ps1 -TestOnly` | Failed at existing file-I/O concurrent tests before the WFL program stage |
| Canonical WFL program stage, run separately with current release binary and complete fixture snapshot | 142 passed, 0 failed, 24 existing canonical skips (166 discovered) |
| `pwsh -NoProfile -File scripts/run_web_tests.ps1` | 2 HTTP/routing suites passed; TLS script skipped because OpenSSL is unavailable |
| `python -X utf8 scripts/validate_docs_examples.py --ci --force --report` | 34 examples passed |
| `python scripts/check_repo_hygiene.py --mode static` | Passed |

The no-fail-fast workspace run reports failures in three unchanged suites:
`file_io_concurrent_test` (concurrent writes, large concurrent operations),
`file_io_error_handling_test` (closed handle, concurrent access), and
`file_io_performance_test` (many small files, rapid operations, concurrent
performance). Their in-memory interpreter path neither dispatches CLI commands
nor calls the wizard. No new skips, relaxed assertions, or retries until green
were introduced.
The no-fail-fast run was used to exercise targets left unrun by the original
fail-fast invocation; its failures remain failures.

For comparison, the unchanged base revision was extracted with `git archive`
under `target/test-artifacts/config-command/baseline`. The three suites were
built with `cargo test --manifest-path <snapshot>/Cargo.toml --target-dir target
--test file_io_concurrent_test --test file_io_error_handling_test
--test file_io_performance_test --no-run`, then their exact executables were
run in an isolated directory under `target/`. Baseline concurrent tests had
3 timeouts (4 passed); baseline error-handling tests had the same 2 timeouts
(8 passed). Baseline performance tests passed 7/7. This reproduces the four
concurrent/error-handling failures from the complete current run on the base;
the three performance failures remain a timing limitation observed in the
full current run, not proven baseline failures. The full presubmit is therefore
**not green**, and this change is not claimed ready for merge/release.

The WFL program stage was extracted unchanged from the canonical PowerShell
runner except for absolute binary/discovery paths. It ran inside the complete
base fixture snapshot so relative fixture paths worked and runtime output
stayed under `target/`; all 217 WFL/config fixtures and the copied subprocess
binary were verified identical to the current checkout. A preceding attempt
from an empty working directory is retained as diagnostic evidence only.

Windows host setup required normalizing duplicate case-variant PATH entries
in the child process environment before running the PowerShell scripts. Python
UTF-8 mode avoided a cp1252 subprocess decoder error in the first docs attempt.
These adjustments changed neither tracked scripts nor test assertions. The
initial setup-failure logs are retained separately. Test outputs left at the
repository root by failed existing tests were identified from their source
and removed after the suites stopped.

Logs are under `target/test-artifacts/config-command/`: `clippy.log`,
`release-build.log`, `workspace-tests.log`, `workspace-tests-complete.log`,
`integration-tests.log`, `programs-tests.log`, `web-tests.log`,
`docs-validation.log`, `baseline-build.log`, and `baseline-file-io.log`.
The shared target directory initially left the old baseline CLI executable in
place: Cargo treated the current artifact as fresh even though the baseline
build had overwritten it. That diagnostic failure is retained in
`baseline-cache-collision.log`. The WFL package's debug artifacts were then
cleaned (`cargo clean -p wfl --profile dev`) before rebuilding the current
source and rerunning the focused regressions; results are in `final-unit.log`
and `final-cli.log`. The verified current release binary was unaffected.

## Platform, review, and recovery

Exercised platform: Windows x86-64, Rust/Cargo 1.98.1. Linux CI was not run
locally. No coverage percentage was measured. An independent read-only review
agent approved the implementation/tests/docs and separately checked the Red
and Green logs and evidence links; it found no actionable defects. It did not
run the broader suites itself.

Concurrency, protocol fuzzing, crypto, and load tests are not newly applicable:
the change only collects configuration choices and dispatches a command; it
does not alter these runtime boundaries. Existing release-relevant suites are
run for regression coverage.

Recovery: the configuration format is unchanged, and files generated during
this initial implementation remain readable by the preceding version.
Declining overwrite or ending input before completion preserves the existing
file. As before, a filesystem failure during the final write is not an atomic
replacement guarantee; atomic config writes are outside this change.
