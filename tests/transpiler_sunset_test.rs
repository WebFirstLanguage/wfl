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

mod common;
use common::wfl_exe;

const SAMPLE: &str = "store greeting as \"Hello\"\ndisplay greeting\n";

/// Run the `wfl` binary with `args` inside a fresh temp dir that contains
/// `main.wfl`, returning (stdout, stderr, exit code, temp dir).
fn run_cli(args: &[&str]) -> (String, String, Option<i32>, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("main.wfl");
    fs::write(&path, SAMPLE).unwrap();
    // A writable subdirectory, so a nested `--output` path is a real place the
    // binary could emit to and `assert_no_js_output` has depth to search.
    fs::create_dir(dir.path().join("nested")).unwrap();

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
///
/// Walks nested directories too — an output path like `nested/out.js` must not
/// slip past this check just because it is not at the top level.
fn assert_no_js_output(dir: &TempDir) {
    fn walk(path: &std::path::Path) {
        for entry in fs::read_dir(path).expect("read temp dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path);
                continue;
            }
            // Compare case-insensitively: `out.JS` is just as much a leaked
            // artifact as `out.js`, and on a case-insensitive filesystem they
            // are the same file.
            let is_js = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("js"));
            assert!(
                !is_js,
                "transpiler is removed, but a JavaScript file was produced: {}",
                path.display()
            );
        }
    }
    walk(dir.path());
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
    // `--output` outlived the transpiler (it still serves --dump-env), so the
    // error has to say so rather than leave a former user guessing whether it
    // still produces JavaScript.
    assert!(
        combined.contains("--output"),
        "expected the error to say what became of --output, got:\n{combined}"
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
        "nested/out.js",
        "<FILE>",
    ]);
    let combined = format!("{stdout}{stderr}");
    assert_eq!(
        code,
        Some(2),
        "historical transpile invocation should fail; got:\n{combined}"
    );
    assert!(
        !dir.path().join("nested").join("out.js").exists(),
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
