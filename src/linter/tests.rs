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

/// Regression for #707 review: `yes`/`no`/`true`/`false` lex case-insensitively
/// (`src/lexer/token.rs:446`), so `YES` becomes a `BooleanLiteral` rather than an
/// `Identifier`. The pre-#707 rule carried `yes`/`no` in its keyword array and
/// warned on them, so skipping non-identifier tokens would silently drop that
/// coverage.
#[test]
fn test_keyword_casing_flags_mis_cased_boolean_literals() {
    for (source, expected) in [
        ("store flag as YES", "YES"),
        ("store flag as No", "No"),
        ("store flag as TRUE", "TRUE"),
        ("store flag as False", "False"),
    ] {
        let diagnostics = keyword_casing_diagnostics(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "`{source}` should report exactly one mis-cased literal, got {diagnostics:?}"
        );
        assert_eq!(
            diagnostics[0].message,
            format!("Keyword '{expected}' should be lowercase")
        );
    }
}

/// Correctly-cased boolean literals must stay silent.
#[test]
fn test_keyword_casing_accepts_lowercase_boolean_literals() {
    let diagnostics = keyword_casing_diagnostics("store flag as yes\nstore other as false");
    assert!(
        diagnostics.is_empty(),
        "lowercase boolean literals must not be linted, got {diagnostics:?}"
    );
}

fn lint_source(linter: &Linter, source: &str) -> Vec<WflDiagnostic> {
    let tokens = lex_wfl_with_positions(source);
    let program = Parser::new(&tokens)
        .parse()
        .unwrap_or_else(|errors| panic!("invalid regression fixture: {errors:?}"));
    linter.lint(&program, source, "test.wfl").0
}

#[test]
fn test_lint_max_line_length_setting_is_applied() {
    let mut linter = Linter::new();
    linter.set_max_line_length(12);
    let diagnostics = lint_source(&linter, "display \"hello\"\n");
    assert!(diagnostics.iter().any(|d| d.code == "LINT-LENGTH"));
    linter.set_max_line_length(120);
    assert!(!lint_source(&linter, &format!("// {}\n", "x".repeat(105)))
        .iter()
        .any(|d| d.code == "LINT-LENGTH"));
}

#[test]
fn test_lint_line_length_counts_unicode_characters() {
    let source = format!("display \"{}\"\n", "é".repeat(50));
    assert!(!lint_source(&Linter::new(), &source)
        .iter()
        .any(|d| d.code == "LINT-LENGTH"));
}

#[test]
fn test_lint_local_config_applies_indentation_width() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join(".wflcfg"), "indent_size = 2\n").unwrap();
    let mut linter = Linter::new();
    linter.load_config(directory.path());
    let diagnostics = lint_source(&linter, "check if yes:\n  display \"ok\"\nend check\n");
    assert!(!diagnostics.iter().any(|d| d.code == "LINT-INDENT"), "{diagnostics:?}");
}

#[test]
fn test_lint_indentation_handles_inline_comments_and_try_branches() {
    let source = "try: // attempt\n    display \"ok\"\ncatch: // recovery\n    display \"error\"\nfinally:\n    display \"done\"\nend try\n";
    let diagnostics = lint_source(&Linter::new(), source);
    assert!(!diagnostics.iter().any(|d| d.code == "LINT-INDENT"), "{diagnostics:?}");
}

#[test]
fn test_lint_indentation_handles_route_arms_and_container_bare_end() {
    let source = "create container Example:\n    action greet: Text\n        return \"hello\"\n    end\nend\nroute 1:\n    when 1:\n        display \"one\"\n    otherwise:\n        display \"other\"\nend route\n";
    let diagnostics = lint_source(&Linter::new(), source);
    assert!(!diagnostics.iter().any(|d| d.code == "LINT-INDENT"), "{diagnostics:?}");
}

#[test]
fn test_lint_does_not_treat_multiline_string_contents_as_layout() {
    let source = "store poem as \"first\nend check:   \n  last\"\ndisplay poem\n";
    let diagnostics = lint_source(&Linter::new(), source);
    assert!(!diagnostics.iter().any(|d| matches!(d.code.as_str(), "LINT-INDENT" | "LINT-WHITESPACE")), "{diagnostics:?}");
}

#[test]
fn test_lint_names_inside_nested_statements() {
    let source = "define action called greet:\n    repeat while no:\n        store BadName as 1\n    end repeat\nend action\n";
    let diagnostics = lint_source(&Linter::new(), source);
    assert!(diagnostics.iter().any(|d| d.code == "LINT-NAME" && d.message.contains("BadName")), "{diagnostics:?}");
}

#[test]
fn test_lint_nesting_setting_covers_repeat_and_test_blocks() {
    let source = "describe \"suite\":\n    test \"nested\":\n        repeat while no:\n            repeat while no:\n                display \"deep\"\n            end repeat\n        end repeat\n    end test\nend describe\n";
    let mut linter = Linter::new();
    linter.set_max_nesting_depth(1);
    let diagnostics = lint_source(&linter, source);
    assert!(diagnostics.iter().any(|d| d.code == "LINT-COMPLEX"), "{diagnostics:?}");
    linter.set_max_nesting_depth(10);
    assert!(!lint_source(&linter, source).iter().any(|d| d.code == "LINT-COMPLEX"));
}

#[test]
fn test_lint_indentation_handles_colonless_loops_and_list_blocks() {
    let source = "repeat while no\n    display \"loop\"\nend repeat\ncreate list items:\n    \"one\",\n    \"two\"\nend list\ncheck if yes\n    display \"ok\"\nend check\n";
    let diagnostics = lint_source(&Linter::new(), source);
    assert!(!diagnostics.iter().any(|d| d.code == "LINT-INDENT"), "{diagnostics:?}");
}
