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

#[test]
fn temporal_type_identity_survives_fix_and_reparse() {
    let input = "define action called inspect with parameters day as date and clock as time and instant as datetime:\n    display day\nend action";
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let fixer = CodeFixer::new();
    let (fixed_code, _) = fixer.fix(&program, input);
    let reparsed_tokens = lex_wfl_with_positions(&fixed_code);
    let reparsed = Parser::new(&reparsed_tokens).parse().unwrap();

    let parameter_types = |program: &Program| {
        let Statement::ActionDefinition { parameters, .. } = &program.statements[0] else {
            panic!("expected action definition");
        };
        parameters
            .iter()
            .map(|parameter| parameter.param_type.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(parameter_types(&program), parameter_types(&reparsed));
    assert!(fixed_code.contains("day as date"));
    assert!(fixed_code.contains("clock as time"));
    assert!(fixed_code.contains("instant as datetime"));
    assert!(fixed_code.contains("date and clock"));
}

#[test]
fn action_return_type_survives_fix_and_reparse() {
    let input = "define action called current_day: date:\n    return today\nend action";
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let fixer = CodeFixer::new();
    let (fixed_code, _) = fixer.fix(&program, input);
    assert!(
        fixed_code.contains("current_day: date:"),
        "the fixer must emit parser-canonical return syntax: {fixed_code}"
    );

    let reparsed = Parser::new(&lex_wfl_with_positions(&fixed_code))
        .parse()
        .expect("fixed action must reparse");
    let Statement::ActionDefinition {
        name, return_type, ..
    } = &reparsed.statements[0]
    else {
        panic!("expected an action definition after fixing");
    };
    assert_eq!(name, "current_day");
    assert_eq!(return_type, &Some(Type::Date));
}

#[test]
fn compound_action_return_types_survive_fix_and_reparse() {
    let cases = [
        (Type::List(Box::new(Type::Text)), "produce: List of Text:"),
        (
            Type::Map(Box::new(Type::Text), Box::new(Type::Binary)),
            "produce: Map of Text to Binary:",
        ),
        (
            Type::Optional(Box::new(Type::List(Box::new(Type::Number)))),
            "produce: Optional of List of Number:",
        ),
    ];

    for (expected_type, expected_source) in cases {
        let program = Program {
            statements: vec![Statement::ActionDefinition {
                name: "produce".to_string(),
                parameters: vec![],
                body: vec![],
                return_type: Some(expected_type.clone()),
                line: 1,
                column: 1,
            }],
        };

        let (fixed_code, _) = CodeFixer::new().fix(&program, "");
        assert!(
            fixed_code.contains(expected_source),
            "fixer emitted an unexpected compound return type: {fixed_code}"
        );

        let reparsed = Parser::new(&lex_wfl_with_positions(&fixed_code))
            .parse()
            .unwrap_or_else(|error| {
                panic!("fixed compound return type must reparse: {fixed_code}\n{error:?}")
            });
        let Statement::ActionDefinition { return_type, .. } = &reparsed.statements[0] else {
            panic!("expected an action definition after fixing");
        };
        assert_eq!(
            return_type,
            &Some(expected_type),
            "fixed return annotation changed type: {fixed_code}"
        );
    }
}
