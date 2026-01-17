use super::*;
use crate::lexer::lex_wfl_with_positions;
use crate::parser::Parser;

#[test]
fn test_fix_variable_naming() {
    let input = "store Counter as 5";
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let fixer = CodeFixer::new();
    let (fixed_code, summary) = fixer.fix(&program, input);

    assert_eq!(fixed_code.trim(), "store counter as 5");
    assert_eq!(summary.vars_renamed, 1);
}

#[test]
fn test_fix_indentation() {
    let input = "define action called my_test:\ndisplay \"Hello\"\nend action";
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let fixer = CodeFixer::new();
    let (_fixed_code, summary) = fixer.fix(&program, input);

    assert!(summary.lines_reformatted > 0);
}

#[test]
fn test_idempotence() {
    let input = "store counter as 5";
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let fixer = CodeFixer::new();
    let (fixed_code, _) = fixer.fix(&program, input);

    let tokens2 = lex_wfl_with_positions(&fixed_code);
    let program2 = Parser::new(&tokens2).parse().unwrap();
    let (fixed_code2, summary2) = fixer.fix(&program2, &fixed_code);

    assert_eq!(fixed_code.trim(), fixed_code2.trim());
    assert_eq!(summary2.vars_renamed, 0);
}

#[test]
fn test_concatenation_simple_no_fix() {
    // Simple concatenations should not be reformatted
    let input = r#"store message as "Hello" with " World""#;
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let fixer = CodeFixer::new();
    let (fixed_code, summary) = fixer.fix(&program, input);

    assert_eq!(
        fixed_code.trim(),
        r#"store message as "Hello" with " World""#
    );
    assert_eq!(summary.concatenations_fixed, 0);
}

#[test]
fn test_concatenation_problematic_multiline() {
    // Concatenations with many newlines should be reformatted
    let input = "store section as \"---\" with \"\\n\" with \"\\n\" with \"## File \" with file_number with \": \" with file_path with \"\\n\" with \"\\n\" with \"more\" with \"\\n\" with \"stuff\"";
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let fixer = CodeFixer::new();
    let (fixed_code, summary) = fixer.fix(&program, input);

    // Should have reformatted the concatenation
    assert_eq!(summary.concatenations_fixed, 1);
    // Should be formatted as multiline
    assert!(fixed_code.contains("with\n"));
}

#[test]
fn test_concatenation_count_newline_literals() {
    let fixer = CodeFixer::new();

    // Test using a actual newline character which is what WFL parses "\\n" as
    let tokens = lex_wfl_with_positions("store x as \"\n\"");
    let program = Parser::new(&tokens).parse().unwrap();
    if let Some(Statement::VariableDeclaration { value, .. }) = program.statements.first() {
        assert_eq!(fixer.count_newline_literals(value), 1);
    }

    // Test with simple string (no newlines)
    let tokens = lex_wfl_with_positions(r#"store x as "hello""#);
    let program = Parser::new(&tokens).parse().unwrap();
    if let Some(Statement::VariableDeclaration { value, .. }) = program.statements.first() {
        assert_eq!(fixer.count_newline_literals(value), 0);
    }
}

#[test]
fn test_concatenation_chain_length() {
    let fixer = CodeFixer::new();

    // Test simple concatenation (chain length = 1)
    let tokens = lex_wfl_with_positions(r#""a" with "b""#);
    let program = Parser::new(&tokens).parse().unwrap();
    if let Some(Statement::ExpressionStatement { expression, .. }) = program.statements.first() {
        assert_eq!(fixer.count_concatenation_chain(expression), 1);
    }

    // Test longer concatenation chain (chain length = 2)
    let tokens = lex_wfl_with_positions(r#""a" with "b" with "c""#);
    let program = Parser::new(&tokens).parse().unwrap();
    if let Some(Statement::ExpressionStatement { expression, .. }) = program.statements.first() {
        assert_eq!(fixer.count_concatenation_chain(expression), 2);
    }
}
