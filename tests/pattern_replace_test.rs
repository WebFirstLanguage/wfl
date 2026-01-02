use std::fs;
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

fn run_wfl(code: &str) -> String {
    let temp_file = TempWflFile::new(code).expect("Failed to create temp file");

    let wfl_exe = if cfg!(target_os = "windows") {
        "target/release/wfl.exe"
    } else {
        "target/release/wfl"
    };

    let output = Command::new(wfl_exe)
        .arg(temp_file.path())
        .output()
        .expect("Failed to execute WFL");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stderr.is_empty() {
        format!("{}{}", stdout, stderr)
    } else {
        stdout.to_string()
    }
}

#[test]
fn test_replace_basic() {
    let result = run_wfl(
        r#"
        store text as "hello world"
        create pattern p:
            "world"
        end pattern
        store result as replace p with "universe" in text
        display result
    "#,
    );
    assert_eq!(result.trim(), "hello universe");
}

#[test]
fn test_replace_full_match_ref() {
    let result = run_wfl(
        r#"
        store text as "foo bar"
        create pattern p:
            "foo"
        end pattern
        store result as replace p with "[$0]" in text
        display result
    "#,
    );
    assert_eq!(result.trim(), "[foo] bar");
}

#[test]
fn test_replace_named_capture() {
    let result = run_wfl(
        r#"
        store text as "hello world"
        create pattern p:
            capture {
                one or more letter
            } as name
            " "
            capture {
                one or more letter
            } as place
        end pattern
        store result as replace p with "$place, $name" in text
        display result
    "#,
    );
    assert_eq!(result.trim(), "world, hello");
}

#[test]
fn test_replace_escape_dollar() {
    let result = run_wfl(
        r#"
        store text as "price 10"
        create pattern p:
            "10"
        end pattern
        store result as replace p with "$$20" in text
        display result
    "#,
    );
    assert_eq!(result.trim(), "price $20");
}

#[test]
fn test_replace_multiple() {
    let result = run_wfl(
        r#"
        store text as "a b c"
        create pattern p:
            " "
        end pattern
        store result as replace p with "-" in text
        display result
    "#,
    );
    assert_eq!(result.trim(), "a-b-c");
}

#[test]
fn test_replace_unicode() {
    let result = run_wfl(
        r#"
        store text as "🚀 rocket 🦀 crab"
        create pattern p:
            " "
        end pattern
        store result as replace p with "_" in text
        display result
    "#,
    );
    assert_eq!(result.trim(), "🚀_rocket_🦀_crab");
}
