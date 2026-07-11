//! Regression / feature tests for issue #602 — expose the running
//! interpreter's version to WFL programs.
//!
//! A WFL program must be able to read the version of the interpreter that is
//! actually executing it, without shelling out to `wfl --version`. This is
//! provided as the global immutable constant `wfl_version`, which resolves to
//! the bare semver text of `wfl::version::VERSION` (mirroring how `newline` and
//! `tab` are exposed).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn wfl_exe() -> &'static str {
    env!("CARGO_BIN_EXE_wfl")
}

/// Run inline WFL source in a fresh temp dir, returning (stdout+stderr, exit code).
fn run_src(src: &str) -> (String, Option<i32>) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("main.wfl");
    fs::write(&path, src).unwrap();
    let output = Command::new(wfl_exe())
        .arg(&path)
        .output()
        .expect("failed to execute WFL");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    drop(dir);
    (combined, output.status.code())
}

/// `wfl_version` resolves to the running interpreter's semver text and matches
/// the compiled-in constant exactly.
#[test]
fn wfl_version_reports_running_interpreter_version() {
    let (out, code) = run_src("store v as wfl_version\ndisplay v\n");
    assert_eq!(code, Some(0), "program should exit cleanly; got:\n{out}");
    let expected = wfl::version::VERSION;
    assert!(
        out.lines().any(|line| line.trim() == expected),
        "expected a line equal to {expected:?}, got:\n{out}"
    );
}

/// `wfl_version` is a Text value (so it composes with text operations).
#[test]
fn wfl_version_is_text() {
    let (out, code) = run_src("display typeof of wfl_version\n");
    assert_eq!(code, Some(0), "program should exit cleanly; got:\n{out}");
    assert!(
        out.lines().any(|line| line.trim() == "Text"),
        "expected typeof of wfl_version to be Text, got:\n{out}"
    );
}

/// `wfl_version` is directly usable inside a string/text expression, so a
/// program can build its own banner without a subprocess.
#[test]
fn wfl_version_composes_in_text() {
    let (out, code) = run_src("display \"WFL \" with wfl_version\n");
    assert_eq!(code, Some(0), "program should exit cleanly; got:\n{out}");
    let expected = format!("WFL {}", wfl::version::VERSION);
    assert!(
        out.lines().any(|line| line.trim() == expected),
        "expected a line equal to {expected:?}, got:\n{out}"
    );
}
