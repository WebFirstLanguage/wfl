//! Lint and fix contracts through the real executable and filesystem.

use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn run(dir: &Path, args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .args(args)
        .current_dir(dir)
        .env("WFL_GLOBAL_CONFIG_PATH", dir.join("absent-global-config"))
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn WFL");
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let out = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let err = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("WFL did not exit within 20 seconds: {args:?}");
        }
        thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: out.join().unwrap(),
        stderr: err.join().unwrap(),
    }
}

fn assert_status(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

const DIRTY: &str = "display \"hello\"   \n";
const CLEAN: &str = "display \"hello\"\n";

#[test]
fn lint_reports_clean_and_dirty_files_without_changing_them() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("program.wfl");
    fs::write(&path, CLEAN).unwrap();
    let clean = run(dir.path(), &["--lint", "program.wfl"]);
    assert_status(&clean, 0);
    assert_eq!(
        String::from_utf8_lossy(&clean.stdout),
        "No lint warnings found.\n"
    );
    assert!(clean.stderr.is_empty());
    assert_eq!(fs::read_to_string(&path).unwrap(), CLEAN);

    fs::write(&path, DIRTY).unwrap();
    let dirty = run(dir.path(), &["--lint", "program.wfl"]);
    assert_status(&dirty, 1);
    assert!(dirty.stdout.is_empty());
    assert!(String::from_utf8_lossy(&dirty.stderr).contains("trailing whitespace"));
    assert_eq!(fs::read_to_string(&path).unwrap(), DIRTY);
}

#[test]
fn fix_stdout_is_only_valid_source_and_does_not_change_the_input() {
    for args in [
        vec!["--lint", "--fix", "program.wfl"],
        vec!["--lint", "program.wfl", "--fix"],
        vec!["--fix", "program.wfl", "--lint"],
    ] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("program.wfl");
        fs::write(&path, DIRTY).unwrap();
        let output = run(dir.path(), &args);
        assert_status(&output, 0);
        assert_eq!(output.stdout, CLEAN.as_bytes(), "args: {args:?}");
        assert!(output.stderr.is_empty(), "args: {args:?}");
        assert_eq!(fs::read_to_string(&path).unwrap(), DIRTY);
        fs::write(dir.path().join("fixed.wfl"), &output.stdout).unwrap();
        let execution = run(dir.path(), &["fixed.wfl"]);
        assert_status(&execution, 0);
        assert_eq!(String::from_utf8_lossy(&execution.stdout).trim(), "hello");
    }
}

#[test]
fn diff_accepts_flag_orderings_and_never_writes_the_file() {
    for args in [
        vec!["--lint", "--fix", "program.wfl", "--diff"],
        vec!["--lint", "program.wfl", "--fix", "--diff"],
        vec!["--diff", "--fix", "--lint", "program.wfl"],
        vec!["--fix", "--diff", "program.wfl", "--lint"],
    ] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("program.wfl");
        fs::write(&path, DIRTY).unwrap();
        let output = run(dir.path(), &args);
        assert_status(&output, 0);
        let diff = String::from_utf8_lossy(&output.stdout);
        assert!(diff.starts_with("--- "), "args: {args:?}; output: {diff}");
        assert!(diff.contains("\n+++ ") && diff.contains("\n@@ "));
        assert!(diff.contains("-display \"hello\"   \n"));
        assert!(diff.contains("+display \"hello\"\n"));
        assert!(output.stderr.is_empty(), "args: {args:?}");
        assert_eq!(fs::read_to_string(&path).unwrap(), DIRTY);
    }
}

#[test]
fn in_place_fixes_once_and_a_second_diff_is_empty() {
    for args in [
        vec!["--lint", "--fix", "program.wfl", "--in-place"],
        vec!["--lint", "program.wfl", "--in-place", "--fix"],
        vec!["--in-place", "--fix", "--lint", "program.wfl"],
    ] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("program.wfl");
        fs::write(&path, DIRTY).unwrap();
        let output = run(dir.path(), &args);
        assert_status(&output, 0);
        assert_eq!(fs::read_to_string(&path).unwrap(), CLEAN, "args: {args:?}");
        let lint = run(dir.path(), &["--lint", "program.wfl"]);
        assert_status(&lint, 0);
        let diff = run(dir.path(), &["--lint", "--fix", "program.wfl", "--diff"]);
        assert_status(&diff, 0);
        assert!(diff.stdout.is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), CLEAN);
    }
}

