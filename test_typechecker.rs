use wfl::typechecker::TypeChecker;
use wfl::parser::ast::*;

fn main() {
    let mut tc = TypeChecker::new();
    let program = Program {
        statements: vec![Statement::ActionDefinition {
            name: "test".to_string(),
            parameters: vec![],
            body: vec![
                Statement::VariableDeclaration {
                    name: "local_var".to_string(),
                    value: Expression::Literal(Literal::Integer(10), 1, 1),
                    is_constant: false,
                    line: 1,
                    column: 1,
                },
                Statement::DisplayStatement {
                    value: Expression::Variable("local_var".to_string(), 2, 1),
                    line: 2,
                    column: 1,
                }
            ],
            return_type: None,
            line: 1,
            column: 1,
        }],
    };
    let res = tc.check_types(&program);
    println!("{:?}", res);
}
