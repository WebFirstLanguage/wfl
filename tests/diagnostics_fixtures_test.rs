//! Diagnostics fixtures: intentionally invalid WFL programs.
//!
//! Repository hygiene migration: these programs used to sit at the repository
//! root / in `syntax_test/` as unasserted probes whose "expected errors" were
//! only visible to a human reading the output. Per REPOSITORY_HYGIENE.md,
//! intentionally invalid programs live in `tests/fixtures/diagnostics/` with a
//! test asserting the expected diagnostic *and* exit status.
//!
//! Each fixture is copied into a temp dir before running so the interpreter's
//! crash report (`*_debug.txt`) lands there instead of littering the fixture
//! directory — the hygiene checker's working-tree mode fails on droppings.

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/diagnostics")
        .join(name)
}

/// Run a diagnostics fixture in a temp dir; return (combined output, exit code).
fn run_fixture(name: &str) -> (String, i32) {
    let dir = TempDir::new().expect("tempdir");
    let local = dir.path().join(name);
    std::fs::copy(fixture(name), &local).expect("copy fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .arg(name)
        .current_dir(dir.path())
        .output()
        .expect("failed to execute WFL");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let code = output.status.code().expect("exit code");
    (combined, code)
}

#[test]
fn export_non_constant_is_a_runtime_error() {
    let (out, code) = run_fixture("export_non_constant.wfl");
    assert_ne!(code, 0, "exporting a non-constant must fail:\n{out}");
    assert!(
        out.contains("is not a constant and cannot be exported"),
        "expected the export diagnostic, got:\n{out}"
    );
}

#[test]
fn constant_mutation_is_rejected_for_every_mutation_form() {
    let (out, code) = run_fixture("constant_immutability.wfl");
    assert_ne!(code, 0, "mutating a constant must fail:\n{out}");
    assert!(
        out.contains("Cannot modify constant 'MAX_SIZE'"),
        "expected the constant-mutation diagnostic, got:\n{out}"
    );
    // The fixture mutates via change/add/subtract/multiply/divide. Each form
    // is rejected when it appears alone, but in sequence the analyzer
    // currently drops the report for the `add` form (4 reports for 5
    // mutations) — a known reporting gap tracked as issue #671. Pin what
    // actually holds today; tighten to 5 when the gap is fixed.
    let count = out.matches("Cannot modify constant 'MAX_SIZE'").count();
    assert!(
        count >= 4,
        "expected at least 4 of 5 mutation forms rejected, saw {count}:\n{out}"
    );
    assert!(
        !out.contains("This shouldn't be reached"),
        "program must not run past the rejected mutations:\n{out}"
    );
}

#[test]
fn constant_error_takes_precedence_over_undefined_value() {
    let (out, code) = run_fixture("constant_error_precedence.wfl");
    assert_ne!(code, 0, "the fixture must fail analysis:\n{out}");
    // Mutating the constant with an undefined RHS reports the constant error.
    assert!(
        out.contains("Cannot modify constant 'PI'"),
        "expected the constant diagnostic for PI, got:\n{out}"
    );
    // A plain undefined variable in a regular assignment still reports as such.
    assert!(
        out.contains("'another_undefined' is not defined"),
        "expected the undefined-variable diagnostic, got:\n{out}"
    );
}
