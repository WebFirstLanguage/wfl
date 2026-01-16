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
        let editor = DefaultEditor::new().map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to create editor: {e}"),
            )
        })?;

        Ok(Self {
            editor,
            checker: ConfigChecker::new(),
            values: HashMap::new(),
        })
    }

    pub fn run(&mut self, output_path: &Path) -> Result<(), io::Error> {
        println!("\nWebFirst Language Configuration Wizard");
        println!("======================================\n");
        println!("This wizard will help you create a .wflcfg file with all configuration options.");
        println!(
            "Press Enter to accept the default value shown in brackets, or type a new value.\n"
        );

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
            let value = self.prompt_setting(setting)?;
            self.values.insert(setting.name.clone(), value);
        }

        println!();
        Ok(())
    }

    fn prompt_setting(&mut self, setting: &ExpectedSetting) -> Result<String, io::Error> {
        let prompt = self.format_prompt(setting);

        loop {
            let line = self.editor.readline(&prompt).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("Readline error: {e}"))
            })?;

            let input = line.trim();

            // Empty input means accept default
            if input.is_empty() {
                if let Some(default) = &setting.default_value {
                    return Ok(default.clone());
                }
            }

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

    fn validate_input(&self, setting: &ExpectedSetting, input: &str) -> Result<String, String> {
        // Empty input is only valid if there's a default
        if input.is_empty() {
            return if setting.default_value.is_some() {
                Ok(setting.default_value.clone().unwrap())
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
                    _ => Err(format!(
                        "Invalid boolean value. Enter y/yes/true/1 or n/no/false/0"
                    )),
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
        } else {
            prompt.push_str("Enter value: ");
        }

        prompt
    }

    fn generate_file(&self, path: &Path) -> Result<(), io::Error> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;

        // Write header
        writeln!(file, "# WebFirst Language Configuration File")?;
        writeln!(
            file,
            "# Created by wfl --init on {}",
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

/// Public entry point for running the wizard
pub fn run_wizard(output_path: &Path) -> Result<(), io::Error> {
    let mut wizard = ConfigWizard::new()?;
    wizard.run(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
        assert_eq!(wizard.validate_input(&setting, "y").unwrap(), "true");
        assert_eq!(wizard.validate_input(&setting, "yes").unwrap(), "true");
        assert_eq!(wizard.validate_input(&setting, "true").unwrap(), "true");
        assert_eq!(wizard.validate_input(&setting, "1").unwrap(), "true");
        assert_eq!(wizard.validate_input(&setting, "n").unwrap(), "false");
        assert_eq!(wizard.validate_input(&setting, "no").unwrap(), "false");
        assert_eq!(wizard.validate_input(&setting, "false").unwrap(), "false");
        assert_eq!(wizard.validate_input(&setting, "0").unwrap(), "false");

        // Test invalid input
        assert!(wizard.validate_input(&setting, "maybe").is_err());
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
        assert_eq!(wizard.validate_input(&setting, "123").unwrap(), "123");
        assert_eq!(wizard.validate_input(&setting, "-456").unwrap(), "-456");

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
            wizard.validate_input(&setting, "127.0.0.1").unwrap(),
            "127.0.0.1"
        );
        assert_eq!(
            wizard.validate_input(&setting, "192.168.1.1").unwrap(),
            "192.168.1.1"
        );

        // Test valid IPv6
        assert_eq!(wizard.validate_input(&setting, "::1").unwrap(), "::1");
        assert_eq!(
            wizard.validate_input(&setting, "fe80::1").unwrap(),
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
        assert_eq!(wizard.validate_input(&setting, "debug").unwrap(), "debug");
        assert_eq!(wizard.validate_input(&setting, "INFO").unwrap(), "info");
        assert_eq!(wizard.validate_input(&setting, "Error").unwrap(), "error");

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
