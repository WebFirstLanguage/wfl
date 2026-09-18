//! Lint and fix contracts through the real executable and filesystem.

use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Run the real CLI with isolated configuration and a bounded child lifetime.
/// Drain both pipes concurrently so verbose errors cannot deadlock the child.
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

/// Include both output streams when an exit-status contract fails.
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

/// A later statement's named argument does not turn a bare interface into a body.
#[test]
fn bare_interface_followed_by_named_argument_is_stable_in_every_lint_mode() {
    let source = "define action called build with parameters value:\n    return value\nend action\ncreate interface Marker display call build with value: 1\ndisplay \"done\"\n";
    assert_clean_source_in_every_lint_mode(source);
}

/// Empty-list expressions cannot borrow a later statement's colon as a body.
#[test]
fn empty_list_followed_by_named_argument_is_stable_in_every_lint_mode() {
    let source = "define action called build with parameters value:\n    return value\nend action\nstore items as create list display call build with value: 1\ndisplay items\n";
    assert_clean_source_in_every_lint_mode(source);
}

/// Contextual create/map values in an expression cannot introduce a map body.
#[test]
fn contextual_create_map_values_are_stable_in_every_lint_mode() {
    let source = "define action called build with parameters value:\n    return value\nend action\nstore create as 1\nstore map as 2\ndisplay create map call build with value: 1\ndisplay \"done\"\n";
    assert_clean_source_in_every_lint_mode(source);
}

/// Contextual create/pattern values in an expression cannot introduce a body.
#[test]
fn contextual_create_pattern_values_are_stable_in_every_lint_mode() {
    let source = "define action called build with parameters value:\n    return value\nend action\nstore create as 1\nstore pattern as 2\ndisplay create pattern call build with value: 1\ndisplay \"done\"\n";
    assert_clean_source_in_every_lint_mode(source);
}

/// A parser-supported try variable reference is not the start of an error block.
#[test]
fn contextual_try_expression_is_stable_in_every_lint_mode() {
    assert_clean_source_in_every_lint_mode("store result as try\ndisplay result\n");
}

/// A parser-supported repeat variable reference is not the start of a loop body.
#[test]
fn contextual_repeat_expression_is_stable_in_every_lint_mode() {
    assert_clean_source_in_every_lint_mode("store result as repeat\ndisplay result\n");
}

/// Pattern splitting cannot borrow a later call's named-argument colon as a body.
#[test]
fn pattern_split_expression_is_stable_in_every_lint_mode() {
    let source = "define action called identity with parameters value:\n    return value\nend action\ncreate pattern comma:\n    \",\"\nend pattern\nstore pieces as split \"a,b\" on pattern comma display call identity with value: 1\ndisplay pieces\n";
    assert_clean_source_in_every_lint_mode(source);
}

/// Contextual main/loop operands cannot open a loop from a later call's colon.
#[test]
fn contextual_main_loop_expression_is_stable_in_every_lint_mode() {
    let source = "define action called identity with parameters value:\n    return value\nend action\ndisplay main loop call identity with value: 1\ndisplay \"done\"\n";
    assert_clean_source_in_every_lint_mode(source);
}

/// An explicit action named main followed by a loop operand is not a loop
/// header; lint, source output, diff, and in-place output must all preserve it.
#[test]
fn explicit_main_action_call_is_stable_in_every_lint_mode() {
    let source = "define action called main:\n    return 1\nend action\ndisplay call main loop\ndisplay \"done\"\n";
    assert_clean_source_in_every_lint_mode(source);
}

/// A when variable cannot hide the real check body that follows on the same line.
#[test]
fn contextual_when_before_check_expression_is_stable_in_every_lint_mode() {
    let source =
        "store result as when check if yes:\n    display \"inside\"\nend check\ndisplay \"done\"\n";
    assert_clean_source_in_every_lint_mode(source);
}

/// Registering an event has no body, even before another statement's argument colon.
#[test]
fn bodyless_event_registration_is_stable_in_every_lint_mode() {
    let source = "define action called identity with parameters value:\n    return value\nend action\non 1 clicked display call identity with value: 1\ndisplay \"done\"\n";
    assert_clean_source_in_every_lint_mode(source);
}

