# Strict Subprocess Allowlist Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `allowlist_only` authorize and execute only canonical absolute executable paths through direct process launches, while rejecting all WFL-created shell launches and same-basename spoofing.

**Architecture:** `CommandSanitizer` will return the exact executable path authorized for every direct launch. In strict allowlist mode it will canonicalize the requested and configured absolute paths and return the matching canonical identity; `IoClient` will carry that identity through a launch decision and pass it directly to `Command::new`. Shell-capable modes retain their existing shell path, but `allowlist_only` fails before shell construction.

**Tech Stack:** Rust 2024, Tokio subprocesses, `tempfile` test fixtures, WFL release-binary integration tests, Markdown/INI documentation.

## Global Constraints

- TDD is mandatory: add each behavioral regression before production code, run it, and observe the expected failure caused by the vulnerable behavior.
- `allowlist_only` must reject both inferred shell syntax and explicit `using shell` for `execute command` and `spawn command`.
- A direct `allowlist_only` launch must require an absolute requested path and a canonical identity match against an absolute `allowed_shell_commands` entry.
- The canonical path returned by authorization must be the path passed to `Command::new`; do not authorize one token and execute another.
- Bare names, relative paths, alternate same-basename paths, PATH lookup, and current-directory lookup must not authorize strict allowlist execution.
- Direct argv items remain literal. `with arguments` data must never be sent through the WFL-created shell path in `allowlist_only`.
- Do not add a denylist of interpreter names. Documentation must explain that explicitly allowlisting an interpreter grants that executable's argument-level capabilities.
- Preserve existing behavior for `forbidden`, `sanitized`, and `unrestricted` modes.
- Apply the same preparation and authorization semantics to foreground execution and background spawning.
- Keep all exploit fixtures harmless; use version output and marker text, never destructive payloads.
- Preserve the existing `.wflcfg` keys and WFL statement syntax. Document the corrective compatibility break and the absolute-path migration.
- Do not publish an advisory, create a public security issue, alter release metadata, or disclose beyond the in-repository fix materials; those are Maintainer decisions.
- Baseline note: before implementation, `cargo test --all --verbose` had one unrelated deterministic Windows failure in `wflpkg --test security_tests::test_extract_archive_rejects_absolute_path`. The archive was correctly rejected, but the test expected “absolute path” while the implementation returned “Archive entry escapes destination.” Do not modify that unrelated crate or test. No new failures are acceptable.
- Required final gates are `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all --verbose`, a release build plus the Windows integration script, and documentation example validation.

---

## File map

- `src/interpreter/command_sanitizer.rs`: strict policy decision, canonical executable resolution, and policy unit tests.
- `src/interpreter/mod.rs`: carries the authorized executable identity into both Tokio process-construction paths.
- `tests/subprocess_security_test.rs`: release-binary exploit regressions for shell injection, spawning, path spoofing, and successful direct argv.
- `src/wfl_config/checker.rs`: cross-setting validation and help text for absolute allowlist paths.
- `Docs/04-advanced-features/subprocess-execution.md`: user behavior, safe usage, and migration.
- `Docs/06-best-practices/security-guidelines.md`: remove the unsafe `cmd.exe /C` argument example and explain interpreter capability.
- `Docs/reference/configuration-reference.md`: authoritative strict allowlist semantics and examples.
- `CHANGELOG.md`: Unreleased security correction and migration summary.
- `Dev diary/2026-07-12-allowlist-subprocess-hardening.md`: engineering record, test evidence, and residual trust boundary.

### Task 1: Bind strict allowlist authorization to direct executable identity

**Files:**
- Modify: `src/interpreter/command_sanitizer.rs:1-319,321-608`
- Modify: `src/interpreter/mod.rs:759-784,1308-1489,9877-9943`
- Modify: `tests/subprocess_security_test.rs:1-504`

**Interfaces:**
- Consumes: `WflConfig::{allow_shell_execution, shell_execution_mode, allowed_shell_commands}`, `ShellExecutionMode`, `CommandSanitizer::parse_command`, and the existing execute/spawn arguments.
- Produces: `ValidationResult::Safe { executable: PathBuf }` and a private `AuthorizedSubprocess::{Shell, Direct { executable: PathBuf }}` decision consumed identically by execute and spawn.

