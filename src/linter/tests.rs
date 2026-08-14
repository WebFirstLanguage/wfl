use super::*;
use crate::lexer::lex_wfl_with_positions;
use crate::parser::Parser;

#[test]
fn test_naming_convention_rule() {
    let input = "store Counter as 5";
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let rule = NamingConventionRule;
    let mut reporter = DiagnosticReporter::new();
    let file_id = reporter.add_file("test.wfl", input);

    let diagnostics = rule.apply(&program, &mut reporter, file_id);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("Counter"));
    assert_eq!(diagnostics[0].code, "LINT-NAME");
}

#[test]
fn test_snake_case_conversion() {
    assert_eq!(to_snake_case("camelCase"), "camel_case");
    assert_eq!(to_snake_case("PascalCase"), "pascal_case");
    assert_eq!(to_snake_case("snake_case"), "snake_case");
    assert_eq!(to_snake_case("with space"), "with_space");
    assert_eq!(to_snake_case("Mixed_Style"), "mixed_style");
}

#[test]
fn test_is_snake_case() {
    assert!(is_snake_case("snake_case"));
    assert!(is_snake_case("simple"));
    assert!(!is_snake_case("camelCase"));
    assert!(!is_snake_case("PascalCase"));
    assert!(!is_snake_case("with space"));
    assert!(!is_snake_case("Mixed_Style"));
}

/// Apply only `KeywordCasingRule` to `input` and return its diagnostics.
///
/// The rule does not consult the AST, so an input that intentionally fails to
/// parse (e.g. `STORE counter as 5`, where `STORE` lexes as an identifier)
/// falls back to an empty program rather than panicking.
fn keyword_casing_diagnostics(input: &str) -> Vec<WflDiagnostic> {
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap_or_default();

    let rule = KeywordCasingRule;
    let mut reporter = DiagnosticReporter::new();
    let file_id = reporter.add_file("test.wfl", input);

    rule.apply(&program, &mut reporter, file_id)
}

/// Regression for #707: keyword casing must not match inside string literals.
#[test]
fn test_keyword_casing_ignores_string_literals() {
    let diagnostics = keyword_casing_diagnostics("store s as \"MNOP\"");
    assert!(
        diagnostics.is_empty(),
        "string literal contents must not be linted, got {diagnostics:?}"
    );
}

/// Regression for #707: keyword casing must not match inside comments.
#[test]
fn test_keyword_casing_ignores_comments() {
    let diagnostics = keyword_casing_diagnostics("// Note: this explains the next step");
    assert!(
        diagnostics.is_empty(),
        "comment text must not be linted, got {diagnostics:?}"
    );
}

/// Regression for #707: keyword casing must not match inside ordinary words.
#[test]
fn test_keyword_casing_ignores_words_containing_keywords() {
    let diagnostics = keyword_casing_diagnostics("store label as \"Ineligible\"");
    assert!(
        diagnostics.is_empty(),
        "substrings of ordinary words must not be linted, got {diagnostics:?}"
    );
}

/// A genuinely mis-cased keyword must still be reported (backward compatibility).
#[test]
fn test_keyword_casing_flags_uppercase_keyword() {
    let diagnostics = keyword_casing_diagnostics("STORE counter as 5");

    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_eq!(diagnostics[0].code, "LINT-KEYWORD");
    assert_eq!(diagnostics[0].severity, Severity::Warning);
    assert_eq!(
        diagnostics[0].message,
        "Keyword 'STORE' should be lowercase"
    );
    assert_eq!(diagnostics[0].notes, vec!["Change to 'store'".to_string()]);
    assert_eq!(diagnostics[0].line, 1);
    assert_eq!(diagnostics[0].column, 1);
}

/// Regression for #707: every occurrence is reported, not just the first.
#[test]
fn test_keyword_casing_reports_every_occurrence() {
    let input = "STORE alpha as 1\nSTORE beta as 2\nSTORE gamma as 3";
    let diagnostics = keyword_casing_diagnostics(input);

    assert_eq!(diagnostics.len(), 3, "got {diagnostics:?}");
    let lines: Vec<usize> = diagnostics.iter().map(|d| d.line).collect();
    assert_eq!(lines, vec![1, 2, 3]);
    assert!(
        diagnostics
            .iter()
            .all(|d| d.message == "Keyword 'STORE' should be lowercase")
    );
}

/// A correctly written program produces no keyword-casing diagnostics.
#[test]
fn test_keyword_casing_clean_program() {
    let input = "store counter as 5\ndisplay counter\n";
    let diagnostics = keyword_casing_diagnostics(input);
    assert!(
        diagnostics.is_empty(),
        "lowercase program must be clean, got {diagnostics:?}"
    );
}

#[test]
fn test_linter_integration() {
    let input = "store Counter as 5\nstore snakecase as 10";
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let linter = Linter::new();
    let (diagnostics, success) = linter.lint(&program, input, "test.wfl");

    assert!(!success);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code == "LINT-NAME" && d.message.contains("Counter"))
    );
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.code == "LINT-NAME" && d.message.contains("snakecase"))
    );
}
