// TDD tests for the `file_mode` / `set_file_mode` builtins (issue #666).
//
// The issue's motivating case is a config file holding an API key: a program must
// be able to make it 0600 *and* to verify it is 0600 so it can refuse to start
// otherwise. Both halves are tested here.
//
// Unix gets real POSIX semantics. Windows has no equivalent, so `file_mode`
// returns a documented approximation and `set_file_mode` raises an explicit
// unsupported error — never a silent no-op, which is the failure mode the issue
// complains about.

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

async fn run_wfl(code: &str) -> Result<Interpreter, String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().map_err(|e| format!("Parse error: {e:?}"))?;

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&ast)
        .await
        .map_err(|e| format!("Runtime error: {e:?}"))?;
    Ok(interpreter)
}

fn get_global(interpreter: &Interpreter, name: &str) -> Value {
    interpreter
        .global_env()
        .borrow()
        .get(name)
        .unwrap_or_else(|| panic!("Variable '{name}' not found"))
}

fn expect_text(value: &Value) -> String {
    match value {
        Value::Text(t) => t.to_string(),
        other => panic!("Expected text, got {other:?}"),
    }
}

/// Create a temp file with some content and return its WFL-safe path string.
fn temp_file(name: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join(name);
    std::fs::write(&path, "api_key = secret\n").expect("write temp file");
    let path_str = path.display().to_string().replace('\\', "/");
    (dir, path_str)
}

#[tokio::test]
async fn file_mode_reads_a_four_character_octal_string() {
    let (_dir, path) = temp_file("config.json");
    let code = format!(
        r#"
store mode as file_mode of "{path}"
"#
    );
    let interpreter = run_wfl(&code).await.expect("program should run");
    let mode = expect_text(&get_global(&interpreter, "mode"));
    assert_eq!(
        mode.len(),
        4,
        "file_mode should return a 4-character octal string like \"0600\", got {mode:?}"
    );
    assert!(
        mode.chars().all(|c| ('0'..='7').contains(&c)),
        "file_mode should return octal digits only, got {mode:?}"
    );
}

#[tokio::test]
async fn file_mode_on_a_missing_path_errors() {
    let code = r#"
store mode as file_mode of "/definitely/not/a/real/path/config.json"
"#;
    let err = run_wfl(code)
        .await
        .err()
        .expect("reading the mode of a missing file must error");
    assert!(
        err.to_lowercase().contains("exist"),
        "error should say the file does not exist, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// Unix: real semantics
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod unix {
    use super::*;

    #[tokio::test]
    async fn set_then_read_round_trips_to_0600() {
        // The issue's core scenario, end to end.
        let (_dir, path) = temp_file("config.json");
        let code = format!(
            r#"
store applied as set_file_mode of "{path}" and "0600"
store mode as file_mode of "{path}"
"#
        );
        let interpreter = run_wfl(&code).await.expect("program should run");
        assert_eq!(
            expect_text(&get_global(&interpreter, "mode")),
            "0600",
            "a file set to 0600 must read back as 0600"
        );
    }

    #[tokio::test]
    async fn the_mode_is_actually_applied_on_disk() {
        // Assert the real side effect, not just what the builtin reports back.
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("secret.toml");
        std::fs::write(&path, "token = \"x\"\n").expect("write");
        let path_str = path.display().to_string();

        let code = format!(
            r#"
store applied as set_file_mode of "{path_str}" and "0600"
"#
        );
        run_wfl(&code).await.expect("program should run");

        let mode = std::fs::metadata(&path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o7777;
        assert_eq!(
            mode, 0o600,
            "the file on disk must actually be 0600, got {mode:o}"
        );
    }

    #[tokio::test]
    async fn a_group_readable_file_is_detectable() {
        // "Refuse to start if the config is group- or world-readable" — the check
        // the issue says is impossible today.
        let (_dir, path) = temp_file("config.json");
        let code = format!(
            r#"
store applied as set_file_mode of "{path}" and "0644"
store mode as file_mode of "{path}"
store is_locked_down as mode is equal to "0600"
"#
        );
        let interpreter = run_wfl(&code).await.expect("program should run");
        assert_eq!(expect_text(&get_global(&interpreter, "mode")), "0644");
        assert_eq!(
            get_global(&interpreter, "is_locked_down"),
            Value::Bool(false),
            "a 0644 config must be distinguishable from a 0600 one"
        );
    }

    #[tokio::test]
    async fn three_digit_and_four_digit_modes_are_both_accepted() {
        let (_dir, path) = temp_file("config.json");
        let code = format!(
            r#"
store a as set_file_mode of "{path}" and "600"
store after_short as file_mode of "{path}"
store b as set_file_mode of "{path}" and "0640"
store after_long as file_mode of "{path}"
"#
        );
        let interpreter = run_wfl(&code).await.expect("program should run");
        assert_eq!(
            expect_text(&get_global(&interpreter, "after_short")),
            "0600"
        );
        assert_eq!(expect_text(&get_global(&interpreter, "after_long")), "0640");
    }

    #[tokio::test]
    async fn malformed_modes_are_rejected() {
        for bad in ["rw-------", "0999", "abc", "", "0o600", "-1", "10000"] {
            let (_dir, path) = temp_file("config.json");
            let code = format!(
                r#"
store applied as set_file_mode of "{path}" and "{bad}"
"#
            );
            let err = match run_wfl(&code).await {
                Ok(_) => panic!("mode {bad:?} must be rejected, not silently masked"),
                Err(e) => e,
            };
            assert!(
                err.to_lowercase().contains("mode"),
                "error for {bad:?} should explain the expected format, got: {err}"
            );
        }
    }

    #[tokio::test]
    async fn set_file_mode_on_a_missing_path_errors() {
        let code = r#"
store applied as set_file_mode of "/definitely/not/a/real/path/x.json" and "0600"
"#;
        run_wfl(code)
            .await
            .err()
            .expect("setting the mode of a missing file must error");
    }
}

// ---------------------------------------------------------------------------
// Windows: explicit and loud, never a silent no-op
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod windows {
    use super::*;

    #[tokio::test]
    async fn set_file_mode_reports_that_it_is_unsupported() {
        let (_dir, path) = temp_file("config.json");
        let code = format!(
            r#"
store applied as set_file_mode of "{path}" and "0600"
"#
        );
        let err = run_wfl(&code)
            .await
            .err()
            .expect("set_file_mode must not silently pretend to work on Windows");
        let lower = err.to_lowercase();
        assert!(
            lower.contains("not supported") || lower.contains("unsupported"),
            "the error must say the operation is unsupported here, got: {err}"
        );
        assert!(
            lower.contains("windows"),
            "the error must name the platform, got: {err}"
        );
    }

    #[tokio::test]
    async fn file_mode_still_returns_an_approximation() {
        // Reading stays available so a cross-platform program can call it
        // unconditionally; the docs state it is an approximation on Windows.
        let (_dir, path) = temp_file("config.json");
        let code = format!(
            r#"
store mode as file_mode of "{path}"
"#
        );
        let interpreter = run_wfl(&code).await.expect("program should run");
        let mode = expect_text(&get_global(&interpreter, "mode"));
        assert!(
            mode == "0666" || mode == "0444",
            "Windows approximation should be 0666 (writable) or 0444 (read-only), got {mode:?}"
        );
    }
}