/// The `in transaction` operands of a find expression do not introduce a
/// database block in lint, source output, diff preview, or in-place output.
#[test]
fn find_in_transaction_expression_is_stable_in_every_lint_mode() {
    let source = "store transaction as \"haystack\"\nstore needle as \"hay\"\nstore hit as find needle in transaction\ndisplay hit\n";
    assert_clean_source_in_every_lint_mode(source);
}

/// A pattern lookaround's `check` keyword must not indent either its terminator
/// or subsequent statements in lint and all formatter output modes.
#[test]
fn pattern_lookaround_is_stable_in_every_lint_mode() {
    let source =
        "create pattern probe:\n    check ahead for {\"x\"}\nend pattern\ndisplay \"done\"\n";
    assert_clean_source_in_every_lint_mode(source);
}

/// Clean source is warning-free and byte-stable through preview and publication.
fn assert_clean_source_in_every_lint_mode(source: &str) {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("program.wfl");
    fs::write(&path, source).unwrap();

    let lint = run(directory.path(), &["--lint", "program.wfl"]);
    assert_status(&lint, 0);
    assert_eq!(fs::read_to_string(&path).unwrap(), source);

    let fixed = run(directory.path(), &["--lint", "--fix", "program.wfl"]);
    assert_status(&fixed, 0);
    assert_eq!(fixed.stdout, source.as_bytes());
    assert_eq!(fs::read_to_string(&path).unwrap(), source);

    let diff = run(
        directory.path(),
        &["--lint", "--fix", "program.wfl", "--diff"],
    );
    assert_status(&diff, 0);
    assert!(diff.stdout.is_empty());

    let inplace = run(
        directory.path(),
        &["--lint", "--fix", "program.wfl", "--in-place"],
    );
    assert_status(&inplace, 0);
    assert_eq!(fs::read_to_string(&path).unwrap(), source);
}

/// A named argument in a legacy constant is an expression, not an instance body.
#[test]
fn legacy_constant_named_arguments_remain_stable_in_every_lint_mode() {
    let directory = TempDir::new().unwrap();
    let source = "define action called build with parameters value:\n    return value\nend action\ncreate new constant result as call build with value: 1\ndisplay result\n";
    let path = directory.path().join("program.wfl");
    fs::write(&path, source).unwrap();

    let lint = run(directory.path(), &["--lint", "program.wfl"]);
    assert_status(&lint, 0);
    assert_eq!(fs::read_to_string(&path).unwrap(), source);

    let fixed = run(directory.path(), &["--lint", "--fix", "program.wfl"]);
    assert_status(&fixed, 0);
    assert_eq!(fixed.stdout, source.as_bytes());
    assert_eq!(fs::read_to_string(&path).unwrap(), source);

    let diff = run(
        directory.path(),
        &["--lint", "--fix", "program.wfl", "--diff"],
    );
    assert_status(&diff, 0);
    assert!(diff.stdout.is_empty());

    let inplace = run(
        directory.path(),
        &["--lint", "--fix", "program.wfl", "--in-place"],
    );
    assert_status(&inplace, 0);
    assert_eq!(fs::read_to_string(&path).unwrap(), source);
}

/// A single dash belongs to the source name; lint findings must remain read-only.
#[test]
fn lint_accepts_single_dash_source_paths_without_writing() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("-program.wfl");
    fs::write(&path, CLEAN).unwrap();
    let clean = run(dir.path(), &["--lint", "-program.wfl"]);
    assert_status(&clean, 0);
    assert_eq!(fs::read_to_string(&path).unwrap(), CLEAN);

    fs::write(&path, DIRTY).unwrap();
    let dirty = run(dir.path(), &["--lint", "-program.wfl"]);
    assert_status(&dirty, 1);
    assert!(String::from_utf8_lossy(&dirty.stderr).contains("trailing whitespace"));
    assert_eq!(fs::read_to_string(&path).unwrap(), DIRTY);
}

/// Source preview accepts historical dash-leading paths and never rewrites them.
#[test]
fn stdout_fix_accepts_single_dash_source_paths_without_writing() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("-program.wfl");
    fs::write(&path, DIRTY).unwrap();
    let output = run(dir.path(), &["--lint", "--fix", "-program.wfl"]);
    assert_status(&output, 0);
    assert_eq!(output.stdout, CLEAN.as_bytes());
    assert_eq!(fs::read_to_string(&path).unwrap(), DIRTY);
}

