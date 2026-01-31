use wfl::analyzer::Analyzer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

#[test]
fn test_export_mutable_variable_as_constant_fails() {
    // Test that mutable variables cannot be exported as constants
    let code = r#"
store mutable_var as 42
export constant mutable_var
"#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    // Analyze the program
    let mut analyzer = Analyzer::new();
    analyzer.analyze(&program).expect("Should analyze");

    // Type check should fail because mutable_var is mutable
    let mut type_checker = TypeChecker::with_analyzer(analyzer);
    let result = type_checker.check_types(&program);

    assert!(
        result.is_err(),
        "Type checking should fail when exporting mutable variable as constant"
    );

    let errors = result.unwrap_err();
    assert!(
        !errors.is_empty(),
        "Should have type errors for mutable constant export"
    );

    let error_msg = errors[0].to_string();
    assert!(
        error_msg.contains("mutable") && error_msg.contains("cannot be exported as constant"),
        "Error message should indicate mutable variable cannot be exported as constant: {}",
        error_msg
    );
}

#[test]
fn test_export_immutable_variable_as_constant_succeeds() {
    // Test that immutable variables can be exported as constants
    let code = r#"
store new constant VERSION as "1.0.0"
export constant VERSION
"#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    // Analyze the program
    let mut analyzer = Analyzer::new();
    analyzer.analyze(&program).expect("Should analyze");

    // Type check should succeed because VERSION is immutable
    let mut type_checker = TypeChecker::with_analyzer(analyzer);
    let result = type_checker.check_types(&program);

    assert!(
        result.is_ok(),
        "Type checking should succeed when exporting immutable variable as constant: {:?}",
        result.err()
    );
}

#[test]
fn test_export_nonexistent_constant_fails() {
    // Test that exporting a non-existent constant produces the correct error
    let code = r#"
export constant MISSING_CONSTANT
"#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    // Analyze the program
    let mut analyzer = Analyzer::new();
    analyzer.analyze(&program).expect("Should analyze");

    // Type check should fail because the constant doesn't exist
    let mut type_checker = TypeChecker::with_analyzer(analyzer);
    let result = type_checker.check_types(&program);

    assert!(
        result.is_err(),
        "Type checking should fail when exporting non-existent constant"
    );

    let errors = result.unwrap_err();
    assert!(
        !errors.is_empty(),
        "Should have type errors for missing constant"
    );

    let error_msg = errors[0].to_string();
    assert!(
        error_msg.contains("not found for export"),
        "Error message should indicate constant not found: {}",
        error_msg
    );
}
