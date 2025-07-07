use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(not(windows))]
const DEFAULT_GLOBAL_CONFIG_PATH: &str = "/etc/wfl/wfl.cfg";

#[cfg(windows)]
const DEFAULT_GLOBAL_CONFIG_PATH: &str = r"C:\wfl\config";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigType {
    String,
    Integer,
    Boolean,
    LogLevel,
}

impl fmt::Display for ConfigType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigType::String => write!(f, "string"),
            ConfigType::Integer => write!(f, "integer"),
            ConfigType::Boolean => write!(f, "boolean"),
            ConfigType::LogLevel => write!(f, "log level"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigIssueKind {
    MissingFile,
    MissingSetting,
    InvalidType,
    InvalidValue,
    UnknownKey,
}

impl fmt::Display for ConfigIssueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigIssueKind::MissingFile => write!(f, "missing file"),
            ConfigIssueKind::MissingSetting => write!(f, "missing setting"),
            ConfigIssueKind::InvalidType => write!(f, "invalid type"),
            ConfigIssueKind::InvalidValue => write!(f, "invalid value"),
            ConfigIssueKind::UnknownKey => write!(f, "unknown key"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigIssueType {
    Error,
    Warning,
}

impl fmt::Display for ConfigIssueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigIssueType::Error => write!(f, "error"),
            ConfigIssueType::Warning => write!(f, "warning"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigIssue {
    pub file_path: PathBuf,
    pub kind: ConfigIssueKind,
    pub issue_type: ConfigIssueType,
    pub message: String,
    pub setting_name: Option<String>,
    pub line_number: Option<usize>,
    pub fix_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExpectedSetting {
    pub name: String,
    pub config_type: ConfigType,
    pub required: bool,
    pub default_value: Option<String>,
    pub description: String,
    pub valid_values: Option<Vec<String>>,
}

pub struct ConfigChecker {
    expected_settings: HashMap<String, ExpectedSetting>,
}

impl Default for ConfigChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigChecker {
    pub fn new() -> Self {
        let mut expected_settings = HashMap::new();

        expected_settings.insert(
            "timeout_seconds".to_string(),
            ExpectedSetting {
                name: "timeout_seconds".to_string(),
                config_type: ConfigType::Integer,
                required: false,
                default_value: Some("60".to_string()),
                description: "Maximum execution time (minimum 1s)".to_string(),
                valid_values: None,
            },
        );

        expected_settings.insert(
            "logging_enabled".to_string(),
            ExpectedSetting {
                name: "logging_enabled".to_string(),
                config_type: ConfigType::Boolean,
                required: false,
                default_value: Some("false".to_string()),
                description: "Enables log output to wfl.log".to_string(),
                valid_values: None,
            },
        );

        expected_settings.insert(
            "debug_report_enabled".to_string(),
            ExpectedSetting {
                name: "debug_report_enabled".to_string(),
                config_type: ConfigType::Boolean,
                required: false,
                default_value: Some("true".to_string()),
                description: "Enables debug reports on runtime errors".to_string(),
                valid_values: None,
            },
        );

        expected_settings.insert(
            "log_level".to_string(),
            ExpectedSetting {
                name: "log_level".to_string(),
                config_type: ConfigType::LogLevel,
                required: false,
                default_value: Some("info".to_string()),
                description: "Log verbosity".to_string(),
                valid_values: Some(vec![
                    "debug".to_string(),
                    "info".to_string(),
                    "warn".to_string(),
                    "error".to_string(),
                ]),
            },
        );

        expected_settings.insert(
            "execution_logging".to_string(),
            ExpectedSetting {
                name: "execution_logging".to_string(),
                config_type: ConfigType::Boolean,
                required: false,
                default_value: Some("false".to_string()),
                description: "Enables execution logging".to_string(),
                valid_values: None,
            },
        );

        expected_settings.insert(
            "max_line_length".to_string(),
            ExpectedSetting {
                name: "max_line_length".to_string(),
                config_type: ConfigType::Integer,
                required: false,
                default_value: Some("100".to_string()),
                description: "Maximum line length".to_string(),
                valid_values: None,
            },
        );

        expected_settings.insert(
            "max_nesting_depth".to_string(),
            ExpectedSetting {
                name: "max_nesting_depth".to_string(),
                config_type: ConfigType::Integer,
                required: false,
                default_value: Some("5".to_string()),
                description: "Maximum control structure depth".to_string(),
                valid_values: None,
            },
        );

        expected_settings.insert(
            "indent_size".to_string(),
            ExpectedSetting {
                name: "indent_size".to_string(),
                config_type: ConfigType::Integer,
                required: false,
                default_value: Some("4".to_string()),
                description: "Spaces per indent level".to_string(),
                valid_values: None,
            },
        );

        expected_settings.insert(
            "snake_case_variables".to_string(),
            ExpectedSetting {
                name: "snake_case_variables".to_string(),
                config_type: ConfigType::Boolean,
                required: false,
                default_value: Some("true".to_string()),
                description: "Enforce snake_case variable names".to_string(),
                valid_values: None,
            },
        );

        expected_settings.insert(
            "trailing_whitespace".to_string(),
            ExpectedSetting {
                name: "trailing_whitespace".to_string(),
                config_type: ConfigType::Boolean,
                required: false,
                default_value: Some("false".to_string()),
                description: "Allow trailing whitespace".to_string(),
                valid_values: None,
            },
        );

        expected_settings.insert(
            "consistent_keyword_case".to_string(),
            ExpectedSetting {
                name: "consistent_keyword_case".to_string(),
                config_type: ConfigType::Boolean,
                required: false,
                default_value: Some("true".to_string()),
                description: "Require consistent keyword casing".to_string(),
                valid_values: None,
            },
        );

        Self { expected_settings }
    }

    pub fn get_global_config_path() -> PathBuf {
        std::env::var("WFL_GLOBAL_CONFIG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_GLOBAL_CONFIG_PATH))
    }

    pub fn find_config_files(&self, start_dir: &Path) -> Vec<PathBuf> {
        let mut config_files = Vec::new();
        let global_path = Self::get_global_config_path();

        if global_path.exists() {
            config_files.push(global_path);
        }

        let mut current_dir = Some(start_dir.to_path_buf());
        while let Some(dir) = current_dir {
            let config_path = dir.join(".wflcfg");
            if config_path.exists() {
                config_files.push(config_path);
            }
            current_dir = dir.parent().map(Path::to_path_buf);
        }

        config_files
    }

    pub fn check_config_file(&self, file_path: &Path) -> Result<Vec<ConfigIssue>, io::Error> {
        if !file_path.exists() {
            return Ok(vec![ConfigIssue {
                file_path: file_path.to_path_buf(),
                kind: ConfigIssueKind::MissingFile,
                issue_type: ConfigIssueType::Error,
                message: format!("Config file not found: {}", file_path.display()),
                setting_name: None,
                line_number: None,
                fix_message: Some("Create the file with default settings".to_string()),
            }]);
        }

        let content = fs::read_to_string(file_path)?;
        let mut issues = Vec::new();

        for (line_number, line) in content.lines().enumerate() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(pos) = line.find('=') {
                let key = line[..pos].trim();
                let value = line[pos + 1..].trim();

                if !self.expected_settings.contains_key(key) {
                    issues.push(ConfigIssue {
                        file_path: file_path.to_path_buf(),
                        kind: ConfigIssueKind::UnknownKey,
                        issue_type: ConfigIssueType::Warning,
                        message: format!("Unknown configuration key: {}", key),
                        setting_name: Some(key.to_string()),
                        line_number: Some(line_number + 1),
                        fix_message: Some("Remove the line or correct the key name".to_string()),
                    });
                    continue;
                }

                let setting = &self.expected_settings[key];

                match setting.config_type {
                    ConfigType::Integer => {
                        if value.parse::<i64>().is_err() {
                            issues.push(ConfigIssue {
                                file_path: file_path.to_path_buf(),
                                kind: ConfigIssueKind::InvalidType,
                                issue_type: ConfigIssueType::Error,
                                message: format!(
                                    "Invalid type for {}: expected integer, got '{}'",
                                    key, value
                                ),
                                setting_name: Some(key.to_string()),
                                line_number: Some(line_number + 1),
                                fix_message: setting
                                    .default_value
                                    .as_ref()
                                    .map(|default| format!("Set to default value: {}", default)),
                            });
                        }
                    }
                    ConfigType::Boolean => {
                        if value != "true" && value != "false" {
                            issues.push(ConfigIssue {
                                file_path: file_path.to_path_buf(),
                                kind: ConfigIssueKind::InvalidType,
                                issue_type: ConfigIssueType::Error,
                                message: format!(
                                    "Invalid type for {}: expected boolean (true/false), got '{}'",
                                    key, value
                                ),
                                setting_name: Some(key.to_string()),
                                line_number: Some(line_number + 1),
                                fix_message: setting
                                    .default_value
                                    .as_ref()
                                    .map(|default| format!("Set to default value: {}", default)),
                            });
                        }
                    }
                    ConfigType::LogLevel => {
                        if let Some(valid_values) = &setting.valid_values {
                            if !valid_values.contains(&value.to_string()) {
                                issues.push(ConfigIssue {
                                    file_path: file_path.to_path_buf(),
                                    kind: ConfigIssueKind::InvalidValue,
                                    issue_type: ConfigIssueType::Error,
                                    message: format!(
                                        "Invalid value for {}: expected one of {:?}, got '{}'",
                                        key, valid_values, value
                                    ),
                                    setting_name: Some(key.to_string()),
                                    line_number: Some(line_number + 1),
                                    fix_message: setting.default_value.as_ref().map(|default| {
                                        format!("Set to default value: {}", default)
                                    }),
                                });
                            }
                        }
                    }
                    ConfigType::String => {
                        if let Some(valid_values) = &setting.valid_values {
                            if !valid_values.contains(&value.to_string()) {
                                issues.push(ConfigIssue {
                                    file_path: file_path.to_path_buf(),
                                    kind: ConfigIssueKind::InvalidValue,
                                    issue_type: ConfigIssueType::Error,
                                    message: format!(
                                        "Invalid value for {}: expected one of {:?}, got '{}'",
                                        key, valid_values, value
                                    ),
                                    setting_name: Some(key.to_string()),
                                    line_number: Some(line_number + 1),
                                    fix_message: setting.default_value.as_ref().map(|default| {
                                        format!("Set to default value: {}", default)
                                    }),
                                });
                            }
                        }
                    }
                }
            } else {
                issues.push(ConfigIssue {
                    file_path: file_path.to_path_buf(),
                    kind: ConfigIssueKind::InvalidType,
                    issue_type: ConfigIssueType::Error,
                    message: format!(
                        "Invalid format in line {}: expected 'key = value'",
                        line_number + 1
                    ),
                    setting_name: None,
                    line_number: Some(line_number + 1),
                    fix_message: Some("Remove the line or fix the format".to_string()),
                });
            }
        }

        for (key, setting) in &self.expected_settings {
            if setting.required {
                let mut found = false;
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with(key) && line.contains('=') {
                        found = true;
                        break;
                    }
                }

                if !found {
                    issues.push(ConfigIssue {
                        file_path: file_path.to_path_buf(),
                        kind: ConfigIssueKind::MissingSetting,
                        issue_type: ConfigIssueType::Error,
                        message: format!("Missing required setting: {}", key),
                        setting_name: Some(key.to_string()),
                        line_number: None,
                        fix_message: setting.default_value.as_ref().map(|default| {
                            format!("Add '{}' with default value: {}", key, default)
                        }),
                    });
                }
            }
        }

        Ok(issues)
    }

    pub fn fix_config_file(&self, file_path: &Path) -> Result<Vec<ConfigIssue>, io::Error> {
        let issues = self.check_config_file(file_path)?;

        if issues
            .iter()
            .any(|issue| issue.kind == ConfigIssueKind::MissingFile)
        {
            let mut content = String::new();
            content.push_str("# WebFirst Language Configuration File\n");
            content.push_str("# Automatically generated by wfl --configFix\n\n");

            for setting in self.expected_settings.values() {
                if let Some(default) = &setting.default_value {
                    content.push_str(&format!("# {}\n", setting.description));
                    content.push_str(&format!("{} = {}\n\n", setting.name, default));
                }
            }

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::write(file_path, content)?;
            println!("✅ Created config file: {}", file_path.display());

            return self.check_config_file(file_path);
        }

        if issues.is_empty() {
            return Ok(issues);
        }

        let content = fs::read_to_string(file_path)?;

        let mut lines: Vec<String> = content.lines().map(ToString::to_string).collect();
        let mut added_settings = HashMap::new();

        for issue in &issues {
            match issue.kind {
                ConfigIssueKind::UnknownKey => {
                    if let Some(line_number) = issue.line_number {
                        if line_number <= lines.len() {
                            lines[line_number - 1] =
                                format!("# {} (unknown key)", lines[line_number - 1]);
                            println!("✅ Commented out unknown key at line {}", line_number);
                        }
                    }
                }
                ConfigIssueKind::InvalidType | ConfigIssueKind::InvalidValue => {
                    if let (Some(line_number), Some(setting_name)) =
                        (issue.line_number, &issue.setting_name)
                    {
                        if line_number <= lines.len() {
                            if let Some(setting) = self.expected_settings.get(setting_name) {
                                if let Some(default_value) = &setting.default_value {
                                    lines[line_number - 1] =
                                        format!("{} = {}", setting_name, default_value);
                                    println!(
                                        "✅ Fixed value for '{}' at line {}",
                                        setting_name, line_number
                                    );
                                }
                            }
                        }
                    }
                }
                ConfigIssueKind::MissingSetting => {
                    if let Some(setting_name) = &issue.setting_name {
                        if let Some(setting) = self.expected_settings.get(setting_name) {
                            if let Some(default_value) = &setting.default_value {
                                if !added_settings.contains_key(setting_name) {
                                    lines.push(String::new());
                                    lines.push(format!("# {}", setting.description));
                                    lines.push(format!("{} = {}", setting_name, default_value));
                                    added_settings.insert(setting_name.clone(), true);
                                    println!("✅ Added missing setting: {}", setting_name);
                                }
                            }
                        }
                    }
                }
                _ => {} // Other issues handled elsewhere
            }
        }

        let updated_content = lines.join("\n");
        fs::write(file_path, updated_content)?;

        self.check_config_file(file_path)
    }

    pub fn print_report(&self, issues: &[ConfigIssue], fix_mode: bool) {
        if issues.is_empty() {
            println!("✅ No configuration issues found!");
            return;
        }

        let mut error_count = 0;
        let mut warning_count = 0;

        for issue in issues {
            match issue.issue_type {
                ConfigIssueType::Error => {
                    error_count += 1;
                    println!("❌ Error: {}", issue.message);
                }
                ConfigIssueType::Warning => {
                    warning_count += 1;
                    println!("⚠️  Warning: {}", issue.message);
                }
            }

            if let Some(line_number) = issue.line_number {
                println!(
                    "   File: {} (line {})",
                    issue.file_path.display(),
                    line_number
                );
            } else {
                println!("   File: {}", issue.file_path.display());
            }

            if let Some(fix_message) = &issue.fix_message {
                if fix_mode {
                    println!("   Fix: {}", fix_message);
                } else {
                    println!("   Suggested fix: {}", fix_message);
                }
            }

            println!();
        }

        println!(
            "{} errors, {} warnings found in configuration files",
            error_count, warning_count
        );

        if !fix_mode && error_count > 0 {
            println!("\n🛠️  Run 'wfl --configFix' to automatically fix these issues");
        }
    }
}

pub fn check_config(dir: &Path) -> Result<(Vec<ConfigIssue>, bool), io::Error> {
    let checker = ConfigChecker::new();
    let config_files = checker.find_config_files(dir);

    let mut all_issues = Vec::new();

    for file_path in config_files {
        let issues = checker.check_config_file(&file_path)?;
        all_issues.extend(issues);
    }

    let has_errors = all_issues
        .iter()
        .any(|issue| issue.issue_type == ConfigIssueType::Error);

    checker.print_report(&all_issues, false);

    Ok((all_issues, !has_errors))
}

pub fn fix_config(dir: &Path) -> Result<(Vec<ConfigIssue>, bool), io::Error> {
    let checker = ConfigChecker::new();
    let config_files = checker.find_config_files(dir);

    let mut all_issues = Vec::new();

    for file_path in config_files {
        let issues = checker.fix_config_file(&file_path)?;
        all_issues.extend(issues);
    }

    let has_errors = all_issues
        .iter()
        .any(|issue| issue.issue_type == ConfigIssueType::Error);

    checker.print_report(&all_issues, true);

    Ok((all_issues, !has_errors))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_check_valid_config() {
        let checker = ConfigChecker::new();
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        let config_content = r#"
# Valid configuration
timeout_seconds = 30
logging_enabled = true
log_level = info
max_line_length = 80
"#;

        fs::write(&config_path, config_content).unwrap();

        let issues = checker.check_config_file(&config_path).unwrap();
        assert!(issues.is_empty(), "Expected no issues, got: {issues:?}");
    }

    #[test]
    fn test_check_missing_file() {
        let checker = ConfigChecker::new();
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("nonexistent.cfg");

        let issues = checker.check_config_file(&config_path).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, ConfigIssueKind::MissingFile);
    }

    #[test]
    fn test_check_invalid_type() {
        let checker = ConfigChecker::new();
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        let config_content = r#"
# Invalid type
timeout_seconds = potato
"#;

        fs::write(&config_path, config_content).unwrap();

        let issues = checker.check_config_file(&config_path).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, ConfigIssueKind::InvalidType);
        assert_eq!(issues[0].setting_name, Some("timeout_seconds".to_string()));
    }

    #[test]
    fn test_check_unknown_key() {
        let checker = ConfigChecker::new();
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        let config_content = r#"
# Unknown key
unknown_setting = value
"#;

        fs::write(&config_path, config_content).unwrap();

        let issues = checker.check_config_file(&config_path).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, ConfigIssueKind::UnknownKey);
        assert_eq!(issues[0].setting_name, Some("unknown_setting".to_string()));
    }

    #[test]
    fn test_fix_missing_file() {
        let checker = ConfigChecker::new();
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        assert!(!config_path.exists());

        let issues = checker.fix_config_file(&config_path).unwrap();

        assert!(config_path.exists());

        assert!(
            issues.is_empty(),
            "Expected no issues after fix, got: {issues:?}"
        );
    }

    #[test]
    fn test_fix_invalid_type() {
        let checker = ConfigChecker::new();
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        let config_content = r#"
# Invalid type
timeout_seconds = potato
"#;

        fs::write(&config_path, config_content).unwrap();

        let issues_before = checker.check_config_file(&config_path).unwrap();
        assert_eq!(issues_before.len(), 1);
        assert_eq!(issues_before[0].kind, ConfigIssueKind::InvalidType);

        let issues_after = checker.fix_config_file(&config_path).unwrap();

        assert!(
            issues_after.is_empty(),
            "Expected no issues after fix, got: {issues_after:?}"
        );

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(
            content.contains("timeout_seconds = 60"),
            "File content after fix: {content}"
        );
    }
}