/// Patch headers retain the leading dash without treating it as a CLI option.
#[test]
fn diff_accepts_single_dash_source_paths_without_writing() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("-program.wfl");
    fs::write(&path, DIRTY).unwrap();
    let output = run(dir.path(), &["--lint", "--fix", "-program.wfl", "--diff"]);
    assert_status(&output, 0);
    let diff = String::from_utf8_lossy(&output.stdout);
    assert!(diff.starts_with("--- a/-program.wfl\n+++ b/-program.wfl\n"));
    assert!(diff.contains("+display \"hello\"\n"));
    assert_eq!(fs::read_to_string(&path).unwrap(), DIRTY);
}

/// Explicit in-place fixing updates the requested dash-leading file.
#[test]
fn in_place_fix_accepts_single_dash_source_paths() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("-program.wfl");
    fs::write(&path, DIRTY).unwrap();
    let output = run(
        dir.path(),
        &["--lint", "--fix", "-program.wfl", "--in-place"],
    );
    assert_status(&output, 0);
    assert_eq!(fs::read_to_string(&path).unwrap(), CLEAN);
}

/// Preserve the old duplicate-path workaround when the filename starts with a dash.
#[test]
fn legacy_repeated_single_dash_source_after_fix_remains_supported() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("-program.wfl");
    fs::write(&path, DIRTY).unwrap();
    let output = run(
        dir.path(),
        &["--lint", "-program.wfl", "--fix", "-program.wfl", "--diff"],
    );
    assert_status(&output, 0);
    assert!(String::from_utf8_lossy(&output.stdout).contains("+display \"hello\"\n"));
    assert_eq!(fs::read_to_string(&path).unwrap(), DIRTY);
}

/// Short version aliases in the historical source position are literal filenames.
#[test]
fn version_aliases_immediately_after_lint_are_source_filenames() {
    for filename in ["-v", "-V"] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(filename);
        fs::write(&path, DIRTY).unwrap();
        let output = run(dir.path(), &["--lint", filename]);
        assert_status(&output, 1);
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("trailing whitespace"));
        assert_eq!(fs::read_to_string(&path).unwrap(), DIRTY);
    }
}

/// Fix previews and publication honor filenames that coincide with version aliases.
#[test]
fn version_aliases_immediately_after_fix_are_source_filenames() {
    for filename in ["-v", "-V"] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(filename);
        fs::write(&path, DIRTY).unwrap();
        let stdout = run(dir.path(), &["--lint", "--fix", filename]);
        assert_status(&stdout, 0);
        assert_eq!(stdout.stdout, CLEAN.as_bytes());
        let legacy_diff = run(
            dir.path(),
            &["--lint", filename, "--fix", filename, "--diff"],
        );
        assert_status(&legacy_diff, 0);
        assert!(String::from_utf8_lossy(&legacy_diff.stdout).contains("+display \"hello\"\n"));
        assert_eq!(fs::read_to_string(&path).unwrap(), DIRTY);
        let in_place = run(dir.path(), &["--lint", "--fix", filename, "--in-place"]);
        assert_status(&in_place, 0);
        assert_eq!(fs::read_to_string(&path).unwrap(), CLEAN);
    }
}

/// A preview/publication option establishes the source slot even before lint/fix.
#[test]
fn version_alias_paths_follow_any_lint_option_order() {
    for filename in ["-v", "-V"] {
        for mode in ["--diff", "--in-place"] {
            for args in [
                vec![mode, filename, "--fix", "--lint"],
                vec![mode, "--time", filename, "--lint", "--fix"],
                vec!["--time", mode, filename, "--fix", "--lint"],
                vec!["--lint", mode, filename, "--fix"],
                vec!["--fix", mode, filename, "--lint"],
                vec![mode, filename, "--lint", filename, "--fix", filename],
            ] {
                let dir = TempDir::new().unwrap();
                let path = dir.path().join(filename);
                fs::write(&path, DIRTY).unwrap();
                let output = run(dir.path(), &args);
                assert_status(&output, 0);
                assert!(output.stderr.is_empty(), "args: {args:?}");
                if mode == "--diff" {
                    let diff = String::from_utf8_lossy(&output.stdout);
                    assert!(
                        diff.starts_with(&format!("--- a/{filename}\n+++ b/{filename}\n")),
                        "args: {args:?}; output: {diff}"
                    );
                    assert!(diff.contains("-display \"hello\"   \n"));
                    assert!(diff.contains("+display \"hello\"\n"));
                    assert_eq!(fs::read_to_string(&path).unwrap(), DIRTY);
                } else {
                    assert_eq!(
                        String::from_utf8_lossy(&output.stdout),
                        "✔ Auto-fixed 1 issues in place.\n",
                        "args: {args:?}"
                    );
                    assert_eq!(fs::read_to_string(&path).unwrap(), CLEAN);
                }
            }
        }
    }
}