- [ ] **Step 1: Add unit regressions for shell authorization and path spoofing**

Add this helper and these tests inside `command_sanitizer.rs`'s existing test module. They use the existing public authorization entry point, so they compile against the vulnerable code and fail behaviorally before the enum refactor.

```rust
fn allowlist_config(allowed_shell_commands: Vec<String>) -> Arc<WflConfig> {
    Arc::new(WflConfig {
        allow_shell_execution: true,
        shell_execution_mode: ShellExecutionMode::AllowlistOnly,
        allowed_shell_commands,
        ..Default::default()
    })
}

#[test]
fn test_allowlist_only_blocks_shell_for_allowed_program() {
    let executable = std::env::current_exe().unwrap();
    let sanitizer = CommandSanitizer::new(allowlist_config(vec![
        executable.to_string_lossy().into_owned(),
    ]));

    let result = sanitizer
        .authorize_process_execution(
            executable.to_str().unwrap(),
            true,
            "allowed-tool --version; echo injected",
        )
        .unwrap();

    assert!(
        matches!(&result, ValidationResult::Blocked { reason }
            if reason.contains("does not permit shell execution")),
        "allowlist_only must reject shell execution, got {result:?}"
    );
}

#[test]
fn test_allowlist_only_rejects_bare_program_names() {
    let sanitizer = CommandSanitizer::new(allowlist_config(vec!["echo".to_string()]));

    let result = sanitizer
        .authorize_process_execution("echo", false, "echo")
        .unwrap();

    assert!(
        matches!(&result, ValidationResult::Blocked { reason }
            if reason.contains("absolute executable path")),
        "bare program names must not enter OS path lookup, got {result:?}"
    );
}

#[test]
fn test_allowlist_only_rejects_different_path_with_same_basename() {
    let temp = tempfile::tempdir().unwrap();
    let allowed_dir = temp.path().join("allowed");
    let attacker_dir = temp.path().join("attacker");
    std::fs::create_dir_all(&allowed_dir).unwrap();
    std::fs::create_dir_all(&attacker_dir).unwrap();
    let allowed = allowed_dir.join("same-name-tool");
    let attacker = attacker_dir.join("same-name-tool");
    std::fs::write(&allowed, b"allowed").unwrap();
    std::fs::write(&attacker, b"attacker").unwrap();

    let sanitizer = CommandSanitizer::new(allowlist_config(vec![
        allowed.to_string_lossy().into_owned(),
    ]));
    let result = sanitizer
        .authorize_process_execution(attacker.to_str().unwrap(), false, "same-name-tool")
        .unwrap();

    assert!(
        matches!(&result, ValidationResult::Blocked { reason }
            if reason.contains("not in allowed_shell_commands")),
        "same basename at another path must be blocked, got {result:?}"
    );
}

#[cfg(unix)]
#[test]
fn test_allowlist_only_accepts_symlink_to_same_canonical_executable() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("real-tool");
    let alias = temp.path().join("alias-tool");
    std::fs::write(&target, b"allowed").unwrap();
    symlink(&target, &alias).unwrap();
    let sanitizer = CommandSanitizer::new(allowlist_config(vec![
        target.to_string_lossy().into_owned(),
    ]));

    let result = sanitizer
        .authorize_process_execution(alias.to_str().unwrap(), false, "alias-tool")
        .unwrap();

    assert!(
        matches!(&result, ValidationResult::Safe),
        "a symlink to the same canonical file should be allowed, got {result:?}"
    );
}
```

- [ ] **Step 2: Run the unit regressions and verify RED**

Run:

```powershell
cargo test --lib interpreter::command_sanitizer::tests::test_allowlist_only -- --nocapture
```

Expected: the shell, bare-name, and same-basename tests fail because the current implementation returns `RequiresShell` or `Safe` from basename matching. On Unix, the symlink-identity test also fails because different basenames are treated as different programs even though they resolve to the same file. Existing unrelated sanitizer tests remain green.

- [ ] **Step 3: Add release-binary exploit regressions before production changes**

Add `Path` to the imports in `tests/subprocess_security_test.rs`, then add these reusable helpers:

