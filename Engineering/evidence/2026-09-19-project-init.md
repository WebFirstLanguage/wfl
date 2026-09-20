# Project initialization change evidence

## Scope and risk

Risk class: **R3**, assigned before implementation because CLI command
dispatch affects filename compatibility and initialization writes into an
existing project. The change adds `wfl init` in the current directory, with
bundled `.wflcfg`, `AGENTS.md`, and `CLAUDE.md` templates. The agent guide is
for WFL applications and points to syntax, tooling, Docker testing, and
Context7 documentation discovery.

The command is noninteractive, preserves existing regular files, rejects
nonregular destination entries before creating files, and does not load or
change global configuration. Explicit paths to extensionless `init` scripts,
normal `.wfl` files, lint source paths, and script arguments remain supported.
The user explicitly requested the bare `init` command rather than `--init`.

The interpreter, configuration parser, network protocols, dependency graph,
and release automation are unchanged. R3 checks focus on filesystem safety,
repeated initialization, CLI compatibility, failure paths, real generated
examples, and independent review. Runtime concurrency, crypto, protocol
fuzzing, and performance changes are not introduced here.

## Acceptance criteria

- The real CLI creates exactly the three documented files from bundled
  templates, without network access or prompts.
- The configuration passes the existing checker and reader.
- Help is discoverable; invalid arguments fail without creating files.
- Existing project/global files remain unchanged, including repeated runs
  and partial scaffolds.
- Directory and symbolic-link collisions are rejected without following
  links or creating other scaffold files.
- Explicit script paths and script arguments retain their prior behavior.
- All generated WFL examples pass real analysis, lint, and execution/tests.
- Generated instructions link to the user-requested Docker and Context7
  pages and explain editor LSP versus agent MCP.

## Verification record

Base revision: `6125ce348f19c9af81636e82578b13e70ab16b86`.
Test-only Red commit: `03477f17` (ancestor of implementation).
Green implementation commit: `5cdbc22d`.

Before production changes, `cargo test --test init_command_test -- --nocapture`
reported **1 passed, 13 failed** on Windows: explicit script-path compatibility
passed, while `init` was still a filename, help lacked the command, and no
scaffold was created. The log is `red-cli.log` under
`target/test-artifacts/project-init/`. Initial test compilation and an unrelated
runtime-log fixture assumption were corrected before this Red capture; their
diagnostic logs are retained separately and are not Red evidence.

The first implementation run passed 13 of 14 initialization tests and caught
an unused-variable warning in the generated test example's CLI static analysis.
The example was corrected to assert the arithmetic expression directly; no
assertion was weakened. All **14 Windows initialization tests** subsequently
passed in the full workspace run, including execution of both generated WFL
examples. The Unix-only symlink test is present but was not executed on this
Windows host.

| Acceptance criterion | Real CLI regression in `tests/init_command_test.rs` |
|---|---|
| Exactly three files, no prompt, valid config | `init_creates_only_project_configuration_and_agent_guidance_without_input` |
| Useful canonical agent context and required links | `generated_agent_adapter_links_to_useful_canonical_guidance` |
| Preserve arbitrary customized bytes and read-only files | `init_preserves_customized_files_and_reports_skips_on_rerun`, `init_preserves_read_only_existing_files_and_creates_missing_files` |
| Fill missing files and concurrent initialization | `init_fills_partial_scaffolds_without_replacing_existing_files`, `simultaneous_initializations_finish_with_one_complete_scaffold` |
| Reject collisions before writes | `init_preflights_all_reserved_names_before_writing_any_files`, Unix `init_rejects_existing_and_dangling_symlinks_without_writing` |
| Global config isolation and argument validation | `init_does_not_require_or_modify_global_configuration`, `init_help_and_main_help_advertise_the_command_without_writes`, `init_rejects_flags_and_extra_arguments_without_writes` |
| Command/filename compatibility | `bare_init_is_a_command_even_when_a_program_named_init_exists`, `explicit_init_program_paths_and_script_arguments_keep_working` |
| Runtime consumes generated config | `generated_project_config_is_discovered_by_normal_program_runs` |
| All generated WFL examples are usable | `generated_wfl_examples_lint_analyze_execute_and_run_their_tests` |

## Broader verification

Exercised platform: Windows x86-64, Rust/Cargo 1.98.1. Commands used the
existing Windows test-thread stack setting `RUST_MIN_STACK=8388608` where
needed. Raw logs are under `target/test-artifacts/project-init/`.

