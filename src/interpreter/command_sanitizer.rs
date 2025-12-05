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

    /// Validate command against security policy
    pub fn validate_command(&self, command: &str) -> Result<ValidationResult, String> {
        // Check if command contains shell features
        let has_shell_features = Self::contains_shell_metacharacters(command);

        if !has_shell_features {
            return Ok(ValidationResult::Safe);
        }

        // Command has shell features - check policy
        match self.config.shell_execution_mode {
            ShellExecutionMode::Forbidden => Ok(ValidationResult::Blocked {
                reason: "Shell execution is disabled by security policy".to_string(),
            }),
            ShellExecutionMode::AllowlistOnly => {
                if self.is_allowlisted(command) {
                    Ok(ValidationResult::RequiresShell {
                        reason: "Command is allowlisted".to_string(),
                        warnings: vec!["Using shell execution (allowlisted)".to_string()],
                    })
                } else {
                    Ok(ValidationResult::Blocked {
                        reason: format!(
                            "Command '{}' is not in the allowlist",
                            self.get_command_base(command)
                        ),
                    })
                }
            }
            ShellExecutionMode::Sanitized => {
                let warnings = self.analyze_shell_features(command);
                Ok(ValidationResult::RequiresShell {
                    reason: "Command contains shell features".to_string(),
                    warnings,
                })
            }
            ShellExecutionMode::Unrestricted => Ok(ValidationResult::RequiresShell {
                reason: "Unrestricted shell mode enabled".to_string(),
                warnings: vec!["⚠️ Using unrestricted shell execution".to_string()],
            }),
        }
    }

    /// Check if command is in the allowlist
    pub fn is_allowlisted(&self, command: &str) -> bool {
        let base_command = self.get_command_base(command);

        self.config
            .allowed_shell_commands
            .iter()
            .any(|allowed| allowed == &base_command)
    }

    /// Extract the base command from a command string
    fn get_command_base(&self, command: &str) -> String {
        command.split_whitespace().next().unwrap_or("").to_string()
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
            shell_execution_mode: mode,
            ..Default::default()
        };
        Arc::new(config)
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
    fn test_validate_command_safe() {
        let sanitizer = CommandSanitizer::new(test_config(ShellExecutionMode::Forbidden));
        let result = sanitizer.validate_command("echo hello").unwrap();
        assert_eq!(result, ValidationResult::Safe);
    }

    #[test]
    fn test_validate_command_blocked_forbidden() {
        let sanitizer = CommandSanitizer::new(test_config(ShellExecutionMode::Forbidden));
        let result = sanitizer.validate_command("echo $HOME").unwrap();
        assert!(matches!(result, ValidationResult::Blocked { .. }));
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
    fn test_is_allowlisted() {
        let config = WflConfig {
            allowed_shell_commands: vec!["echo".to_string(), "ls".to_string()],
            ..Default::default()
        };
        let sanitizer = CommandSanitizer::new(Arc::new(config));

        assert!(sanitizer.is_allowlisted("echo hello"));
        assert!(sanitizer.is_allowlisted("ls -la"));
        assert!(!sanitizer.is_allowlisted("rm -rf /"));
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