```rust
fn escape_wfl_text(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

fn escape_wfl_string(value: &Path) -> String {
    escape_wfl_text(value.to_string_lossy().as_ref())
}

fn absolute_allowlist_config(executable: &Path) -> String {
    format!(
        "allow_shell_execution = true\nshell_execution_mode = allowlist_only\nallowed_shell_commands = {}\nwarn_on_shell_execution = false\n",
        executable.display()
    )
}

fn copy_release_binary(destination: &Path) {
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::copy(fs::canonicalize(wfl_exe()).unwrap(), destination).unwrap();
}

fn test_tool_name() -> &'static str {
    if cfg!(windows) {
        "same-name-tool.exe"
    } else {
        "same-name-tool"
    }
}
```

Add execute and spawn shell regressions using the intentionally insecure basename fixture so the vulnerable implementation demonstrates the first-token bypass:

```rust
#[test]
fn test_allowlist_only_blocks_shell_command_after_allowed_first_token() {
    #[cfg(not(windows))]
    let code = r#"
        wait for execute command "echo allowlisted; echo injected" as result
        display result
    "#;
    #[cfg(windows)]
    let code = r#"
        wait for execute command "cmd.exe /C echo allowlisted & echo injected" as result
        display result
    "#;

    assert_blocked(
        run_wfl_with_config(code, Some(ALLOWLIST_PROGRAM_CONFIG)),
        "Allowlisted first token with shell continuation",
    );
}

#[test]
fn test_allowlist_only_blocks_shell_spawn_after_allowed_first_token() {
    #[cfg(not(windows))]
    let code = r#"
        spawn command "echo allowlisted; echo injected" as proc_id
        wait for process proc_id to complete
    "#;
    #[cfg(windows)]
    let code = r#"
        spawn command "cmd.exe /C echo allowlisted & echo injected" as proc_id
        wait for process proc_id to complete
    "#;

    assert_blocked(
        run_wfl_with_config(code, Some(ALLOWLIST_PROGRAM_CONFIG)),
        "Allowlisted spawn first token with shell continuation",
    );
}

#[test]
fn test_allowlist_only_blocks_explicit_using_shell_execute() {
    #[cfg(not(windows))]
    let code = r#"
        wait for execute command "echo allowlisted" using shell as result
        display result
    "#;
    #[cfg(windows)]
    let code = r#"
        wait for execute command "cmd.exe /C echo allowlisted" using shell as result
        display result
    "#;

    assert_blocked(
        run_wfl_with_config(code, Some(ALLOWLIST_PROGRAM_CONFIG)),
        "Explicit using shell execute under allowlist_only",
    );
}

#[test]
fn test_allowlist_only_blocks_explicit_using_shell_spawn() {
    #[cfg(not(windows))]
    let code = r#"
        spawn command "echo allowlisted" using shell as proc_id
        wait for process proc_id to complete
    "#;
    #[cfg(windows)]
    let code = r#"
        spawn command "cmd.exe /C echo allowlisted" using shell as proc_id
        wait for process proc_id to complete
    "#;

    assert_blocked(
        run_wfl_with_config(code, Some(ALLOWLIST_PROGRAM_CONFIG)),
        "Explicit using shell spawn under allowlist_only",
    );
}
```

Add the absolute same-basename path regression:

```rust
#[test]
fn test_allowlist_only_blocks_same_basename_at_another_absolute_path() {
    let temp = TempDir::new().unwrap();
    let allowed = temp.path().join("allowed").join(test_tool_name());
    let attacker = temp.path().join("attacker").join(test_tool_name());
    copy_release_binary(&allowed);
    copy_release_binary(&attacker);
    let config = absolute_allowlist_config(&allowed);
    let code = format!(
        "wait for execute command \"{}\" with arguments [\"--version\"] as result\ndisplay result\n",
        escape_wfl_string(&attacker)
    );

    assert_blocked(
        run_wfl_with_config(&code, Some(&config)),
        "Same basename at another absolute path",
    );
}
```

Replace `test_allowlist_only_allows_listed_program` with an absolute-path green control that invokes the release WFL binary with `--version` through direct argv:

```rust
#[test]
fn test_allowlist_only_allows_exact_absolute_program() {
    let executable = fs::canonicalize(wfl_exe()).unwrap();
    let config = absolute_allowlist_config(&executable);
    let code = format!(
        "wait for execute command \"{}\" with arguments [\"--version\"] as result\ndisplay result\n",
        escape_wfl_string(&executable)
    );

    let result = run_wfl_with_config(&code, Some(&config));
    assert!(result.is_ok(), "Exact absolute executable should run: {result:?}");
}

#[test]
fn test_allowlist_only_keeps_metacharacters_literal_in_explicit_args() {
    let temp = TempDir::new().unwrap();
    let child_program = temp.path().join("argv_control.wfl");
    fs::write(&child_program, "display \"ARGUMENT_BOUNDARY_OK\"\n").unwrap();
    let executable = fs::canonicalize(wfl_exe()).unwrap();
    let config = absolute_allowlist_config(&executable);
    let literal_arg = if cfg!(windows) {
        "literal & echo SHELL_INJECTED"
    } else {
        "literal; echo SHELL_INJECTED"
    };
    let code = format!(
        "wait for execute command \"{}\" with arguments [\"{}\", \"{}\"] as result\ndisplay result\n",
        escape_wfl_string(&executable),
        escape_wfl_string(&child_program),
        escape_wfl_text(literal_arg)
    );

    let output = run_wfl_with_config(&code, Some(&config))
        .expect("literal metacharacter argv must remain a direct launch");
    assert!(output.contains("ARGUMENT_BOUNDARY_OK"), "child did not run: {output}");
    assert!(
        !output.lines().any(|line| line.trim() == "SHELL_INJECTED"),
        "argument data was interpreted by a shell: {output}"
    );
}
```

- [ ] **Step 4: Build the unchanged release binary and verify integration RED**

Run:

```powershell
cargo build --release
cargo test --test subprocess_security_test allowlist_only -- --nocapture
```

Expected: the inferred-shell, explicit-shell, and alternate-path tests fail because the vulnerable implementation authorizes a basename and permits `RequiresShell`. The exact-absolute and literal-argv controls pass and demonstrate behavior that the fix must preserve.

- [ ] **Step 5: Implement canonical strict-allowlist authorization**

In `src/interpreter/command_sanitizer.rs`, import `Path` and `PathBuf`, change the safe decision to carry the authorized executable, and keep the existing shell/block variants:

```rust
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, PartialEq)]
pub enum ValidationResult {
    Safe { executable: PathBuf },
    RequiresShell {
        reason: String,
        warnings: Vec<String>,
    },
    Blocked { reason: String },
}
```

Add these helpers inside `impl CommandSanitizer`:

```rust
fn strip_optional_path_quotes(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}

fn canonicalize_absolute_executable(value: &str, label: &str) -> Result<PathBuf, String> {
    let value = Self::strip_optional_path_quotes(value);
    let path = Path::new(value);
    if !path.is_absolute() {
        return Err(format!(
            "{label} '{value}' is not an absolute executable path"
        ));
    }

    let canonical = std::fs::canonicalize(path)
        .map_err(|error| format!("{label} '{}' could not be resolved: {error}", path.display()))?;
    if !canonical.is_file() {
        return Err(format!(
            "{label} '{}' does not resolve to a file",
            canonical.display()
        ));
    }
    Ok(canonical)
}

#[cfg(windows)]
fn executable_paths_match(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(right.to_string_lossy().as_ref())
}

#[cfg(not(windows))]
fn executable_paths_match(left: &Path, right: &Path) -> bool {
    left == right
}

fn resolve_allowlisted_executable(&self, program: &str) -> Result<PathBuf, String> {
    let requested = Self::canonicalize_absolute_executable(program, "Requested executable")?;
    let mut unusable_entries = Vec::new();

    for allowed in &self.config.allowed_shell_commands {
        match Self::canonicalize_absolute_executable(allowed, "Allowlist entry") {
            Ok(candidate) if Self::executable_paths_match(&requested, &candidate) => {
                return Ok(requested);
            }
            Ok(_) => {}
            Err(reason) => unusable_entries.push(reason),
        }
    }

    let unusable = if unusable_entries.is_empty() {
        String::new()
    } else {
        format!(" Unusable entries: {}", unusable_entries.join("; "))
    };
    Err(format!(
        "Executable '{}' resolves to '{}' but is not in allowed_shell_commands.{unusable}",
        program,
        requested.display()
    ))
}
```

Replace the `AllowlistOnly`, direct `Sanitized`, and direct `Unrestricted` branches in `authorize_process_execution` with decisions that carry a path:

```rust
ShellExecutionMode::AllowlistOnly => {
    if needs_shell {
        Ok(ValidationResult::Blocked {
            reason: "allowlist_only does not permit shell execution; use direct execution with arguments or deliberately select a shell-capable mode".to_string(),
        })
    } else {
        match self.resolve_allowlisted_executable(program) {
            Ok(executable) => Ok(ValidationResult::Safe { executable }),
            Err(reason) => Ok(ValidationResult::Blocked { reason }),
        }
    }
}
ShellExecutionMode::Sanitized => {
    if needs_shell {
        let warnings = self.analyze_shell_features(command_for_analysis);
        Ok(ValidationResult::RequiresShell {
            reason: "Command contains shell features".to_string(),
            warnings,
        })
    } else {
        Ok(ValidationResult::Safe {
            executable: PathBuf::from(program),
        })
    }
}
ShellExecutionMode::Unrestricted => {
    if needs_shell {
        Ok(ValidationResult::RequiresShell {
            reason: "Unrestricted shell mode enabled".to_string(),
            warnings: vec!["⚠️ Using unrestricted shell execution".to_string()],
        })
    } else {
        Ok(ValidationResult::Safe {
            executable: PathBuf::from(program),
        })
    }
}
```

Make `validate_command` fail closed without a first-token fallback:

```rust
pub fn validate_command(&self, command: &str) -> Result<ValidationResult, String> {
    let has_shell_features = Self::contains_shell_metacharacters(command);
    let program = if has_shell_features {
        String::new()
    } else {
        Self::parse_command(command)?.0
    };
    self.authorize_process_execution(&program, has_shell_features, command)
}
```

Remove the pre-match `program_base` local, the private `is_program_allowlisted`, and the private `get_command_base`. Preserve the public `program_basename` utility for source compatibility, but do not call it from authorization. Preserve `is_allowlisted` with strict semantics by replacing its body with:

```rust
pub fn is_allowlisted(&self, command: &str) -> bool {
    if Self::contains_shell_metacharacters(command) {
        return false;
    }
    let Ok((program, _)) = Self::parse_command(command) else {
        return false;
    };
    self.resolve_allowlisted_executable(&program).is_ok()
}
```

Update remaining safe-result assertions to destructure `ValidationResult::Safe { executable }`; for strict allowlist success, assert that `executable` equals `std::fs::canonicalize(requested)`. Update `test_is_allowlisted` to use the canonical current test executable and assert that a shell-syntax suffix, a bare basename, and a different same-basename path return false. Keep `test_program_basename` only as a non-security string-utility test.

- [ ] **Step 6: Carry the authorized path into both process constructors**

Add the private launch decision beside `IoClient` in `src/interpreter/mod.rs`:

```rust
enum AuthorizedSubprocess {
    Shell,
    Direct { executable: PathBuf },
}
```

Change `authorize_subprocess` to return it and eliminate first-token parsing:

```rust
fn authorize_subprocess(
    &self,
    command: &str,
    args: &[&str],
    use_shell: bool,
    line: usize,
    column: usize,
) -> Result<AuthorizedSubprocess, String> {
    use crate::interpreter::command_sanitizer::{CommandSanitizer, ValidationResult};

    let needs_shell = use_shell
        || (args.is_empty() && CommandSanitizer::contains_shell_metacharacters(command));
    let program = if needs_shell {
        String::new()
    } else if args.is_empty() {
        CommandSanitizer::parse_command(command)?.0
    } else {
        command.to_string()
    };

    let sanitizer = CommandSanitizer::new(Arc::clone(&self.config));
    match sanitizer.authorize_process_execution(&program, needs_shell, command)? {
        ValidationResult::Safe { executable } => {
            Ok(AuthorizedSubprocess::Direct { executable })
        }
        ValidationResult::RequiresShell { warnings, .. } => {
            if self.config.warn_on_shell_execution {
                eprintln!("⚠️  Security Warning (line {}, column {}):", line, column);
                eprintln!("   Shell execution enabled for command: {}", command);
                for warning in warnings {
                    eprintln!("   - {}", warning);
                }
                eprintln!(
                    "   Prefer direct execution with an argument list when shell syntax is unnecessary."
                );
            }
            Ok(AuthorizedSubprocess::Shell)
        }
        ValidationResult::Blocked { reason } => Err(format!(
            "Command blocked by security policy: {}\n\
             Subprocess execution is disabled by default. To allow direct execution, update .wflcfg:\n\
               allow_shell_execution = true\n\
               shell_execution_mode = allowlist_only\n\
               allowed_shell_commands = <absolute executable path>\n\
             (line {}, column {})",
            reason, line, column
        )),
    }
}
```

