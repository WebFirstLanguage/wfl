use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Test context for creating temporary WFL files and running import tests
pub struct ImportTestContext {
    temp_dir: TempDir,
    files: HashMap<String, String>,
}

impl Default for ImportTestContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ImportTestContext {
    /// Create a new test context with a temporary directory
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().expect("Failed to create temp directory"),
            files: HashMap::new(),
        }
    }

    /// Add a file to be created in the temporary directory
    pub fn add_file(&mut self, name: &str, content: &str) -> &mut Self {
        self.files.insert(name.to_string(), content.to_string());
        self
    }

    /// Write all registered files to the temporary directory
    pub fn write_files(&self) {
        for (name, content) in &self.files {
            let path = self.temp_dir.path().join(name);

            // Create parent directories if needed
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("Failed to create parent directories");
            }

            fs::write(&path, content).expect("Failed to write file");
        }
    }

    /// Run a WFL program and return its output (stdout + stderr)
    pub fn run(&self, main_file: &str) -> String {
        self.write_files();

        let main_path = self.temp_dir.path().join(main_file);

        let output = std::process::Command::new(wfl_exe())
            .arg(&main_path)
            .output()
            .expect("Failed to execute WFL");

        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }

    /// Get the path to a file in the temporary directory
    pub fn file_path(&self, name: &str) -> PathBuf {
        self.temp_dir.path().join(name)
    }

    /// Get the temporary directory path
    pub fn temp_dir_path(&self) -> &Path {
        self.temp_dir.path()
    }
}

/// Get the path to the WFL executable
fn wfl_exe() -> &'static str {
    if cfg!(target_os = "windows") {
        "target/release/wfl.exe"
    } else {
        "target/release/wfl"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = ImportTestContext::new();
        assert!(ctx.temp_dir_path().exists());
    }

    #[test]
    fn test_add_and_write_files() {
        let mut ctx = ImportTestContext::new();
        ctx.add_file("test.wfl", "display \"test\"");
        ctx.write_files();

        let test_path = ctx.file_path("test.wfl");
        assert!(test_path.exists());

        let content = fs::read_to_string(test_path).unwrap();
        assert_eq!(content, "display \"test\"");
    }
}