/// Alias-shaped sources cannot bypass option validation or trigger any writes.
#[test]
fn version_alias_paths_preserve_invalid_mode_errors_without_writes() {
    for filename in ["-v", "-V"] {
        for (args, expected) in [
            (vec!["--diff", filename], "requires --fix"),
            (vec!["--in-place", filename], "requires --fix"),
            (vec!["--diff", filename, "--fix"], "--lint"),
            (vec!["--in-place", filename, "--fix"], "--lint"),
            (
                vec!["--diff", filename, "--fix", "--lint", "--in-place"],
                "mutually exclusive",
            ),
            (
                vec!["--in-place", filename, "--lint", "--fix", "--diff"],
                "mutually exclusive",
            ),
            (
                vec!["--diff", filename, "--lint", "--fix", "extra.wfl"],
                "one file",
            ),
            (
                vec!["--in-place", filename, "--lint", "--fix", "--unknown"],
                "Unknown option",
            ),
        ] {
            let dir = TempDir::new().unwrap();
            let path = dir.path().join(filename);
            let other_path = dir.path().join("extra.wfl");
            fs::write(&path, DIRTY).unwrap();
            fs::write(&other_path, DIRTY).unwrap();
            let output = run(dir.path(), &args);
            assert_status(&output, 2);
            assert!(output.stdout.is_empty(), "args: {args:?}");
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(expected),
                "args: {args:?}; stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), DIRTY);
            assert_eq!(fs::read_to_string(&other_path).unwrap(), DIRTY);
        }
    }
}

/// Output-mode options cannot reinterpret a second source as a version request.
#[test]
fn version_alias_paths_after_output_modes_reject_extra_sources_without_writes() {
    for filename in ["-v", "-V"] {
        for mode in ["--diff", "--in-place"] {
            for args in [
                vec!["--lint", "program.wfl", mode, filename, "--fix"],
                vec!["--lint", "--fix", "program.wfl", mode, filename],
                vec!["--lint", filename, "--fix", mode, filename],
            ] {
                let dir = TempDir::new().unwrap();
                let source_path = dir.path().join("program.wfl");
                let alias_path = dir.path().join(filename);
                fs::write(&source_path, DIRTY).unwrap();
                fs::write(&alias_path, DIRTY).unwrap();
                let output = run(dir.path(), &args);
                assert_status(&output, 2);
                assert!(output.stdout.is_empty(), "args: {args:?}");
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("one file"),
                    "args: {args:?}"
                );
                assert_eq!(fs::read_to_string(&source_path).unwrap(), DIRTY);
                assert_eq!(fs::read_to_string(&alias_path).unwrap(), DIRTY);
            }
        }
    }
}

/// Missing or malformed alias-shaped inputs remain errors in both fix modes.
#[test]
fn version_alias_paths_preserve_source_errors_without_writes() {
    for filename in ["-v", "-V"] {
        for mode in ["--diff", "--in-place"] {
            for source in [None, Some("display \"hello\"\n@\n")] {
                let dir = TempDir::new().unwrap();
                let path = dir.path().join(filename);
                if let Some(source) = source {
                    fs::write(&path, source).unwrap();
                }
                let output = run(dir.path(), &[mode, filename, "--fix", "--lint"]);
                assert_status(&output, 2);
                assert!(output.stdout.is_empty());
                assert!(!output.stderr.is_empty());
                if let Some(source) = source {
                    assert_eq!(fs::read_to_string(&path).unwrap(), source);
                } else {
                    assert!(!path.exists());
                }
            }
        }
    }
}