In both `execute_command` and `spawn_process`, match the returned decision. Keep the existing platform shell construction only in the `Shell` arm. In the direct arm, parse only the arguments and pass the returned executable to `Command::new`:

```rust
let authorization = self.authorize_subprocess(command, args, use_shell, line, column)?;
let mut cmd = match authorization {
    AuthorizedSubprocess::Shell => {
        #[cfg(target_os = "windows")]
        {
            let mut cmd = Command::new("cmd.exe");
            cmd.args(["/C", command]);
            cmd
        }
        #[cfg(not(target_os = "windows"))]
        {
            let mut cmd = Command::new("sh");
            cmd.args(["-c", command]);
            cmd
        }
    }
    AuthorizedSubprocess::Direct { executable } => {
        let parsed_args = if args.is_empty() {
            CommandSanitizer::parse_command(command)?.1
        } else {
            args.iter().map(|value| value.to_string()).collect()
        };
        let mut cmd = Command::new(executable);
        cmd.args(parsed_args);
        cmd
    }
};
```

- [ ] **Step 7: Verify GREEN for policy and release-binary regressions**

Run:

```powershell
cargo fmt --all
cargo test --lib interpreter::command_sanitizer::tests -- --nocapture
cargo test --lib interpreter::process_tests -- --nocapture
cargo build --release
cargo test --test subprocess_security_test -- --nocapture
cargo test --test subprocess_test -- --nocapture
cargo test --test subprocess_cleanup_test -- --nocapture
```

Expected: all listed tests pass. The new exploit regressions report policy blocks before any secondary marker or alternate executable runs. Existing sanitized/unrestricted tests remain green.

- [ ] **Step 8: Commit Task 1**

```powershell
git add -- src/interpreter/command_sanitizer.rs src/interpreter/mod.rs tests/subprocess_security_test.rs
git commit -m "fix(security): bind subprocess allowlist to executable paths"
```

### Task 2: Validate and explain strict allowlist configuration

**Files:**
- Modify: `src/wfl_config/checker.rs:299-345,566-768,949-1071`

**Interfaces:**
- Consumes: raw `.wflcfg` text and the existing `ConfigIssue` model.
- Produces: an `InvalidValue` error for each non-absolute `allowed_shell_commands` entry when the effective file-local mode is `allowlist_only`; absolute entries remain valid.

- [ ] **Step 1: Write failing configuration-checker tests**

Add these tests to `src/wfl_config/checker.rs`:

```rust
#[test]
fn test_allowlist_only_rejects_relative_allowed_command() {
    let checker = ConfigChecker::new();
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join(".wflcfg");
    fs::write(
        &config_path,
        "shell_execution_mode = allowlist_only\nallowed_shell_commands = echo\n",
    )
    .unwrap();

    let issues = checker.check_config_file(&config_path).unwrap();
    assert!(
        issues.iter().any(|issue| {
            issue.kind == ConfigIssueKind::InvalidValue
                && issue.setting_name.as_deref() == Some("allowed_shell_commands")
                && issue.message.contains("absolute executable path")
        }),
        "relative allowlist entry must be invalid: {issues:?}"
    );
}

#[test]
fn test_allowlist_only_accepts_absolute_allowed_command() {
    let checker = ConfigChecker::new();
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join(".wflcfg");
    let executable = std::env::current_exe().unwrap();
    fs::write(
        &config_path,
        format!(
            "shell_execution_mode = allowlist_only\nallowed_shell_commands = {}\n",
            executable.display()
        ),
    )
    .unwrap();

    let issues = checker.check_config_file(&config_path).unwrap();
    assert!(issues.is_empty(), "absolute allowlist entry should be valid: {issues:?}");
}
```

- [ ] **Step 2: Run the checker tests and verify RED**

Run:

```powershell
cargo test --lib wfl_config::checker::tests::test_allowlist_only -- --nocapture
```

Expected: the relative-entry test fails because `StringList` currently performs no value validation. The absolute-entry control passes.

