use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, Literal, Program, Statement};
use wfl::typechecker::TypeChecker;

fn parse(source: &str) -> Program {
    Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .expect("test program should parse")
}

fn typechecks(program: &Program) -> bool {
    TypeChecker::new().check_types(program).is_ok()
}

#[test]
fn branch_only_bindings_do_not_escape_multi_line_if() {
    for source in [
        r#"
check if no:
    store branch_only as 1
end check
display branch_only
"#,
        r#"
check if yes:
    display "then"
otherwise:
    store branch_only as 1
end check
display branch_only
"#,
    ] {
        let program = parse(source);
        assert!(
            !typechecks(&program),
            "a name missing from a reachable branch is not definitely bound: {source}"
        );
    }
}

#[test]
fn bindings_created_in_both_multi_line_branches_escape() {
    let program = parse(
        r#"
check if yes:
    store branch_result as 1
otherwise:
    store branch_result as 2
end check
display branch_result
"#,
    );
    assert!(
        typechecks(&program),
        "a name created on every branch is definitely bound"
    );
}

#[test]
fn bindings_created_in_both_single_line_branches_escape() {
    let store = |value| Statement::VariableDeclaration {
        name: "branch_result".to_string(),
        value: Expression::Literal(Literal::Integer(value), 1, 1),
        is_constant: false,
        line: 1,
        column: 1,
    };
    let program = Program {
        statements: vec![
            Statement::SingleLineIf {
                condition: Expression::Literal(Literal::Boolean(true), 1, 1),
                then_stmt: Box::new(store(1)),
                else_stmt: Some(Box::new(store(2))),
                line: 1,
                column: 1,
            },
            Statement::DisplayStatement {
                value: Expression::Variable("branch_result".to_string(), 2, 1),
                line: 2,
                column: 1,
            },
        ],
    };
    assert!(
        typechecks(&program),
        "single-line and multi-line if must use the same definite-binding rule"
    );
}

fn declaration(name: &str, value: i64, is_constant: bool) -> Statement {
    Statement::VariableDeclaration {
        name: name.to_string(),
        value: Expression::Literal(Literal::Integer(value), 1, 1),
        is_constant,
        line: 1,
        column: 1,
    }
}

fn assignment(name: &str) -> Statement {
    Statement::Assignment {
        name: name.to_string(),
        value: Expression::Literal(Literal::Integer(3), 2, 1),
        line: 2,
        column: 1,
    }
}

#[test]
fn mixed_mutability_branches_merge_as_immutable_multi_line() {
    for (then_constant, else_constant) in [(false, true), (true, false)] {
        let program = Program {
            statements: vec![
                Statement::IfStatement {
                    condition: Expression::Literal(Literal::Boolean(true), 1, 1),
                    then_block: vec![declaration("branch_result", 1, then_constant)],
                    else_block: Some(vec![declaration("branch_result", 2, else_constant)]),
                    line: 1,
                    column: 1,
                },
                assignment("branch_result"),
            ],
        };
        assert!(
            !typechecks(&program),
            "a binding is mutable after a join only when every branch creates it mutable"
        );
    }
}

#[test]
fn mixed_mutability_branches_merge_as_immutable_single_line() {
    for (then_constant, else_constant) in [(false, true), (true, false)] {
        let program = Program {
            statements: vec![
                Statement::SingleLineIf {
                    condition: Expression::Literal(Literal::Boolean(true), 1, 1),
                    then_stmt: Box::new(declaration("branch_result", 1, then_constant)),
                    else_stmt: Some(Box::new(declaration("branch_result", 2, else_constant))),
                    line: 1,
                    column: 1,
                },
                assignment("branch_result"),
            ],
        };
        assert!(
            !typechecks(&program),
            "single-line joins must not make a maybe-constant binding mutable"
        );
    }
}
