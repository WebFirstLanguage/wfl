//! CLI and runtime contract after the `wflpkg` package manager was removed.
//!
//! WFL shipped an in-tree package manager (the `wflpkg` crate) that reached the
//! language in two places: a set of positional `wfl` subcommands
//! (`create`/`add`/`share`/`login`/…) and a `package:` prefix understood by
//! `load module from` / `include from`, which resolved through
//! `packages/<name>/` next to a `project.wfl` manifest.
//!
//! That system is being redesigned from scratch, so it was removed wholesale
//! before the first RC. These tests pin the post-removal behavior:
//!
//! * `package:` is no longer a protocol — it is just an ordinary (unresolvable)
//!   relative path, and none of the package manager's guidance strings survive.
//! * the positional subcommands are gone, including the `wfl run <file>` and
//!   `wfl test <file>` aliases that lived inside the same dispatch block.
//! * the file-based module system that `package:` was bolted onto is untouched.

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn wfl_exe() -> &'static str {
    env!("CARGO_BIN_EXE_wfl")
}

/// Run `wfl <args...>` with `dir` as the working directory.
/// Returns (stdout, stderr, exit code).
fn run_in(dir: &Path, args: &[&str]) -> (String, String, Option<i32>) {
    let output = Command::new(wfl_exe())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute WFL");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code(),
    )
}

/// Run a script by absolute path, with `dir` as the working directory.
///
/// The absolute path matters for the package tests: the removed
/// `find_project_root` walked up from the *source file's* parent directory, so
/// a bare `main.wfl` argument gave it an empty parent and it bailed out before
/// ever consulting `packages/`. Passing the absolute path exercises the
/// resolver for real, which is what makes these tests fail against the old
/// code for the intended reason rather than by accident.
fn run_script(dir: &Path, name: &str) -> (String, String, Option<i32>) {
    let script = dir.join(name);
    let script = script.to_str().expect("utf-8 temp path");
    run_in(dir, &[script])
}

/// Build the exact project layout the removed resolver resolved successfully:
/// a `project.wfl` manifest at the root (so the old `find_project_root` walk
/// succeeded) and an installed package at `packages/demo/main.wfl` (the old
/// root-`main.wfl` entry-point fallback).
fn project_with_installed_package(dir: &Path) {
    fs::write(
        dir.join("project.wfl"),
        "name is demo-app\nversion is 26.1.1\ndescription is Test\nentry is main.wfl\n",
    )
    .expect("write project.wfl");

    let pkg_dir = dir.join("packages").join("demo");
    fs::create_dir_all(&pkg_dir).expect("create packages/demo");
    fs::write(
        pkg_dir.join("main.wfl"),
        "display \"package module loaded\"\n",
    )
    .expect("write packages/demo/main.wfl");
}

/// A `package:` import must fail as an ordinary unresolvable path, even when a
/// project manifest and a fully installed package are sitting right there.
#[test]
fn package_protocol_no_longer_resolves() {
    let dir = TempDir::new().expect("tempdir");
    project_with_installed_package(dir.path());
    fs::write(
        dir.path().join("main.wfl"),
        "load module from \"package:demo\"\ndisplay \"main finished\"\n",
    )
    .expect("write main.wfl");

    let (stdout, stderr, code) = run_script(dir.path(), "main.wfl");
    let combined = format!("{stdout}{stderr}");

    assert_ne!(
        code,
        Some(0),
        "a package: import must not succeed; got:\n{combined}"
    );
    assert!(
        combined.contains("Cannot resolve module path"),
        "a package: import should fail as a plain unresolvable path; got:\n{combined}"
    );
    assert!(
        !stdout.contains("package module loaded"),
        "the installed package must not have been executed; got:\n{combined}"
    );
}

/// Negative assertion: none of the package manager's guidance strings may
/// survive in the failure path. Their presence would mean a wflpkg error branch
/// is still wired into the interpreter.
#[test]
fn package_import_failure_mentions_no_package_manager() {
    let dir = TempDir::new().expect("tempdir");
    project_with_installed_package(dir.path());
    fs::write(
        dir.path().join("main.wfl"),
        "load module from \"package:demo\"\n",
    )
    .expect("write main.wfl");

    let (stdout, stderr, code) = run_script(dir.path(), "main.wfl");
    let combined = format!("{stdout}{stderr}");

    // Guard against a vacuous pass: if the import ever succeeds again there is
    // no failure text to inspect, and the loop below would assert nothing.
    assert_ne!(
        code,
        Some(0),
        "expected the package: import to fail so its message can be checked; got:\n{combined}"
    );

    for forbidden in [
        "project.wfl",
        "wfl create project",
        "wfl add",
        "is not installed",
        "packages directory",
    ] {
        assert!(
            !combined.contains(forbidden),
            "removed package-manager guidance {forbidden:?} still appears in:\n{combined}"
        );
    }
}