#[test]
fn malformed_source_never_produces_a_fix_or_changes_the_input() {
    for source in ["check if true:\n", "display \"hello\"\n@\n"] {
        for extra in [
            vec![],
            vec!["--fix"],
            vec!["--fix", "--diff"],
            vec!["--fix", "--in-place"],
        ] {
            let dir = TempDir::new().unwrap();
            let path = dir.path().join("program.wfl");
            fs::write(&path, source).unwrap();
            let mut args = vec!["--lint", "program.wfl"];
            args.extend(extra);
            let output = run(dir.path(), &args);
            assert_status(&output, 2);
            assert!(output.stdout.is_empty(), "args: {args:?}");
            assert!(!output.stderr.is_empty());
            assert_eq!(fs::read_to_string(&path).unwrap(), source);
        }
    }
}

#[test]
fn invalid_lint_options_are_usage_errors_without_writes() {
    for (args, expected) in [
        (vec!["--lint", "program.wfl", "--diff"], "requires --fix"),
        (
            vec!["--lint", "program.wfl", "--in-place"],
            "requires --fix",
        ),
        (vec!["--diff", "program.wfl"], "requires --fix"),
        (vec!["--in-place", "program.wfl"], "requires --fix"),
        (vec!["--fix", "program.wfl"], "--lint"),
        (
            vec!["--lint", "--fix", "program.wfl", "--diff", "--in-place"],
            "mutually exclusive",
        ),
        (
            vec!["--in-place", "--lint", "--fix", "program.wfl", "--diff"],
            "mutually exclusive",
        ),
        (vec!["--lint", "--fix"], "file path"),
        (vec!["--lint", "program.wfl", "extra.wfl"], "one file"),
        (vec!["--lint", "program.wfl", "--unknown"], "Unknown option"),
        (
            vec!["--step", "--lint", "program.wfl"],
            "cannot be combined",
        ),
        (
            vec!["--test", "--lint", "program.wfl"],
            "cannot be combined",
        ),
        (vec!["--lex", "--lint", "program.wfl"], "cannot be combined"),
        (
            vec!["--edit", "program.wfl", "--lint"],
            "cannot be combined",
        ),
        (
            vec!["--dump-env", "--lint", "program.wfl"],
            "cannot be combined",
        ),
    ] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("program.wfl");
        fs::write(&path, DIRTY).unwrap();
        let output = run(dir.path(), &args);
        assert_status(&output, 2);
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "args: {args:?}; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty(), "args: {args:?}");
        assert_eq!(fs::read_to_string(&path).unwrap(), DIRTY);
    }
}

#[test]
fn lint_input_io_errors_exit_two_without_creating_files() {
    let dir = TempDir::new().unwrap();
    for args in [
        vec!["--lint", "missing.wfl"],
        vec!["--lint", "--fix", "missing.wfl", "--in-place"],
    ] {
        let output = run(dir.path(), &args);
        assert_status(&output, 2);
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert!(!dir.path().join("missing.wfl").exists());
    }
    let path = dir.path().join("invalid-utf8.wfl");
    fs::write(&path, [0xff, 0xfe]).unwrap();
    let output = run(dir.path(), &["--lint", "invalid-utf8.wfl"]);
    assert_status(&output, 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains("UTF-8"));
    assert_eq!(fs::read(&path).unwrap(), [0xff, 0xfe]);
}

#[test]
fn in_place_write_failure_preserves_readonly_source_and_permissions() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("program.wfl");
    fs::write(&path, DIRTY).unwrap();
    let original_permissions = fs::metadata(&path).unwrap().permissions();
    let mut readonly_permissions = original_permissions.clone();
    readonly_permissions.set_readonly(true);
    fs::set_permissions(&path, readonly_permissions).unwrap();

    let output = run(
        dir.path(),
        &["--lint", "--fix", "program.wfl", "--in-place"],
    );
    let contents = fs::read_to_string(&path).unwrap();
    let still_readonly = fs::metadata(&path).unwrap().permissions().readonly();
    // Restore before assertions so the temporary tree is removable on Windows
    // even if a regression causes one of the assertions below to fail.
    fs::set_permissions(&path, original_permissions).unwrap();
    assert_status(&output, 2);
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    assert_eq!(contents, DIRTY);
    assert!(still_readonly);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn oversized_source_is_rejected_in_every_mode_without_output_or_writes() {
    for args in [
        vec!["--lint", "program.wfl"],
        vec!["--lint", "--fix", "program.wfl"],
        vec!["--lint", "--fix", "program.wfl", "--diff"],
        vec!["--lint", "--fix", "program.wfl", "--in-place"],
    ] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("program.wfl");
        fs::write(&path, DIRTY).unwrap();
        fs::write(dir.path().join(".wflcfg"), "max_source_size = 8\n").unwrap();
        let output = run(dir.path(), &args);
        assert_status(&output, 2);
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("Source file too large"));
        assert_eq!(fs::read_to_string(&path).unwrap(), DIRTY);
    }
}

