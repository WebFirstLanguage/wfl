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

    if !stderr.is_empty() {
        format!("{}{}", stdout, stderr)
    } else {
        stdout.to_string()
    }
}

#[test]
fn test_newline_escape_in_string() {
    let result = run_wfl(
        r#"
        store text as "line1\nline2\nline3"
        store lines as split text by "\n"
        display length of lines
        "#,
    );
    assert_eq!(result.trim(), "3");
}

#[test]
fn test_tab_escape_in_string() {
    let result = run_wfl(
        r#"
        store text as "name\tvalue"
        display length of text
        "#,
    );
    // "name\tvalue" with real tab should be 10 chars (name + tab + value)
    assert_eq!(result.trim(), "10");
}

#[test]
fn test_backslash_escape_in_string() {
    let result = run_wfl(
        r#"
        store path as "C:\\Users\\Alice"
        display length of path
        "#,
    );
    // Should be 14 chars: C:\Users\Alice
    assert_eq!(result.trim(), "14");
}

#[test]
fn test_carriage_return_escape() {
    let result = run_wfl(
        r#"
        store text as "hello\rworld"
        display length of text
        "#,
    );
    // Should be 11 chars: hello + \r + world
    assert_eq!(result.trim(), "11");
}

#[test]
fn test_null_escape() {
    let result = run_wfl(
        r#"
        store text as "data\0more"
        display length of text
        "#,
    );
    // Should be 9 chars: data + null + more
    assert_eq!(result.trim(), "9");
}

#[test]
fn test_double_quote_escape() {
    let result = run_wfl(
        r#"
        store text as "She said \"hello\""
        display text
        "#,
    );
    assert_eq!(result.trim(), r#"She said "hello""#);
}

#[test]
fn test_mixed_escapes() {
    let result = run_wfl(
        r#"
        store text as "Line1\nTab:\there\r\nEnd"
        display length of text
        "#,
    );
    // Line1 (5) + \n (1) + Tab: (4) + \t (1) + here (4) + \r (1) + \n (1) + End (3) = 20
    assert_eq!(result.trim(), "20");
}

#[test]
fn test_backslash_before_n_literal() {
    let result = run_wfl(
        r#"
        store text as "path\\nfile"
        display length of text
        "#,
    );
    // Should be 10 chars: path\nfile (backslash and 'n', not newline)
    assert_eq!(result.trim(), "10");
}

#[test]
fn test_invalid_escape_errors() {
    let result = run_wfl(
        r#"
        store text as "invalid\xescape"
        display text
        "#,
    );
    // Should contain an error message about invalid escape sequence
    assert!(
        result.contains("error") || result.contains("Error") || result.contains("ERROR"),
        "Expected error for invalid escape sequence, got: {}",
        result
    );
}