/// An empty `package:` prefix gets no special diagnostic either.
#[test]
fn bare_package_prefix_is_not_special_cased() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("main.wfl"),
        "load module from \"package:\"\n",
    )
    .expect("write main.wfl");

    let (stdout, stderr, code) = run_script(dir.path(), "main.wfl");
    let combined = format!("{stdout}{stderr}");

    assert_ne!(code, Some(0), "expected failure; got:\n{combined}");
    assert!(
        !combined.contains("requires a package name"),
        "the package: protocol diagnostic should be gone; got:\n{combined}"
    );
}

/// The positional package-manager subcommands are gone. Each is now treated as
/// an ordinary (missing) input file path.
#[test]
fn package_subcommands_are_removed() {
    for args in [
        vec!["create", "project"],
        vec!["add", "some-lib"],
        vec!["remove", "some-lib"],
        vec!["update"],
        vec!["build"],
        vec!["share"],
        vec!["search", "http"],
        vec!["info", "some-lib"],
        vec!["login"],
        vec!["logout"],
        vec!["check", "security"],
    ] {
        let dir = TempDir::new().expect("tempdir");
        let (stdout, stderr, code) = run_in(dir.path(), &args);
        let combined = format!("{stdout}{stderr}");

        assert_ne!(
            code,
            Some(0),
            "`wfl {}` should no longer be a subcommand; got:\n{combined}",
            args.join(" ")
        );
        assert!(
            !dir.path().join("project.wfl").exists(),
            "`wfl {}` must not create a project manifest",
            args.join(" ")
        );
    }
}

/// `wfl --help` no longer advertises package management.
#[test]
fn help_has_no_package_management_section() {
    let dir = TempDir::new().expect("tempdir");
    let (stdout, stderr, code) = run_in(dir.path(), &["--help"]);
    let combined = format!("{stdout}{stderr}");

    assert_eq!(code, Some(0), "--help should exit 0; got:\n{combined}");
    assert!(
        stdout.contains("USAGE:"),
        "--help should still print the usage banner; got:\n{combined}"
    );
    for forbidden in ["PACKAGE MANAGEMENT", "Publish to the registry", "wflhub"] {
        assert!(
            !stdout.contains(forbidden),
            "--help still mentions {forbidden:?}; got:\n{combined}"
        );
    }
}

/// The `wfl run <file>` and `wfl test <file>` positional aliases lived inside
/// the package-subcommand dispatch block and went with it. Only `wfl <file>`
/// and `wfl --test <file>` remain.
#[test]
fn run_and_test_positional_aliases_are_removed() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("main.wfl"), "display \"hello from wfl\"\n").expect("write main.wfl");

    for alias in ["run", "test"] {
        let (stdout, stderr, code) = run_in(dir.path(), &[alias, "main.wfl"]);
        let combined = format!("{stdout}{stderr}");
        assert_ne!(
            code,
            Some(0),
            "`wfl {alias} main.wfl` should no longer be accepted; got:\n{combined}"
        );
        assert!(
            !stdout.contains("hello from wfl"),
            "`wfl {alias} main.wfl` must not execute the program; got:\n{combined}"
        );
    }
}

/// Regression guard: the documented spellings still work, and the file-based
/// module system that `package:` was bolted onto is untouched.
#[test]
fn relative_module_paths_still_work() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("helper.wfl"),
        "define action called shout with parameters word:\n    \
         display \"helper says \" with word\nend action\n",
    )
    .expect("write helper.wfl");
    fs::write(
        dir.path().join("main.wfl"),
        "include from \"helper.wfl\"\ncall shout with \"hi\"\n",
    )
    .expect("write main.wfl");

    let (stdout, stderr, code) = run_script(dir.path(), "main.wfl");
    let combined = format!("{stdout}{stderr}");
    assert_eq!(
        code,
        Some(0),
        "relative include should run; got:\n{combined}"
    );
    assert!(
        stdout.contains("helper says hi"),
        "relative include should execute the helper; got:\n{combined}"
    );
}

/// Regression guard: `load module from` with a relative path still executes.
#[test]
fn relative_load_module_still_works() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("side_effect.wfl"),
        "display \"module ran\"\n",
    )
    .expect("write side_effect.wfl");
    fs::write(
        dir.path().join("main.wfl"),
        "load module from \"side_effect.wfl\"\n",
    )
    .expect("write main.wfl");

    let (stdout, stderr, code) = run_script(dir.path(), "main.wfl");
    let combined = format!("{stdout}{stderr}");
    assert_eq!(
        code,
        Some(0),
        "relative module load should run; got:\n{combined}"
    );
    assert!(
        stdout.contains("module ran"),
        "relative module load should execute the module; got:\n{combined}"
    );
}
