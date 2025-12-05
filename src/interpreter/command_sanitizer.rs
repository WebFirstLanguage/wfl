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
    /// Example: "echo hello world" -> ("echo", ["hello", "world"])
    pub fn parse_command(command: &str) -> Result<(String, Vec<String>), String> {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return Err("Empty command".to_string());
        }

        // Split by whitespace, respecting quotes
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let chars = trimmed.chars().peekable();

        for ch in chars {
            match ch {
                '"' if !in_quotes => {
                    in_quotes = true;
                }
                '"' if in_quotes => {
                    in_quotes = false;
                }
                ' ' | '\t' if !in_quotes => {
                    if !current.is_empty() {
                        parts.push(current.clone());
                        current.clear();
                    }
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.is_empty() {
            parts.push(current);
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