- [ ] **Step 3: Implement file-local mode detection and path validation**

Add these private helpers near the other checker helpers:

```rust
fn strip_optional_quotes(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}

fn file_uses_allowlist_only(content: &str) -> bool {
    let mut allowlist_only = false;
    for line in content.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=')
            && key.trim() == "shell_execution_mode"
        {
            allowlist_only = matches!(
                value
                    .split('#')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_ascii_lowercase()
                    .as_str(),
                "allowlist_only" | "allowlistonly"
            );
        }
    }
    allowlist_only
}
```

At the start of `check_config_file`, after reading `content`, compute:

```rust
let allowlist_only = file_uses_allowlist_only(&content);
```

Replace the empty `ConfigType::StringList` branch with:

```rust
ConfigType::StringList => {
    if key == "allowed_shell_commands" && allowlist_only {
        for entry in value.split(',').map(str::trim).filter(|entry| !entry.is_empty()) {
            let path_text = strip_optional_quotes(entry);
            if !Path::new(path_text).is_absolute() {
                issues.push(ConfigIssue {
                    file_path: file_path.to_path_buf(),
                    kind: ConfigIssueKind::InvalidValue,
                    issue_type: ConfigIssueType::Error,
                    message: format!(
                        "Invalid allowed_shell_commands entry '{entry}': allowlist_only requires an absolute executable path"
                    ),
                    setting_name: Some(key.to_string()),
                    line_number: Some(line_number + 1),
                    fix_message: Some(
                        "Replace the entry with the executable's canonical absolute path"
                            .to_string(),
                    ),
                });
            }
        }
    }
}
```

Change the expected-setting description to:

```rust
description: "Comma-separated absolute executable paths allowed for direct execution in allowlist_only mode".to_string(),
```

- [ ] **Step 4: Verify GREEN and checker regression coverage**

Run:

```powershell
cargo fmt --all
cargo test --lib wfl_config::checker::tests -- --nocapture
cargo test --lib config::tests -- --nocapture
```

Expected: all checker and configuration tests pass, with one `InvalidValue` for the relative test and no issues for the absolute test.

- [ ] **Step 5: Commit Task 2**

```powershell
git add -- src/wfl_config/checker.rs
git commit -m "fix(config): validate strict subprocess allowlist paths"
```

### Task 3: Ship strict-allowlist migration and security guidance

**Files:**
- Modify: `Docs/04-advanced-features/subprocess-execution.md:5-39,295-350`
- Modify: `Docs/06-best-practices/security-guidelines.md:9-37,252-269`
- Modify: `Docs/reference/configuration-reference.md:194-202,342-402,543-553`
- Modify: `CHANGELOG.md:7-28`
- Create: `Dev diary/2026-07-12-allowlist-subprocess-hardening.md`

**Interfaces:**
- Consumes: the implemented Task 1 and Task 2 behavior and runtime error vocabulary.
- Produces: one consistent user contract: strict mode is direct-only, both sides use canonical absolute paths, and interpreters are full capabilities rather than safe argument relays.

- [ ] **Step 1: Update the configuration reference as the source of truth**

Make these exact semantic changes throughout `Docs/reference/configuration-reference.md`:

```markdown
- `allowlist_only` — direct execution only; the requested executable must be an absolute path whose canonical identity matches an absolute path in `allowed_shell_commands`. Shell syntax and `using shell` are blocked.
```

Replace the basename example with platform-specific absolute examples:

```ini
# Unix example
allowed_shell_commands = /usr/bin/git, /usr/bin/printf

# Windows example
allowed_shell_commands = C:\Windows\System32\where.exe
```

State explicitly that entries which are bare, relative, unresolved, or merely share a basename cannot authorize a launch; Windows canonical identity comparison is case-insensitive. Explain that intentional shell syntax requires `sanitized` or `unrestricted` and that allowlisting an interpreter grants its full capabilities.

- [ ] **Step 2: Update the subprocess guide and security guide**

In `Docs/04-advanced-features/subprocess-execution.md`, replace every claim that basenames authorize strict mode. Add a migration subsection containing these exact mappings:

```markdown
- `allowed_shell_commands = git` → use the canonical absolute Git executable path.
- `execute command "git" with arguments ["status"]` → use that same absolute path and keep `["status"]` as a separate argument list.
- A shell command string under `allowlist_only` → use direct `with arguments`, or deliberately change to a shell-capable mode if shell semantics are required.
```

