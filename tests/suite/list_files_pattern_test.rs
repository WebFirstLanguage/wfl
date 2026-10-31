//! `list files in <dir> with pattern <glob>` (issue #578).
//!
//! The documented contract (`Docs/05-standard-library/filesystem-module.md`) is
//! that `with pattern` takes a glob such as `"*.wfl"`. These tests drive the
//! real `wfl` binary against a real temp directory and assert the exact file
//! names returned, not just that the program ran.

use std::fs;
use std::process::Command;
use tempfile::{NamedTempFile, TempDir};

use crate::common::wfl_exe;

/// Fixture layout shared by most tests: three `.txt` files, one `.log`, a
/// directory whose name looks like a `.txt` file, and a nested `.txt`.
const FIXTURE: &[&str] = &[
    "a.txt",
    "b.txt",
    "report.txt",
    "c.log",
    "folder.txt/inside.log",
    "sub/deep.txt",
];

/// Create `files` under a fresh temp dir, run `src` with that dir as the
/// working directory, and return (stdout + stderr, exit code).
fn run_in_fixture(files: &[&str], src: &str) -> (String, Option<i32>) {
    let dir = TempDir::new().expect("tempdir");
    for name in files {
        let path = dir.path().join(name);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("mkdir");
        fs::write(&path, "x\n").expect("write fixture file");
    }
    // The program lives outside the listed directory so it is never listed.
    let program = NamedTempFile::with_suffix(".wfl").expect("program tempfile");
    fs::write(program.path(), src).expect("write program");
    // Pin an empty global config so a machine-wide one cannot change limits.
    let global_cfg = NamedTempFile::new().expect("empty global-config tempfile");
    let output = Command::new(wfl_exe())
        .arg(program.path())
        .current_dir(dir.path())
        .env("WFL_GLOBAL_CONFIG_PATH", global_cfg.path())
        .output()
        .expect("failed to execute WFL");
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (combined, output.status.code())
}

/// WFL that stores `listing` and prints one `FILE=<path>` line per entry.
fn display_listing(listing: &str) -> String {
    format!(
        "store listed as {listing}\n\
         for each listed_item in listed:\n    \
             display \"FILE=\" with listed_item\n\
         end for\n\
         display \"DONE\"\n"
    )
}

/// Sorted final path components of every `FILE=` line, so assertions do not
/// depend on directory order or the platform's path separator.
fn listed_names(out: &str) -> Vec<String> {
    let mut names: Vec<String> = out
        .lines()
        .filter_map(|line| line.strip_prefix("FILE="))
        .map(|path| path.rsplit(['/', '\\']).next().unwrap_or(path).to_string())
        .collect();
    names.sort();
    names
}

/// Run `listing` over [`FIXTURE`] and assert it returns exactly `expected`.
fn assert_listing(listing: &str, expected: &[&str]) {
    let (out, code) = run_in_fixture(FIXTURE, &display_listing(listing));
    assert_eq!(code, Some(0), "`{listing}` should exit 0: {out}");
    assert!(out.contains("DONE"), "`{listing}` did not finish: {out}");
    assert_eq!(listed_names(&out), expected, "`{listing}` output: {out}");
}

#[test]
fn star_glob_returns_matching_files_only() {
    // Excludes `c.log`, the `folder.txt` directory, and nested files.
    assert_listing(
        r#"list files in "." with pattern "*.txt""#,
        &["a.txt", "b.txt", "report.txt"],
    );
}

#[test]
fn exact_file_name_pattern_returns_that_file() {
    assert_listing(
        r#"list files in "." with pattern "report.txt""#,
        &["report.txt"],
    );
}

#[test]
fn question_mark_and_set_globs_match_single_characters() {
    assert_listing(
        r#"list files in "." with pattern "?.txt""#,
        &["a.txt", "b.txt"],
    );
    assert_listing(
        r#"list files in "." with pattern "[ab].*""#,
        &["a.txt", "b.txt"],
    );
}

