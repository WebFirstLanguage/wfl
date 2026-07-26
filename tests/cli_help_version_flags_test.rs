//! CLI contract for the help and version flags, including their short aliases.
//!
//! `main()` already classifies `-h` and `-V` as trivial, non-interpreting
//! invocations (they skip the large-stack reservation), but the argument parser
//! in `run()` only ever recognized the long forms plus `-v`. That left `-h` and
//! `-V` to fall through to the generic argument arm, where they were taken as an
//! input file path and reported a baffling "No such file or directory".
//!
//! These tests pin every documented spelling to its intended behavior.

use std::process::Command;
use tempfile::TempDir;

fn wfl_exe() -> &'static str {
    env!("CARGO_BIN_EXE_wfl")
}

/// Run `wfl <flag>` in an empty temp dir, returning (stdout, stderr, exit code).
fn run_flag(flag: &str) -> (String, String, Option<i32>) {
    let dir = TempDir::new().expect("tempdir");
    let output = Command::new(wfl_exe())
        .arg(flag)
        .current_dir(dir.path())
        .output()
        .expect("failed to execute WFL");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code(),
    )
}

/// Every help spelling prints the usage banner and exits cleanly.
#[test]
fn help_flags_print_usage() {
    for flag in ["--help", "-h"] {
        let (stdout, stderr, code) = run_flag(flag);
        let combined = format!("{stdout}{stderr}");
        assert_eq!(code, Some(0), "{flag} should exit 0; got:\n{combined}");
        assert!(
            stdout.contains("USAGE:") && stdout.contains("WebFirst Language"),
            "{flag} should print the usage banner, got:\n{combined}"
        );
    }
}

/// Every version spelling prints the version line and exits cleanly.
#[test]
fn version_flags_print_version() {
    let expected = wfl::version::VERSION;
    for flag in ["--version", "-v", "-V"] {
        let (stdout, stderr, code) = run_flag(flag);
        let combined = format!("{stdout}{stderr}");
        assert_eq!(code, Some(0), "{flag} should exit 0; got:\n{combined}");
        assert!(
            stdout.contains(expected),
            "{flag} should print version {expected}, got:\n{combined}"
        );
    }
}

/// The regression itself: no help/version spelling may be mistaken for a file
/// path. That misparse is what produced the confusing "No such file or
/// directory" for `-h` and `-V`.
#[test]
fn help_and_version_flags_are_never_treated_as_file_paths() {
    for flag in ["--help", "-h", "--version", "-v", "-V"] {
        let (stdout, stderr, code) = run_flag(flag);
        let combined = format!("{stdout}{stderr}");
        assert!(
            !combined.contains("No such file or directory"),
            "{flag} was misparsed as an input file path, got:\n{combined}"
        );
        assert_eq!(code, Some(0), "{flag} should exit 0; got:\n{combined}");
    }
}
