//! Sunset tests for the removed WFL to JavaScript transpiler.
//!
//! The transpiler has been retired. These tests pin the user-visible contract
//! of that removal so it cannot silently regress:
//!
//! 1. `--transpile` (and its transpiler-only options) is rejected with a clear,
//!    actionable message instead of being mistaken for an input file path.
//! 2. No JavaScript output file is produced for any invocation.
//! 3. `wfl --help` no longer advertises transpilation.
//! 4. Ordinary interpretation of the same program is unaffected.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn wfl_exe() -> &'static str {
    env!("CARGO_BIN_EXE_wfl")
}

const SAMPLE: &str = "store greeting as \"Hello\"\ndisplay greeting\n";

/// Run the `wfl` binary with `args` inside a fresh temp dir that contains
/// `main.wfl`, returning (stdout, stderr, exit code, temp dir).
fn run_cli(args: &[&str]) -> (String, String, Option<i32>, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("main.wfl");
    fs::write(&path, SAMPLE).unwrap();

    let mut cmd = Command::new(wfl_exe());
    cmd.current_dir(dir.path());
    for arg in args {
        if *arg == "<FILE>" {
            cmd.arg(&path);
        } else {
            cmd.arg(arg);
        }
    }
    let output = cmd.output().expect("failed to execute WFL");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code(),
        dir,
    )
}

/// Assert no `.js` artifact was written anywhere under `dir`.
fn assert_no_js_output(dir: &TempDir) {
    for entry in fs::read_dir(dir.path()).expect("read temp dir") {
        let path = entry.expect("dir entry").path();
        assert_ne!(
            path.extension().and_then(|e| e.to_str()),
            Some("js"),
            "transpiler is removed, but a JavaScript file was produced: {}",
            path.display()
        );
    }
}

/// `--transpile` must fail loudly with a sunset message, not be swallowed as a
/// file path (which would produce a confusing "file not found" error).
#[test]
fn transpile_flag_is_rejected_with_sunset_message() {
    let (stdout, stderr, code, dir) = run_cli(&["--transpile", "<FILE>"]);
    let combined = format!("{stdout}{stderr}");

    assert_eq!(
        code,
        Some(2),
        "--transpile should exit with usage error code 2; got:\n{combined}"
    );
    let lowered = combined.to_lowercase();
    assert!(
        lowered.contains("transpiler") && lowered.contains("removed"),
        "expected a message saying the transpiler was removed, got:\n{combined}"
    );
    assert!(
        lowered.contains("wfl <file"),
        "expected the error to point at running the file directly, got:\n{combined}"
    );
    assert_no_js_output(&dir);
}

/// The transpiler-only sub-options are rejected the same way, so a stale build
/// script gets a real explanation rather than silent misparsing.
#[test]
fn transpiler_sub_options_are_rejected() {
    for flag in ["--target", "--no-runtime", "--es-modules"] {
        let (stdout, stderr, code, dir) = run_cli(&[flag, "node", "<FILE>"]);
        let combined = format!("{stdout}{stderr}");
        assert_eq!(
            code,
            Some(2),
            "{flag} should exit with usage error code 2; got:\n{combined}"
        );
        assert!(
            combined.to_lowercase().contains("transpiler"),
            "expected {flag} to report the transpiler sunset, got:\n{combined}"
        );
        assert_no_js_output(&dir);
    }
}

/// Even the full historical invocation writes nothing and produces no JS.
#[test]
fn full_transpile_invocation_produces_no_javascript() {
    let (stdout, stderr, code, dir) = run_cli(&[
        "--transpile",
        "--target",
        "browser",
        "--es-modules",
        "--output",
        "out.js",
        "<FILE>",
    ]);
    let combined = format!("{stdout}{stderr}");
    assert_eq!(
        code,
        Some(2),
        "historical transpile invocation should fail; got:\n{combined}"
    );
    assert!(
        !dir.path().join("out.js").exists(),
        "no JavaScript output file may be written after the sunset"
    );
    assert_no_js_output(&dir);
}

/// `wfl --help` must not advertise a feature that no longer exists.
#[test]
fn help_output_does_not_mention_transpilation() {
    let (stdout, stderr, _code, _dir) = run_cli(&["--help"]);
    let combined = format!("{stdout}{stderr}").to_lowercase();
    assert!(
        !combined.contains("transpil"),
        "help text still advertises transpilation:\n{combined}"
    );
    for stale in ["--target", "--no-runtime", "--es-modules"] {
        assert!(
            !combined.contains(stale),
            "help text still lists the transpiler-only option {stale}:\n{combined}"
        );
    }
}

/// Removing the transpiler must not disturb ordinary interpretation.
#[test]
fn interpreting_the_same_program_still_works() {
    let (stdout, stderr, code, dir) = run_cli(&["<FILE>"]);
    let combined = format!("{stdout}{stderr}");
    assert_eq!(code, Some(0), "program should still run; got:\n{combined}");
    assert!(
        stdout.contains("Hello"),
        "expected the program's output, got:\n{combined}"
    );
    assert_no_js_output(&dir);
}
