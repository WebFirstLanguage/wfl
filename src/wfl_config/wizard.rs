use rustyline::DefaultEditor;
use std::collections::HashMap;
use std::io;
use std::path::Path;

use super::checker::{ConfigChecker, ConfigType, ExpectedSetting};

pub struct ConfigWizard {
    editor: DefaultEditor,
    checker: ConfigChecker,
    values: HashMap<String, String>,
}

impl ConfigWizard {
    pub fn new() -> Result<Self, io::Error> {
        let editor = DefaultEditor::new()
            .map_err(|e| io::Error::other(format!("Failed to create editor: {e}")))?;

        Ok(Self {
            editor,
            checker: ConfigChecker::new(),
            values: HashMap::new(),
        })
    }

    pub fn run(&mut self, output_path: &Path) -> Result<(), io::Error> {
        println!("\nWebFirst Language Configuration Wizard");
        println!("======================================\n");
        println!(
            "This wizard will help you create a WFL configuration file with all configuration options."
        );
        println!(
            "Press Enter to accept the default value shown in brackets, or type a new value.\n"
        );
        println!("Optional settings without a default can be skipped by pressing Enter.\n");

        // Get settings grouped by category - collect into owned data to avoid borrow issues
        let categories: Vec<(String, Vec<ExpectedSetting>)> = self
            .checker
            .get_settings_by_category()
            .into_iter()
            .map(|(cat, settings)| (cat, settings.into_iter().cloned().collect()))
            .collect();

        for (category_name, settings) in categories {
            self.prompt_category(&category_name, settings)?;
        }

        // Generate the config file
        self.generate_file(output_path)?;

        Ok(())
    }

    fn prompt_category(
        &mut self,
        category: &str,
        mut settings: Vec<ExpectedSetting>,
    ) -> Result<(), io::Error> {
        // Sort settings alphabetically within category for consistency
        settings.sort_by(|a, b| a.name.cmp(&b.name));

        println!(
            "================================================================================"
        );
        println!("{}", category);
        println!(
            "================================================================================\n"
        );

        for setting in &settings {
            if let Some(value) = self.prompt_setting(setting)? {
                self.values.insert(setting.name.clone(), value);
            } else {
                self.values.remove(&setting.name);
            }
        }

        println!();
        Ok(())
    }

    fn prompt_setting(&mut self, setting: &ExpectedSetting) -> Result<Option<String>, io::Error> {
        let prompt = self.format_prompt(setting);

        loop {
            let line = self
                .editor
                .readline(&prompt)
                .map_err(|e| io::Error::other(format!("Readline error: {e}")))?;

            let input = line.trim();

            // Validate the input
            match self.validate_input(setting, input) {
                Ok(value) => return Ok(value),
                Err(error) => {
                    eprintln!("✗ {error}");
                    eprintln!("  Please try again.\n");
                }
            }
        }
    }

    fn validate_input(
        &self,
        setting: &ExpectedSetting,
        input: &str,
    ) -> Result<Option<String>, String> {
        // Empty input accepts a default or leaves an optional setting absent.
        if input.is_empty() {
            return if let Some(default) = &setting.default_value {
                Ok(Some(default.clone()))
            } else if !setting.required {
                Ok(None)
            } else {
                Err("Value is required".to_string())
            };
        }

        match setting.config_type {
            ConfigType::Boolean => {
                let normalized = input.trim().to_lowercase();
                match normalized.as_str() {
                    "y" | "yes" | "true" | "1" => Ok("true".to_string()),
                    "n" | "no" | "false" | "0" => Ok("false".to_string()),
                    _ => {
                        Err("Invalid boolean value. Enter y/yes/true/1 or n/no/false/0".to_string())
                    }
                }
            }
            ConfigType::Integer => {
                input
                    .parse::<i64>()
                    .map_err(|_| format!("Invalid integer value: '{input}'"))?;
                Ok(input.to_string())
            }
            ConfigType::LogLevel => {
                if let Some(valid_values) = &setting.valid_values {
                    let normalized = input.trim().to_lowercase();
                    if valid_values.iter().any(|v| v.to_lowercase() == normalized) {
                        Ok(normalized)
                    } else {
                        Err(format!(
                            "Invalid log level. Valid options: {}",
                            valid_values.join(", ")
                        ))
                    }
                } else {
                    Ok(input.to_string())
                }
            }
            ConfigType::ShellMode => {
                if let Some(valid_values) = &setting.valid_values {
                    let normalized = input.trim().to_lowercase();
                    if valid_values.iter().any(|v| v.to_lowercase() == normalized) {
                        Ok(normalized)
                    } else {
                        Err(format!(
                            "Invalid shell mode. Valid options: {}",
                            valid_values.join(", ")
                        ))
                    }
                } else {
                    Ok(input.to_string())
                }
            }
            ConfigType::StringList => {
                // Accept comma-separated values
                // Validation is minimal - just ensure it's not malformed
                Ok(input.to_string())
            }
            ConfigType::IpAddress => {
                use std::net::IpAddr;
                if input.parse::<IpAddr>().is_ok() {
                    // Warn about 0.0.0.0 binding (security concern)
                    if input == "0.0.0.0" {
                        eprintln!(
                            "⚠ Warning: Binding to 0.0.0.0 makes the server accessible from any network interface."
                        );
                        eprintln!("  This may be a security risk if not intended.");
                    }
                    Ok(input.to_string())
                } else {
                    Err(format!("Invalid IP address: '{input}'"))
                }
            }
            ConfigType::String => {
                if let Some(valid_values) = &setting.valid_values {
                    if valid_values.contains(&input.to_string()) {
                        Ok(input.to_string())
                    } else {
                        Err(format!(
                            "Invalid value. Valid options: {}",
                            valid_values.join(", ")
                        ))
                    }
                } else {
                    Ok(input.to_string())
                }
            }
        }
        .map(Some)
    }

