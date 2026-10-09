use std::fs;
use wfl::fixer::{CodeFixer, FixerOutputMode};

#[test]
/// Lint and fix must agree on non-ASCII whitespace outside string literals.
fn fixer_removes_the_same_unicode_comment_whitespace_that_lint_reports() {
    let source = "// trailing comment\u{a0}\ndisplay 1\n";
    let tokens = wfl::lexer::lex_wfl_with_positions(source);
    let program = wfl::parser::Parser::new(&tokens).parse().unwrap();
    assert!(
        wfl::linter::Linter::new()
            .lint(&program, source, "source.wfl")
            .0
            .iter()
            .any(|diagnostic| diagnostic.code == "LINT-WHITESPACE")
    );
    let (fixed, _) = CodeFixer::new().fix_checked(&program, source).unwrap();
    assert_eq!(fixed, "// trailing comment\ndisplay 1\n");
    assert!(
        wfl::linter::Linter::new()
            .lint(&program, &fixed, "source.wfl")
            .1
    );
}

#[test]
/// Check patch headers, hunk extents, and unchanged context around an edit.
fn diff_is_a_complete_unified_patch_with_context() {
    let original = "store value as 1\ndisplay value   \ndisplay \"done\"\n";
    let fixed = "store value as 1\ndisplay value\ndisplay \"done\"\n";
    let diff = CodeFixer::new().diff(original, fixed);
    assert!(
        diff.starts_with("--- a/source.wfl\n+++ b/source.wfl\n"),
        "{diff}"
    );
    assert!(diff.contains("@@ -1,3 +1,3 @@\n"), "{diff}");
    assert!(diff.contains(" store value as 1\n"), "{diff}");
    assert!(
        diff.contains("-display value   \n+display value\n"),
        "{diff}"
    );
    assert!(diff.contains(" display \"done\"\n"), "{diff}");
}

#[test]
/// Distinguish missing final newlines and zero-length sides of a patch.
fn diff_reports_final_newline_changes_and_empty_files() {
    let fixer = CodeFixer::new();
    assert_eq!(fixer.diff("display 1\n", "display 1\n"), "");
    let diff = fixer.diff("display 1", "display 1\n");
    assert!(
        diff.contains("-display 1\n\\ No newline at end of file\n+display 1\n"),
        "{diff}"
    );
    let diff = fixer.diff("", "display 1\n");
    assert!(diff.contains("@@ -0,0 +1,1 @@\n+display 1\n"), "{diff}");
    let diff = fixer.diff("display 1\n", "");
    assert!(diff.contains("@@ -1,1 +0,0 @@\n-display 1\n"), "{diff}");
}

#[test]
/// A recovering lexer must not allow invalid bytes to disappear during a write.
fn inplace_rejects_lexical_errors_without_changing_source() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("invalid.wfl");
    let original = "display 1 $\n";
    fs::write(&path, original).unwrap();
    let result = CodeFixer::new().fix_file(&path, FixerOutputMode::InPlace);
    assert!(result.is_err(), "invalid source must not be rewritten");
    assert_eq!(fs::read_to_string(path).unwrap(), original);
}

#[test]
/// Incomplete syntax must fail without replacing the user's original bytes.
fn inplace_rejects_parse_errors_without_changing_source() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("invalid.wfl");
    let original = "store value as\n";
    fs::write(&path, original).unwrap();
    let result = CodeFixer::new().fix_file(&path, FixerOutputMode::InPlace);
    assert!(result.is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), original);
}

#[test]
/// Refuse a read-only destination even when its directory permits replacement.
fn inplace_retains_readonly_source_on_write_failure() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("readonly.wfl");
    let original = "display 1   \n";
    fs::write(&path, original).unwrap();
    let original_permissions = fs::metadata(&path).unwrap().permissions();
    let mut readonly = original_permissions.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly).unwrap();
    let result = CodeFixer::new().fix_file(&path, FixerOutputMode::InPlace);
    let contents = fs::read_to_string(&path).unwrap();
    fs::set_permissions(&path, original_permissions).unwrap();
    assert!(result.is_err(), "read-only files must not be replaced");
    assert_eq!(contents, original);
}