#[test]
fn star_glob_includes_hidden_files_like_the_glob_builtin() {
    // `glob of "*.txt" and "."` and `with extension ".txt"` both include
    // dot-files, so `with pattern` does too.
    let (out, code) = run_in_fixture(
        &["a.txt", ".hidden.txt", "c.log"],
        &display_listing(r#"list files in "." with pattern "*.txt""#),
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
    assert_eq!(
        listed_names(&out),
        [".hidden.txt", "a.txt"],
        "output: {out}"
    );
}

#[test]
fn pattern_held_in_a_variable_is_used_as_a_glob() {
    let (out, code) = run_in_fixture(
        FIXTURE,
        &format!(
            "store wanted as \"*.log\"\n{}",
            display_listing(r#"list files in "." with pattern wanted"#)
        ),
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
    assert_eq!(listed_names(&out), ["c.log"], "output: {out}");
}

#[test]
fn recursive_listing_with_pattern_searches_subdirectories() {
    // The pattern is matched against each file's name, at any depth.
    assert_listing(
        r#"list files recursively in "." with pattern "*.txt""#,
        &["a.txt", "b.txt", "deep.txt", "report.txt"],
    );
    assert_listing(
        r#"list files in "." recursively with pattern "*.log""#,
        &["c.log", "inside.log"],
    );
}

#[test]
fn pattern_cannot_reach_outside_the_listed_directory() {
    // `a.txt` exists one level up, but only names inside `sub` are compared.
    for pattern in ["../*", "../a.txt"] {
        let listing = format!(r#"list files in "sub" with pattern "{pattern}""#);
        assert_listing(&listing, &[]);
    }
}

#[test]
fn invalid_pattern_is_a_clear_catchable_error() {
    // An unterminated `[` set is not a valid glob. Silently returning an empty
    // list would hide the mistake, so it must raise a catchable error.
    let (out, code) = run_in_fixture(
        FIXTURE,
        "try:\n    \
             store listed as list files in \".\" with pattern \"[abc\"\n    \
             display \"UNREACHABLE\"\n\
         catch:\n    \
             display \"CAUGHT\"\n\
         end try\n",
    );
    assert_eq!(code, Some(0), "the error should be catchable: {out}");
    assert!(
        out.contains("CAUGHT"),
        "expected the error to be caught: {out}"
    );
    assert!(
        !out.contains("UNREACHABLE"),
        "listing must not succeed: {out}"
    );

    let (out, code) = run_in_fixture(
        FIXTURE,
        &display_listing(r#"list files in "." with pattern "[abc""#),
    );
    assert_ne!(
        code,
        Some(0),
        "an uncaught invalid pattern must fail: {out}"
    );
    assert!(
        out.contains("Invalid file pattern") && out.contains("[abc"),
        "the error should name the invalid pattern: {out}"
    );
    assert!(
        !out.contains("DONE"),
        "the program must stop at the error: {out}"
    );
}

// --- Backward compatibility: these forms already work and must not change. --

#[test]
fn extension_filter_is_unchanged() {
    assert_listing(
        r#"list files in "." with extension ".txt""#,
        &["a.txt", "b.txt", "report.txt"],
    );
    assert_listing(
        r#"list files in "." with extensions [".txt" and ".log"]"#,
        &["a.txt", "b.txt", "c.log", "report.txt"],
    );
    assert_listing(
        r#"list files recursively in "." with extension ".log""#,
        &["c.log", "inside.log"],
    );
}

#[test]
fn bare_extension_pattern_still_matches_by_extension() {
    // Before #578 was fixed, `with pattern ".txt"` behaved like
    // `with extension ".txt"`. Programs relying on that keep working.
    assert_listing(
        r#"list files in "." with pattern ".txt""#,
        &["a.txt", "b.txt", "report.txt"],
    );
}