    fn format_prompt(&self, setting: &ExpectedSetting) -> String {
        let mut prompt = format!("{} - {}\n", setting.name, setting.description);

        // Show valid values for enums
        if let Some(valid_values) = &setting.valid_values {
            prompt.push_str(&format!("Valid options: {}\n", valid_values.join(", ")));
        }

        // Show default value
        if let Some(default) = &setting.default_value {
            prompt.push_str(&format!("Enter value [{}]: ", default));
        } else if !setting.required {
            prompt.push_str("Enter value (optional; press Enter to skip): ");
        } else {
            prompt.push_str("Enter value: ");
        }

        prompt
    }

    /// Save the collected values, using defaults for unanswered settings.
    fn generate_file(&self, path: &Path) -> Result<(), io::Error> {
        write_config_file(path, |file| self.write_config(file))
    }

    /// Serialize configuration text, propagating every output error.
    fn write_config(&self, mut file: impl io::Write) -> Result<(), io::Error> {
        // Write header
        writeln!(file, "# WebFirst Language Configuration File")?;
        writeln!(
            file,
            "# Created by wfl config on {}",
            chrono::Local::now().format("%Y-%m-%d")
        )?;
        writeln!(file)?;

        // Get settings grouped by category
        let categories = self.checker.get_settings_by_category();

        for (category_name, mut settings) in categories {
            writeln!(
                file,
                "# ================================================================================"
            )?;
            writeln!(file, "# {}", category_name)?;
            writeln!(
                file,
                "# ================================================================================"
            )?;
            writeln!(file)?;

            // Sort settings alphabetically within category
            settings.sort_by(|a, b| a.name.cmp(&b.name));

            for setting in settings {
                // Write description as comment
                writeln!(file, "# {}", setting.description)?;

                // Write setting value
                if let Some(value) = self.values.get(&setting.name) {
                    writeln!(file, "{} = {}", setting.name, value)?;
                } else if let Some(default) = &setting.default_value {
                    writeln!(file, "{} = {}", setting.name, default)?;
                }

                writeln!(file)?;
            }
        }

        Ok(())
    }
}

/// Write a configuration after creating any missing destination directories.
fn write_config_file(
    path: &Path,
    write: impl FnOnce(&mut std::fs::File) -> io::Result<()>,
) -> io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(path)?;
    write(&mut file)
}