Keep bare beginner examples under the already documented `sanitized` opt-in, and make every strict-mode example use an absolute path.

In `Docs/06-best-practices/security-guidelines.md`, delete the recommendation to pass untrusted text through `cmd.exe /C`. Replace it with this guidance:

```markdown
On every platform, use a dedicated executable by absolute path and pass untrusted data as a separate argv item. Do not use `sh -c`, `cmd.exe /C`, PowerShell command strings, or another interpreter as an argument-safety wrapper; allowlisting an interpreter grants its normal code-execution capabilities. When the task can be performed by WFL itself, prefer the built-in WFL operation and avoid a subprocess entirely.
```

- [ ] **Step 3: Add changelog and Dev Diary records**

Under `CHANGELOG.md` → `Unreleased` → `Security`, add bullets that state:

```markdown
- **`allowlist_only` is now direct-execution only.** Commands that require shell parsing, including explicit `using shell`, are rejected instead of authorizing the first token and passing the full string to `sh -c` or `cmd.exe /C`.
- **Allowlisted executables now use canonical absolute identity.** Bare names, relative paths, and alternate same-basename paths no longer match. Update both `allowed_shell_commands` and the WFL command expression to the executable's absolute path.
```

Create `Dev diary/2026-07-12-allowlist-subprocess-hardening.md` with these sections and concrete content:

```markdown
# Dev Diary — Allowlist subprocess hardening (2026-07-12)

## Summary

`allowlist_only` now authorizes only direct launches of canonical absolute executable paths. It no longer authorizes a first token and then sends the complete command string to a shell, and it no longer collapses paths to basenames.

## Root cause

Shell-needed commands were authorized from only their first whitespace token, while execution passed the untouched string to `sh -c` or `cmd.exe /C`. Direct commands compared only executable basenames, so an alternate path could impersonate an allowed program.

## Resolution

- Strict allowlist mode rejects inferred and explicit shell execution.
- Requested and configured executable paths must be absolute and resolve to the same canonical file.
- Authorization returns the canonical path used by process creation.
- Execute and spawn share the same policy decision.

## Migration

Replace basename entries and bare command expressions with the executable's canonical absolute path. Use `with arguments` for data. Choose a shell-capable mode only when shell semantics are intentional, and treat an allowlisted interpreter as a full code-execution capability.

## Verification

Unit and release-binary regressions cover shell chaining, execute and spawn paths, same-basename spoofing, successful exact-path argv execution, and configuration diagnostics. Full quality gates were run, with the pre-existing Windows `wflpkg` assertion-message mismatch recorded separately.
```

- [ ] **Step 4: Validate documentation and inspect the migration diff**

Run:

```powershell
python scripts/validate_docs_examples.py
python scripts/test_docs_code_blocks.py
rg -n "basename|allowlist_only|allowed_shell_commands|cmd\.exe /C|cmd /c|sh -c" Docs README.md CHANGELOG.md "Dev diary"
git diff --check
```

Expected: both documentation validators pass; remaining basename references describe the old behavior only in the changelog/Dev Diary root-cause context; no guidance recommends `cmd.exe /C` or `sh -c` for untrusted data; the diff has no whitespace errors. The WFL LSP MCP validators are unavailable in this session, so local repository validators are the required fallback evidence.

- [ ] **Step 5: Commit Task 3**

```powershell
git add -- CHANGELOG.md Docs/04-advanced-features/subprocess-execution.md Docs/06-best-practices/security-guidelines.md Docs/reference/configuration-reference.md "Dev diary/2026-07-12-allowlist-subprocess-hardening.md"
git commit -m "docs: document strict subprocess allowlist migration"
```

## Final controller verification

After all three task reviews are clean, the controller runs fresh verification rather than relying on subagent reports:

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --verbose
cargo build --release
./scripts/run_integration_tests.ps1
python scripts/validate_docs_examples.py
python scripts/test_docs_code_blocks.py
git diff --check 49f3373..HEAD
git status --short --branch
```

The full test result is compared to the recorded baseline. The unrelated `wflpkg` assertion-message mismatch may remain, but every WFL subprocess test and every test changed by this plan must pass, and no new failure is acceptable.
