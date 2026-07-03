// TDD parser tests for the database statement syntax.
// Written before the parser implementation.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{DatabaseQueryKind, Expression, Statement};

fn parse(code: &str) -> Vec<Statement> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("Failed to parse {code:?}: {e:?}"))
        .statements
}

fn parse_single(code: &str) -> Statement {
    let mut statements = parse(code);
    assert_eq!(statements.len(), 1, "Expected one statement for {code:?}");
    statements.remove(0)
}

#[test]
fn test_open_database_statement() {
    let stmt = parse_single(r#"open database at "sqlite://./app.db" as db"#);
    match stmt {
        Statement::OpenDatabaseStatement { variable_name, .. } => {
            assert_eq!(variable_name, "db");
        }
        other => panic!("Expected OpenDatabaseStatement, got {other:?}"),
    }
}

#[test]
fn test_connect_to_database_statement() {
    let stmt = parse_single(r#"connect to database at "postgres://localhost/mydb" as db"#);
    match stmt {
        Statement::OpenDatabaseStatement { variable_name, .. } => {
            assert_eq!(variable_name, "db");
        }
        other => panic!("Expected OpenDatabaseStatement, got {other:?}"),
    }
}

#[test]
fn test_query_statement_without_parameters() {
    let stmt = parse_single(r#"store users as query db with "SELECT * FROM users""#);
    match stmt {
        Statement::DatabaseQueryStatement {
            variable_name,
            parameters,
            kind,
            ..
        } => {
            assert_eq!(variable_name, "users");
            assert!(parameters.is_none());
            assert_eq!(kind, DatabaseQueryKind::Query);
        }
        other => panic!("Expected DatabaseQueryStatement, got {other:?}"),
    }
}

#[test]
fn test_query_statement_with_parameters() {
    let stmt = parse_single(
        r#"store users as query db with "SELECT * FROM users WHERE age > ?" and parameters [21]"#,
    );
    match stmt {
        Statement::DatabaseQueryStatement {
            variable_name,
            parameters,
            kind,
            ..
        } => {
            assert_eq!(variable_name, "users");
            assert!(parameters.is_some());
            assert_eq!(kind, DatabaseQueryKind::Query);
        }
        other => panic!("Expected DatabaseQueryStatement, got {other:?}"),
    }
}

#[test]
fn test_execute_statement_without_parameters() {
    let stmt = parse_single(r#"store result as execute db with "DELETE FROM users""#);
    match stmt {
        Statement::DatabaseQueryStatement {
            variable_name,
            parameters,
            kind,
            ..
        } => {
            assert_eq!(variable_name, "result");
            assert!(parameters.is_none());
            assert_eq!(kind, DatabaseQueryKind::Execute);
        }
        other => panic!("Expected DatabaseQueryStatement, got {other:?}"),
    }
}

#[test]
fn test_execute_statement_with_parameters() {
    let stmt = parse_single(
        r#"store result as execute db with "INSERT INTO users (name) VALUES (?)" and parameters [user_name]"#,
    );
    match stmt {
        Statement::DatabaseQueryStatement {
            parameters, kind, ..
        } => {
            assert!(parameters.is_some());
            assert_eq!(kind, DatabaseQueryKind::Execute);
        }
        other => panic!("Expected DatabaseQueryStatement, got {other:?}"),
    }
}

#[test]
fn test_close_database_statement() {
    let stmt = parse_single("close database db");
    assert!(
        matches!(stmt, Statement::CloseDatabaseStatement { .. }),
        "Expected CloseDatabaseStatement, got {stmt:?}"
    );
}

#[test]
fn test_query_inside_wait_for() {
    let stmt = parse_single(r#"wait for store users as query db with "SELECT * FROM users""#);
    match stmt {
        Statement::WaitForStatement { inner, .. } => {
            assert!(
                matches!(*inner, Statement::DatabaseQueryStatement { .. }),
                "Expected DatabaseQueryStatement inside wait for, got {inner:?}"
            );
        }
        other => panic!("Expected WaitForStatement, got {other:?}"),
    }
}

#[test]
fn test_open_database_inside_wait_for() {
    let stmt = parse_single(r#"wait for open database at "sqlite://./app.db" as db"#);
    match stmt {
        Statement::WaitForStatement { inner, .. } => {
            assert!(matches!(*inner, Statement::OpenDatabaseStatement { .. }));
        }
        other => panic!("Expected WaitForStatement, got {other:?}"),
    }
}

// === Return-position database queries (issue #559) ===

/// Extract the return value expression from a one-statement action body.
fn return_value_of_action(code: &str) -> Expression {
    let stmt = parse_single(code);
    let Statement::ActionDefinition { body, .. } = stmt else {
        panic!("Expected ActionDefinition, got {stmt:?}");
    };
    assert_eq!(body.len(), 1, "Expected one statement in action body");
    let Statement::ReturnStatement {
        value: Some(value), ..
    } = &body[0]
    else {
        panic!("Expected ReturnStatement with a value, got {:?}", body[0]);
    };
    value.clone()
}

#[test]
fn test_return_query_with_parameters() {
    let value = return_value_of_action(
        r#"define action called get_n with parameters conn and id:
    return query conn with "SELECT n FROM t WHERE id = ?" and parameters [id]
end action"#,
    );
    match value {
        Expression::DatabaseQuery {
            parameters, kind, ..
        } => {
            assert!(parameters.is_some());
            assert_eq!(kind, DatabaseQueryKind::Query);
        }
        other => panic!("Expected DatabaseQuery expression, got {other:?}"),
    }
}

#[test]
fn test_return_query_without_parameters() {
    let value = return_value_of_action(
        r#"define action called get_all with parameters conn:
    return query conn with "SELECT n FROM t"
end action"#,
    );
    match value {
        Expression::DatabaseQuery {
            parameters, kind, ..
        } => {
            assert!(parameters.is_none());
            assert_eq!(kind, DatabaseQueryKind::Query);
        }
        other => panic!("Expected DatabaseQuery expression, got {other:?}"),
    }
}

#[test]
fn test_return_execute_with_parameters() {
    let value = return_value_of_action(
        r#"define action called add_row with parameters conn and id:
    return execute conn with "INSERT INTO t (id) VALUES (?)" and parameters [id]
end action"#,
    );
    match value {
        Expression::DatabaseQuery {
            parameters, kind, ..
        } => {
            assert!(parameters.is_some());
            assert_eq!(kind, DatabaseQueryKind::Execute);
        }
        other => panic!("Expected DatabaseQuery expression, got {other:?}"),
    }
}

#[test]
fn test_return_execute_without_parameters() {
    let value = return_value_of_action(
        r#"define action called clear_rows with parameters conn:
    return execute conn with "DELETE FROM t"
end action"#,
    );
    match value {
        Expression::DatabaseQuery {
            parameters, kind, ..
        } => {
            assert!(parameters.is_none());
            assert_eq!(kind, DatabaseQueryKind::Execute);
        }
        other => panic!("Expected DatabaseQuery expression, got {other:?}"),
    }
}

#[test]
fn test_give_back_query_with_parameters() {
    let value = return_value_of_action(
        r#"define action called get_n with parameters conn and id:
    give back query conn with "SELECT n FROM t WHERE id = ?" and parameters [id]
end action"#,
    );
    assert!(
        matches!(value, Expression::DatabaseQuery { .. }),
        "Expected DatabaseQuery expression, got {value:?}"
    );
}

#[test]
fn test_return_variable_named_query_still_works() {
    // A plain variable named `query` (no handle) after return must keep
    // parsing as an ordinary expression.
    let value = return_value_of_action(
        r#"define action called passthrough:
    return query
end action"#,
    );
    assert!(
        matches!(&value, Expression::Variable(name, ..) if name == "query"),
        "Expected Variable(\"query\"), got {value:?}"
    );
}

#[test]
fn test_return_query_concatenation_still_works() {
    // `return query with "..."` has no db handle, so it must stay a
    // concatenation of the variable `query`, not a database query.
    let value = return_value_of_action(
        r#"define action called label with parameters query:
    return query with " suffix"
end action"#,
    );
    assert!(
        !matches!(value, Expression::DatabaseQuery { .. }),
        "Expected non-database expression, got {value:?}"
    );
}

// === Backward compatibility characterization ===

#[test]
fn test_variable_named_query_still_works() {
    // `query` is a plain identifier; storing and copying it must keep parsing
    // as ordinary variable use.
    let statements = parse(
        r#"
store query as "SELECT 1"
store copy as query
display copy
"#,
    );
    assert_eq!(statements.len(), 3);
    assert!(matches!(
        &statements[1],
        Statement::VariableDeclaration { name, .. } if name == "copy"
    ));
}

#[test]
fn test_variable_named_query_in_concatenation() {
    // `store x as query with "..."` would be ambiguous with the DB form only if
    // the db-handle position is missing; concatenation directly after `query`
    // must keep working.
    let stmt = parse_single(r#"store message as query with " suffix""#);
    assert!(
        matches!(&stmt, Statement::VariableDeclaration { name, .. } if name == "message"),
        "Expected VariableDeclaration, got {stmt:?}"
    );
}

#[test]
fn test_open_file_statement_unchanged() {
    let stmt = parse_single(r#"open file at "data.txt" for reading as f"#);
    assert!(matches!(stmt, Statement::OpenFileStatement { .. }));
}

#[test]
fn test_close_file_statement_unchanged() {
    let stmt = parse_single("close file f");
    assert!(matches!(stmt, Statement::CloseFileStatement { .. }));
}

#[test]
fn test_execute_command_statement_unchanged() {
    let stmt = parse_single(r#"wait for execute command "echo hi" as cmd_result"#);
    fn contains_database_statement(stmt: &Statement) -> bool {
        match stmt {
            Statement::DatabaseQueryStatement { .. } => true,
            Statement::WaitForStatement { inner, .. } => contains_database_statement(inner),
            _ => false,
        }
    }
    assert!(
        !contains_database_statement(&stmt),
        "execute command must not become a database statement: {stmt:?}"
    );
}