#[test]
fn lint_and_fix_share_project_indentation_settings() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("program.wfl");
    fs::write(&path, "check if true:\n display \"hello\"\nend check\n").unwrap();
    fs::write(dir.path().join(".wflcfg"), "indent_size = 2\n").unwrap();
    let before = run(dir.path(), &["--lint", "program.wfl"]);
    assert_status(&before, 1);
    let fix = run(
        dir.path(),
        &["--lint", "--fix", "program.wfl", "--in-place"],
    );
    assert_status(&fix, 0);
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "check if true:\n  display \"hello\"\nend check\n"
    );
    let after = run(dir.path(), &["--lint", "program.wfl"]);
    assert_status(&after, 0);
}

#[test]
fn lint_like_script_arguments_remain_arguments_to_the_executed_program() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("program.wfl");
    fs::write(&path, DIRTY).unwrap();
    let output = run(
        dir.path(),
        &["program.wfl", "--lint", "--fix", "--diff", "--in-place"],
    );
    assert_status(&output, 0);
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
    assert_eq!(fs::read_to_string(&path).unwrap(), DIRTY);
}

#[test]
fn extreme_indentation_configuration_cannot_panic_or_overwrite_source() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("program.wfl");
    let source = "check if true:\n    display \"hello\"\nend check\n";
    fs::write(&path, source).unwrap();
    fs::write(
        dir.path().join(".wflcfg"),
        format!("indent_size = {}\n", usize::MAX),
    )
    .unwrap();
    let lint = run(dir.path(), &["--lint", "program.wfl"]);
    assert_status(&lint, 1);
    for args in [
        vec!["--lint", "--fix", "program.wfl"],
        vec!["--lint", "--fix", "program.wfl", "--diff"],
        vec!["--lint", "--fix", "program.wfl", "--in-place"],
    ] {
        let output = run(dir.path(), &args);
        assert_status(&output, 2);
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"));
        assert_eq!(fs::read_to_string(&path).unwrap(), source);
    }
}

#[test]
fn legacy_repeated_source_after_fix_remains_supported() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("program.wfl");
    fs::write(&path, DIRTY).unwrap();
    let output = run(
        dir.path(),
        &["--lint", "program.wfl", "--fix", "program.wfl", "--diff"],
    );
    assert_status(&output, 0);
    assert!(String::from_utf8_lossy(&output.stdout).contains("+display \"hello\"\n"));
    assert_eq!(fs::read_to_string(&path).unwrap(), DIRTY);
}

#[test]
fn emitted_diff_applies_and_reverses_without_losing_line_endings() {
    for source in [
        "display \"hello\"   \n",
        "display \"hello\"   \r\n",
        "display \"hello\"   ",
        "// keep this comment\r\ndisplay \"hello\"   \r\n",
    ] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("source.wfl");
        fs::write(&path, source).unwrap();
        let fixed = run(dir.path(), &["--lint", "--fix", "source.wfl"]);
        assert_status(&fixed, 0);
        let diff = run(dir.path(), &["--lint", "--fix", "source.wfl", "--diff"]);
        assert_status(&diff, 0);
        assert_eq!(fs::read_to_string(&path).unwrap(), source);
        fs::write(dir.path().join("changes.diff"), diff.stdout).unwrap();
        for reverse in [false, true] {
            let mut git = Command::new("git");
            git.args([
                "-c",
                "core.autocrlf=false",
                "apply",
                "--no-index",
                "--whitespace=nowarn",
            ])
            .current_dir(dir.path());
            if reverse {
                git.arg("--reverse");
            }
            let output = git
                .arg("changes.diff")
                .output()
                .expect("Git is required to verify the CLI's unified diff contract");
            assert_status(&output, 0);
            let expected = if reverse {
                source.as_bytes()
            } else {
                &fixed.stdout
            };
            assert_eq!(fs::read(&path).unwrap(), expected, "reverse={reverse}");
        }
    }
}