/// Standalone aliases and explicit long version requests retain their meaning.
#[test]
fn version_alias_paths_do_not_change_explicit_version_requests() {
    let dir = TempDir::new().unwrap();
    for filename in ["-v", "-V", "program.wfl"] {
        fs::write(dir.path().join(filename), DIRTY).unwrap();
    }
    for args in [
        vec!["-v"],
        vec!["-V"],
        vec!["--version"],
        vec!["--time", "-v"],
        vec!["--time", "-V"],
        vec!["--diff", "--version", "--fix", "--lint"],
        vec!["--lint", "program.wfl", "--in-place", "--version", "--fix"],
    ] {
        let output = run(dir.path(), &args);
        assert_status(&output, 0);
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            format!(
                "WebFirst Language (WFL) version {}\n",
                wfl::version::VERSION
            ),
            "args: {args:?}"
        );
        assert!(output.stderr.is_empty(), "args: {args:?}");
        for filename in ["-v", "-V", "program.wfl"] {
            assert_eq!(
                fs::read_to_string(dir.path().join(filename)).unwrap(),
                DIRTY
            );
        }
    }
}

/// Lint status distinguishes clean input from warnings without changing either.
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

/// Redirected fix output is executable source with no banners or diagnostic noise.
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

/// Flag ordering must not change the patch or turn a preview into a write.
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

/// Published fixes satisfy lint and reach a stable state with no second patch.
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

/// Parser and lexer failures cannot publish partial or recovered source.
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

/// Invalid combinations fail before executing, launching an editor, or writing.
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

/// Missing and non-UTF-8 inputs are errors, never candidates for replacement.
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

/// A refused publication preserves bytes, readonly permissions, and directory contents.
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

/// Every lint/fix mode enforces the configured source-size limit before output.
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

/// A configured indentation width must make lint and the formatter agree.
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

/// Executable scripts retain their argument boundary even for lint-like strings.
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

/// Pathological indentation settings fail safely instead of overflowing allocations.
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

/// The only working fix spelling in older releases remains a supported invocation.
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

/// A real patch tool must apply and reverse emitted diffs byte-for-byte, including
/// nested filenames with spaces, CRLF, and a missing final newline.
#[test]
fn emitted_diff_applies_and_reverses_without_losing_line_endings() {
    // Resolve the executable before changing the child's directory. This also
    // works on Windows hosts that supply Git through a bundled runtime PATH.
    let git_name = if cfg!(windows) { "git.exe" } else { "git" };
    let git_path = std::env::split_paths(&std::env::var_os("PATH").expect("PATH is set"))
        .map(|directory| directory.join(git_name))
        .find(|candidate| candidate.is_file())
        .expect("Git is required to verify the CLI's unified diff contract");
    for source in [
        "display \"hello\"   \n",
        "display \"hello\"   \r\n",
        "display \"hello\"   ",
        "// keep this comment\r\ndisplay \"hello\"   \r\n",
    ] {
        let dir = TempDir::new().unwrap();
        let relative_path = "nested folder/program with spaces.wfl";
        let path = dir.path().join(relative_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, source).unwrap();
        let fixed = run(dir.path(), &["--lint", "--fix", relative_path]);
        assert_status(&fixed, 0);
        let diff = run(dir.path(), &["--lint", "--fix", relative_path, "--diff"]);
        assert_status(&diff, 0);
        assert_eq!(fs::read_to_string(&path).unwrap(), source);
        fs::write(dir.path().join("changes.diff"), diff.stdout).unwrap();
        for reverse in [false, true] {
            let mut git = Command::new(&git_path);
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

/// Disabling a style rule suppresses both its lint warning and its source rewrite.
#[test]
fn lint_rule_configuration_is_shared_with_fixes() {
    for (setting, source) in [
        (
            "snake_case_variables = false\n",
            "store BadName as 1\ndisplay BadName\n",
        ),
        ("trailing_whitespace = true\n", DIRTY),
        ("consistent_keyword_case = false\n", "display TRUE\n"),
    ] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("program.wfl");
        fs::write(&path, source).unwrap();
        let defaults = run(dir.path(), &["--lint", "program.wfl"]);
        assert_status(&defaults, 1);
        fs::write(dir.path().join(".wflcfg"), setting).unwrap();
        let configured = run(dir.path(), &["--lint", "program.wfl"]);
        assert_status(&configured, 0);
        let fixed = run(dir.path(), &["--lint", "--fix", "program.wfl"]);
        assert_status(&fixed, 0);
        assert_eq!(fixed.stdout, source.as_bytes(), "setting: {setting}");
        let diff = run(dir.path(), &["--lint", "--fix", "program.wfl", "--diff"]);
        assert_status(&diff, 0);
        assert!(diff.stdout.is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), source);
    }
}