| Check | Result / evidence |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | Passed, `clippy.log` |
| `cargo test --all --no-fail-fast` | **2,414 passed, 0 failed, 27 pre-existing ignored**, `workspace-tests.log` |
| `cargo build --release` | Passed, `release-build.log` |
| `pwsh -NoProfile -File scripts/run_integration_tests.ps1 -TestOnly` | **Failed**: Rust integration portion 1,552 passed / 0 failed / 10 pre-existing ignored (includes repeated split suite); release WFL programs 142 passed / 2 failed / 24 predefined skips; `integration-canonical.log` |
| `pwsh -NoProfile -File scripts/run_web_tests.ps1` | Two HTTP/routing suites passed; existing TLS case skipped because OpenSSL was unavailable; `web-canonical.log` |
| Release binary project journey | Passed: creation, `--configCheck`, lint/analyze/run of both guide examples, one passing WFL assertion, and byte-for-byte preservation on rerun; `release-smoke.log`, reviewable output under `release-smoke-verified/` |
| `cargo check --locked --manifest-path fuzz/Cargo.toml --target-dir target` | Passed, `fuzz-check.log`; no dependency or lockfile changes |
| `python -X utf8 scripts/validate_docs_examples.py --ci --force --report` | **36 passed**, `docs-validation.log` |
| Generated examples through actual `wfl-lsp --mcp` | Both examples passed `parse_wfl`, `analyze_wfl`, `typecheck_wfl`, and `lint_wfl` with no diagnostics; eight calls recorded with template hash in `target/reports/project-init/mcp-template-validation.json` |
| `python scripts/check_repo_hygiene.py --mode static` | Passed, `hygiene.log` |

The original canonical WFL run recorded two failures:

- `test_basic_server.wfl`: a diagnostic capture confirmed Windows socket
  error 10048 when binding port 8080, which was already in use. No existing
  service was stopped. See `diagnostic-basic-server.log`.
- `file_io_comprehensive.wfl`: exceeded the canonical 30-second timeout. Its
  unchanged program recursively lists the current directory, including this
  checkout's large `target/` tree. A bounded diagnostic run took 16.407 seconds
  in the repository and 0.094 seconds in an isolated directory. This supports
  sensitivity to working-directory size; it does not turn the original
  timeout into a pass. See `diagnostic-file-io-root.log` and
  `diagnostic-file-io-isolated.log`.

After the user reported clearing port 8080, `test_basic_server.wfl` was rerun
once at **2026-09-19 14:13:51 UTC**, using the same release executable, unchanged
program, repository working directory, and 30-second deadline. It **passed**
in 0.031 seconds with exit code 0 and the expected server-start/listening
messages. The command and assertions are recorded in `port-8080-retest.log`.
This verifies the previously blocked bind/start test after the environment
changed; it does not exercise an HTTP request. No code, assertions, or timeout
limits were changed, and the full canonical suite was not rerun.

The original canonical result is retained, and the full local presubmit is
still **not green** despite passing all workspace and new initialization tests
and the port-8080 retest. Linux CI, the directory-scan timeout, and the
unavailable TLS script prerequisite remain verification limits before claiming
merge/release readiness.

The Windows host supplied conflicting case-variant PATH entries. The canonical
script initially refused that environment before testing. A temporary helper
selected the path returned by native Windows `GetEnvironmentVariableW`, removed
case variants from the child environment, and supplied that one value without
merging paths. The repository scripts were not changed. Initial environment
diagnostics are retained separately. Python checks used the bundled interpreter
and native Git path; docs validation used UTF-8 mode. Test-created root files
were checked after completion and had been cleaned by the test programs.

The independent reviewer inspected actual command dispatch, the initializer,
CLI tests, Red/initial Green logs, and the dependency's Windows/Unix
`persist_noclobber` implementation. No blocking findings remained. Review
confirmed no-clobber behavior, preflight without following symlinks, cleanup
of temporary files on ordinary failures, and explicit script-path compatibility.
The review called out the initially failing example (subsequently fixed),
unexecuted Unix coverage, and the absence of a whole-project transaction or
power-loss durability guarantee.

No coverage percentage was measured. Linux CI and Docker execution were not
run locally; Docker commands were checked against the canonical guide and
Dockerfile. Live retrieval of the two user-supplied URLs was unavailable;
Docker guidance was checked in the local repository, and Context7 is linked
as a discovery entry point without claiming its current indexed contents.

## Pull request preparation

The feature branch was integrated with `origin/main` at
`3e23bd8972b996beacfe6553e98236745c050f35` before publishing the pull request.
That revision contains the already-tested lint changes as a squash commit and
the version update to 26.9.11. The merge retains all of main's version and
lockfile metadata. The two overlapping additions in `src/main.rs` and the
configuration reference resolve to the same content as the tested feature
files. The resulting diff against main contains only the 14 initialization
implementation, test, template, documentation, and evidence files.