/// Public entry point for running the wizard
pub fn run_wizard(output_path: &Path) -> Result<(), io::Error> {
    let mut wizard = ConfigWizard::new()?;
    wizard.run(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    /// Fail after real partial output, or after rendering during flush.
    struct FailingWriter<'a> {
        file: &'a mut std::fs::File,
        remaining: Option<usize>,
    }

    impl Write for FailingWriter<'_> {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.remaining == Some(0) {
                return Err(io::Error::other("injected write failure"));
            }
            let limit = self.remaining.unwrap_or(bytes.len()).min(bytes.len());
            let written = self.file.write(&bytes[..limit])?;
            if let Some(remaining) = &mut self.remaining {
                *remaining -= written;
            }
            Ok(written)
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("injected flush failure"))
        }
    }

    #[test]
    fn test_config_save_failures_preserve_destination_and_clean_temporary_files() {
        let wizard = ConfigWizard::new().unwrap();
        for existing in [false, true] {
            for remaining in [Some(32), None] {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("config");
                let original = b"# existing configuration\ntimeout_seconds = 19\n";
                if existing {
                    std::fs::write(&path, original).unwrap();
                }
                let error = write_config_file(&path, |file| {
                    let mut writer = FailingWriter { file, remaining };
                    wizard.write_config(&mut writer)?;
                    writer.flush()
                })
                .unwrap_err();
                assert!(error.to_string().contains("injected"), "{error}");
                if existing {
                    assert_eq!(std::fs::read(&path).unwrap(), original);
                } else {
                    assert!(
                        !path.exists(),
                        "a failed save must not publish partial output"
                    );
                }
                assert_eq!(
                    std::fs::read_dir(directory.path()).unwrap().count(),
                    usize::from(existing),
                    "failed saves must remove temporary output"
                );
            }
        }
    }

    #[test]
    fn test_config_save_replaces_file_without_changing_existing_readers() {
        let wizard = ConfigWizard::new().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config");
        let original = "# previous complete configuration\n";
        std::fs::write(&path, original).unwrap();
        let mut reader = std::fs::File::open(&path).unwrap();

        wizard.generate_file(&path).unwrap();

        let mut old_contents = String::new();
        reader.read_to_string(&mut old_contents).unwrap();
        assert_eq!(
            old_contents, original,
            "existing readers must retain the old file"
        );
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("# Created by wfl config on "));
        assert!(contents.contains("timeout_seconds = 60"));
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn test_config_save_cleans_temporary_file_when_replacement_fails() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config");
        let sentinel = path.join("keep.txt");
        let error = write_config_file(&path, |file| {
            file.write_all(b"complete replacement")?;
            std::fs::create_dir(&path)?;
            std::fs::write(&sentinel, b"preserve this directory")?;
            Ok(())
        })
        .unwrap_err();
        assert!(
            path.is_dir(),
            "replacement failure must preserve the directory: {error}"
        );
        assert_eq!(std::fs::read(sentinel).unwrap(), b"preserve this directory");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn test_config_save_preserves_symlink_and_target_permissions() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("settings");
        let link = directory.path().join("config");
        std::fs::write(&target, b"old settings").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640)).unwrap();
        symlink("settings", &link).unwrap();

        ConfigWizard::new().unwrap().generate_file(&link).unwrap();

        assert!(link.is_symlink());
        assert!(
            std::fs::read_to_string(&target)
                .unwrap()
                .contains("timeout_seconds = 60")
        );
        assert_eq!(
            std::fs::metadata(&target).unwrap().permissions().mode() & 0o777,
            0o640
        );
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn test_config_save_rejects_dangling_symlink_without_creating_target() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("missing");
        let link = directory.path().join("config");
        symlink("missing", &link).unwrap();

        assert!(ConfigWizard::new().unwrap().generate_file(&link).is_err());

        assert!(link.is_symlink());
        assert!(!target.exists());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn test_generate_file_creates_missing_parent_directories() {
        let wizard = ConfigWizard::new().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested").join("wfl").join("config");
        assert!(!path.parent().unwrap().exists());

        wizard.generate_file(&path).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("# Created by wfl config on "));
        assert!(contents.contains("timeout_seconds = 60"));
        assert!(!contents.contains("web_server_tls_cert_file ="));
        assert!(!contents.contains("web_server_tls_key_file ="));
    }

    #[test]
    fn test_generate_file_preserves_file_blocking_parent_directory() {
        let wizard = ConfigWizard::new().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let parent = directory.path().join("wfl");
        let original = "existing file must be preserved";
        std::fs::write(&parent, original).unwrap();
        let path = parent.join("config");

        assert!(wizard.generate_file(&path).is_err());

        assert_eq!(std::fs::read_to_string(parent).unwrap(), original);
        assert!(!path.exists());
    }

    #[test]
    fn test_optional_tls_settings_accept_blank_input() {
        let wizard = ConfigWizard::new().unwrap();
        for name in ["web_server_tls_cert_file", "web_server_tls_key_file"] {
            let setting = &wizard.checker.get_expected_settings()[name];
            assert!(!setting.required);
            assert!(setting.default_value.is_none());
            assert_eq!(
                wizard.validate_input(setting, "").unwrap(),
                None,
                "optional setting {name} must allow Enter to skip"
            );
        }
    }

    #[test]
    fn test_required_setting_without_default_rejects_blank_input() {
        let wizard = ConfigWizard::new().unwrap();
        let mut setting =
            wizard.checker.get_expected_settings()["web_server_tls_cert_file"].clone();
        setting.required = true;
        assert_eq!(
            wizard.validate_input(&setting, "").unwrap_err(),
            "Value is required"
        );
    }

    #[test]
    fn test_optional_tls_prompts_explain_enter_to_skip() {
        let wizard = ConfigWizard::new().unwrap();
        for name in ["web_server_tls_cert_file", "web_server_tls_key_file"] {
            let prompt = wizard.format_prompt(&wizard.checker.get_expected_settings()[name]);
            assert!(prompt.contains("optional"), "{prompt}");
            assert!(prompt.contains("Enter to skip"), "{prompt}");
        }
    }

    #[test]
    fn test_generated_config_omits_unset_tls_settings() {
        let mut wizard = ConfigWizard::new().unwrap();
        for setting in wizard.checker.get_expected_settings().values() {
            if let Some(value) = wizard.validate_input(setting, "").unwrap() {
                wizard.values.insert(setting.name.clone(), value);
            }
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(".wflcfg");
        wizard.generate_file(&path).unwrap();
        let contents = std::fs::read_to_string(path).unwrap();
        assert!(!contents.contains("web_server_tls_cert_file ="));
        assert!(!contents.contains("web_server_tls_key_file ="));
        assert!(contents.contains("web_server_bind_address = 127.0.0.1"));
        assert!(contents.contains("allowed_shell_commands = \n"));
    }

    #[test]
    fn test_generated_config_preserves_explicit_tls_paths() {
        let mut wizard = ConfigWizard::new().unwrap();
        wizard.values.insert(
            "web_server_tls_cert_file".to_string(),
            "certificates/server certificate.pem".to_string(),
        );
        wizard.values.insert(
            "web_server_tls_key_file".to_string(),
            "certificates/server key.pem".to_string(),
        );
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(".wflcfg");
        wizard.generate_file(&path).unwrap();
        let contents = std::fs::read_to_string(path).unwrap();
        assert!(
            contents.contains("web_server_tls_cert_file = certificates/server certificate.pem")
        );
        assert!(contents.contains("web_server_tls_key_file = certificates/server key.pem"));
    }

    #[test]
    fn test_generated_config_names_config_command() {
        let wizard = ConfigWizard::new().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(".wflcfg");
        wizard.generate_file(&path).unwrap();
        let contents = std::fs::read_to_string(path).unwrap();
        assert!(contents.contains("# Created by wfl config on "));
    }

    #[test]
    fn test_validate_boolean_input() {
        let wizard = ConfigWizard::new().unwrap();
        let setting = ExpectedSetting {
            name: "test".to_string(),
            config_type: ConfigType::Boolean,
            required: false,
            default_value: Some("false".to_string()),
            description: "Test setting".to_string(),
            valid_values: None,
            category: "Test".to_string(),
        };

        // Test various boolean inputs
        assert_eq!(
            wizard.validate_input(&setting, "y").unwrap().unwrap(),
            "true"
        );
        assert_eq!(
            wizard.validate_input(&setting, "yes").unwrap().unwrap(),
            "true"
        );
        assert_eq!(
            wizard.validate_input(&setting, "true").unwrap().unwrap(),
            "true"
        );
        assert_eq!(
            wizard.validate_input(&setting, "1").unwrap().unwrap(),
            "true"
        );
        assert_eq!(
            wizard.validate_input(&setting, "n").unwrap().unwrap(),
            "false"
        );
        assert_eq!(
            wizard.validate_input(&setting, "no").unwrap().unwrap(),
            "false"
        );
        assert_eq!(
            wizard.validate_input(&setting, "false").unwrap().unwrap(),
            "false"
        );
        assert_eq!(
            wizard.validate_input(&setting, "0").unwrap().unwrap(),
            "false"
        );

        // Test invalid input
        assert!(wizard.validate_input(&setting, "maybe").is_err());
    }

    #[test]
    fn test_validate_outbound_stream_lifetime_accepts_unsigned_range_only() {
        let wizard = ConfigWizard::new().unwrap();
        let setting = ExpectedSetting {
            name: "outbound_stream_max_seconds".to_string(),
            config_type: ConfigType::Integer,
            required: false,
            default_value: Some("300".to_string()),
            description: "Total outbound stream lifetime; 0 disables the cap".to_string(),
            valid_values: None,
            category: "Web Server".to_string(),
        };
        for value in ["0", "60", "300", "18446744073709551615"] {
            assert_eq!(
                wizard.validate_input(&setting, value).unwrap().as_deref(),
                Some(value)
            );
        }
        for value in ["-1", "1.5", "abc", "18446744073709551616"] {
            assert!(wizard.validate_input(&setting, value).is_err(), "{value}");
        }
    }

    #[test]
    fn test_validate_integer_input() {
        let wizard = ConfigWizard::new().unwrap();
        let setting = ExpectedSetting {
            name: "test".to_string(),
            config_type: ConfigType::Integer,
            required: false,
            default_value: Some("60".to_string()),
            description: "Test setting".to_string(),
            valid_values: None,
            category: "Test".to_string(),
        };

        // Test valid integers
        assert_eq!(
            wizard.validate_input(&setting, "123").unwrap().unwrap(),
            "123"
        );
        assert_eq!(
            wizard.validate_input(&setting, "-456").unwrap().unwrap(),
            "-456"
        );

        // Test invalid input
        assert!(wizard.validate_input(&setting, "abc").is_err());
        assert!(wizard.validate_input(&setting, "12.34").is_err());
    }

    #[test]
    fn test_validate_ip_address() {
        let wizard = ConfigWizard::new().unwrap();
        let setting = ExpectedSetting {
            name: "test".to_string(),
            config_type: ConfigType::IpAddress,
            required: false,
            default_value: Some("127.0.0.1".to_string()),
            description: "Test setting".to_string(),
            valid_values: None,
            category: "Test".to_string(),
        };

        // Test valid IPv4
        assert_eq!(
            wizard
                .validate_input(&setting, "127.0.0.1")
                .unwrap()
                .unwrap(),
            "127.0.0.1"
        );
        assert_eq!(
            wizard
                .validate_input(&setting, "192.168.1.1")
                .unwrap()
                .unwrap(),
            "192.168.1.1"
        );

        // Test valid IPv6
        assert_eq!(
            wizard.validate_input(&setting, "::1").unwrap().unwrap(),
            "::1"
        );
        assert_eq!(
            wizard.validate_input(&setting, "fe80::1").unwrap().unwrap(),
            "fe80::1"
        );

        // Test invalid input
        assert!(wizard.validate_input(&setting, "999.999.999.999").is_err());
        assert!(wizard.validate_input(&setting, "not-an-ip").is_err());
    }

    #[test]
    fn test_validate_log_level() {
        let wizard = ConfigWizard::new().unwrap();
        let setting = ExpectedSetting {
            name: "test".to_string(),
            config_type: ConfigType::LogLevel,
            required: false,
            default_value: Some("info".to_string()),
            description: "Test setting".to_string(),
            valid_values: Some(vec![
                "debug".to_string(),
                "info".to_string(),
                "warn".to_string(),
                "error".to_string(),
            ]),
            category: "Test".to_string(),
        };

        // Test valid log levels
        assert_eq!(
            wizard.validate_input(&setting, "debug").unwrap().unwrap(),
            "debug"
        );
        assert_eq!(
            wizard.validate_input(&setting, "INFO").unwrap().unwrap(),
            "info"
        );
        assert_eq!(
            wizard.validate_input(&setting, "Error").unwrap().unwrap(),
            "error"
        );

        // Test invalid input
        assert!(wizard.validate_input(&setting, "trace").is_err());
        assert!(wizard.validate_input(&setting, "critical").is_err());
    }

    #[test]
    fn test_format_prompt() {
        let wizard = ConfigWizard::new().unwrap();
        let setting = ExpectedSetting {
            name: "timeout_seconds".to_string(),
            config_type: ConfigType::Integer,
            required: false,
            default_value: Some("60".to_string()),
            description: "Maximum execution time".to_string(),
            valid_values: None,
            category: "Test".to_string(),
        };

        let prompt = wizard.format_prompt(&setting);
        assert!(prompt.contains("timeout_seconds"));
        assert!(prompt.contains("Maximum execution time"));
        assert!(prompt.contains("[60]"));
    }
}
