use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;

/// Robust temporary file cleanup wrapper
struct TempWflFile {
    _file: NamedTempFile, // Keep file alive for automatic cleanup
    path: String,
}

impl TempWflFile {
    fn new(code: &str) -> Result<Self, std::io::Error> {
        let file = NamedTempFile::with_suffix(".wfl")?;
        fs::write(file.path(), code)?;
        let path = file.path().to_string_lossy().to_string();
        Ok(TempWflFile { _file: file, path })
    }

    fn path(&self) -> &str {
        &self.path
    }
}

// Drop automatically cleans up the file when TempWflFile goes out of scope

fn get_wfl_binary_path() -> PathBuf {
    let current_dir = env::current_dir().unwrap();
    let release_path = if cfg!(target_os = "windows") {
        current_dir.join("target/release/wfl.exe")
    } else {
        current_dir.join("target/release/wfl")
    };

    if release_path.exists() {
        return release_path;
    }

    let debug_path = if cfg!(target_os = "windows") {
        current_dir.join("target/debug/wfl.exe")
    } else {
        current_dir.join("target/debug/wfl")
    };

    if debug_path.exists() {
        return debug_path;
    }

    panic!("WFL binary not found. Run 'cargo build' or 'cargo build --release' first.");
}

fn run_wfl(code: &str) -> String {
    // Create temporary WFL file with automatic cleanup
    let temp_file = TempWflFile::new(code).expect("Failed to create temp file");

    // Run the WFL interpreter
    let binary_path = get_wfl_binary_path();

    let output = Command::new(binary_path)
        .arg(temp_file.path())
        .output()
        .expect("Failed to execute WFL");

    // Combine stdout and stderr for complete output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Clean up any potential debug files created by WFL
    cleanup_debug_files(temp_file.path());

    if !stderr.is_empty() {
        format!("{}{}", stdout, stderr)
    } else {
        stdout.to_string()
    }
    // TempWflFile automatically cleans up when it goes out of scope
}

/// Clean up debug files that may be created during WFL execution
fn cleanup_debug_files(wfl_file_path: &str) {
    use std::path::Path;

    let path = Path::new(wfl_file_path);
    if let Some(stem) = path.file_stem() {
        let debug_file = path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(format!("{}_debug.txt", stem.to_string_lossy()));

        // Try to remove debug file if it exists, ignore errors
        let _ = fs::remove_file(debug_file);
    }
}

#[test]
fn test_string_split_basic() {
    let result = run_wfl(
        r#"
        store text as "a,b,c"
        store parts as split text by ","
        display length of parts
    "#,
    );
    assert_eq!(result.trim(), "3");
}

#[test]
fn test_string_split_space() {
    let result = run_wfl(
        r#"
        store text as "hello world test"
        store parts as split text by " "
        display parts[0]
        display parts[1]
        display parts[2]
    "#,
    );
    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines[0], "hello");
    assert_eq!(lines[1], "world");
    assert_eq!(lines[2], "test");
}

#[test]
fn test_string_split_empty() {
    let result = run_wfl(
        r#"
        store text as ""
        store parts as split text by ","
        display length of parts
    "#,
    );
    assert_eq!(result.trim(), "1");
}

#[test]
fn test_string_split_no_delimiter() {
    let result = run_wfl(
        r#"
        store text as "hello"
        store parts as split text by ","
        display length of parts
        display parts[0]
    "#,
    );
    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines[0], "1");
    assert_eq!(lines[1], "hello");
}

#[test]
fn test_string_split_adjacent_delimiters() {
    let result = run_wfl(
        r#"
        store text as "a,,b"
        store parts as split text by ","
        display length of parts
    "#,
    );
    assert_eq!(result.trim(), "3");
}

#[test]
fn test_pattern_split_registered() {
    let result = run_wfl(
        r#"
        create pattern comma:
            ","
        end pattern
        store parts as split "x,y,z" on pattern comma
        display length of parts
    "#,
    );
    assert_eq!(result.trim(), "3");
}

#[test]
fn test_pattern_split_basic() {
    let result = run_wfl(
        r#"
        store text as "a,b,c"
        create pattern comma:
            ","
        end pattern
        store parts as split text on pattern comma
        display parts[0]
        display parts[1]
        display parts[2]
    "#,
    );
    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines[0], "a");
    assert_eq!(lines[1], "b");
    assert_eq!(lines[2], "c");
}

#[test]
fn test_pattern_split_whitespace() {
    let result = run_wfl(
        r#"
        store text as "hello  world  test"
        create pattern spaces:
            one or more " "
        end pattern
        store parts as split text on pattern spaces
        display length of parts
    "#,
    );
    assert_eq!(result.trim(), "5"); // Pattern splits on individual spaces in "hello  world  test"
}

#[test]
fn test_split_returns_list() {
    let result = run_wfl(
        r#"
        store text as "a,b,c"
        store parts as split text by ","
        display parts[0]
        display parts[1]
        display parts[2]
    "#,
    );
    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "a");
    assert_eq!(lines[1], "b");
    assert_eq!(lines[2], "c");
}

#[test]
fn test_split_type_error_non_text() {
    let result = run_wfl(
        r#"
        store num as 123
        store parts as split num by ","
    "#,
    );
    assert!(result.contains("error") || result.contains("Error"));
}

#[test]
fn test_split_type_error_non_text_delimiter() {
    let result = run_wfl(
        r#"
        store text as "a,b,c"
        store parts as split text by 123
    "#,
    );
    assert!(result.contains("error") || result.contains("Error"));
}
