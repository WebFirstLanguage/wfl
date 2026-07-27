use wfl::fixer::CodeFixer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Program, Statement, Type};

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
        (
            Type::List(Box::new(Type::Optional(Box::new(Type::Number)))),
            "produce: List of Optional of Number:",
        ),
        (Type::Custom("datetime".to_string()), "produce: datetime:"),
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

#[test]
fn returns_inside_a_legacy_multi_word_action_name_is_not_an_annotation() {
    let source = "define action called calculate returns schedule:\nend action";
    let program = Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .expect("legacy multi-word action name must parse");
    let Statement::ActionDefinition {
        name, return_type, ..
    } = &program.statements[0]
    else {
        panic!("expected an action definition");
    };

    assert_eq!(name, "calculate returns schedule");
    assert_eq!(return_type, &None);
}
