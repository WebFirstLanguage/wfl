use super::*;
use crate::lexer::lex_wfl_with_positions;
use crate::parser::Parser;
use crate::utils::string::to_snake_case;

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