On this integrated tree, `cargo fmt --all -- --check` passed and
`cargo test --test init_command_test --test config_command_test
--test cli_help_version_flags_test --test lint_cli_test` passed **84 tests**
with no failures (`pr-cli-tests.log`). The integrated-tree
`cargo clippy --all-targets --all-features -- -D warnings` check also passed
(`pr-clippy.log`). An independent read-only review of the
diff confirmed that the existing lint behavior and main's version are
preserved. The earlier full workspace and end-to-end results above retain
their original scope and limitations; this focused check does not claim a
new complete presubmit run.

## Initial GitHub verification and review

For head `5a34cadc969e2f838deb31216d043d6d8d79a075`,
[CI run 35487564648](https://github.com/WebFirstLanguage/wfl/actions/runs/35487564648),
[Docker runtime run 35487564664](https://github.com/WebFirstLanguage/wfl/actions/runs/35487564664),
and [configuration lint run 35487564544](https://github.com/WebFirstLanguage/wfl/actions/runs/35487564544)
all completed successfully. Both Linux and Windows integration logs record
144 WFL programs passed, zero failed, 24 existing exclusions, 36 documentation
examples passed, and the HTTP, routing, and HTTPS tests passed. Both explicitly
passed `file_io_comprehensive.wfl`. Unix symlink coverage, workspace tests,
extension checks, Clippy, hygiene, database tests, and fuzz compilation passed
in their applicable jobs. These results supersede the earlier pending CI/TLS
coverage statements for this head; they do not erase or explain the original
local directory-scan timeout.

[Codex review comment 4056051976](https://github.com/WebFirstLanguage/wfl/pull/736#discussion_r4056051976)
identified that staged Unix files inherited tempfile's default `0600` mode,
preventing ordinary shared-project reads. CodeRabbit reported missing helper
documentation (78.57% docstring coverage), with no inline behavioral findings;
Devin reported no issues. Review follow-up adds explicit Unix creation-mask
and existing-permission regression coverage plus useful helper documentation.

### Unix permissions regression

- Affected base: `5a34cadc969e2f838deb31216d043d6d8d79a075`.
- Test-only Red: `f6bb75efc939d74cf0a5f911405455657bbad1d2`.
  [Linux integration job 106020774651](https://github.com/WebFirstLanguage/wfl/actions/runs/35489070117/job/106020774651)
  failed at **2026-09-20 04:27:48 UTC**, before the implementation changed:
  `init_uses_normal_file_permissions_respecting_each_child_umask` observed
  `.wflcfg` mode `384` (`0600`) instead of `420` (`0644`) under mask `0022`.
  The initialization suite had 16 passes and this one failure. The separate
  Build/Test job reproduced the same assertion. Windows integration was
  canceled by matrix fail-fast, not evidence of a Windows test failure.
- The CLI regression covers masks `0022`, `0002`, and `0077` for all three
  generated files, and retains existing `0640`, `0600`, and `0444` modes and
  contents under the more permissive mask `0000`. A child shell applies each
  mask before executing the real CLI; the Rust test process's mask is never
  changed. The existing bounded process/output helper is shared with these
  tests. Windows ran all 14 applicable initializer tests successfully before
  the fix; Unix-specific assertions run in Linux CI because this host has no
  installed Linux environment.
- The fix asks tempfile's builder for mode `0666` on Unix at creation. The
  operating system applies the inherited mask; there is no process-wide mask
  mutation or later `chmod` that could undo restrictions. `persist_noclobber`,
  preflight, preservation of existing files, and Windows defaults are retained.
  Cached tempfile 3.27.0 source and an independent review confirm this path.
- Helper docstrings now explain filesystem error context, CLI argument
  handling, bounded execution, output checks, and scaffold assertions. The
  CodeRabbit percentage is an external review result and is not assumed to
  have changed until a fresh review reports it.

Post-fix Windows validation passed: `cargo fmt --all -- --check`,
`cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo test --all --no-fail-fast` (**2,414 passed, zero failed, 27 existing
ignored**), including all 14 Windows initializer tests. Logs are
`target/reports/project-init/review-workspace-tests.log` and
`review-clippy.log`. An independent review of the actual fix found no blocking
issues, including Unix mask handling and unchanged Windows creation defaults.
Linux verification of the fixed revision remains pending until the next CI
run; earlier green CI is not evidence for the new permission behavior.

## Recovery and limits

No existing file is replaced, merged, or refreshed. Correct a conflicting
destination and rerun to fill missing files. Creation is not a transaction
across all three files: a late filesystem error can leave completed files.
Do not delete an existing customized file merely to regenerate the template.

The embedded instructions work offline, while following external links
requires access to those sites. The guide directs agents to check runtime
versions because upstream `main` documentation and nightly images can move.
No integrations are installed or configured by initialization.
