// TDD tests for the database runtime (sqlx bindings).
//
// SQLite tests run everywhere with no external services. PostgreSQL and
// MariaDB tests are gated on WFL_TEST_POSTGRES_URL / WFL_TEST_MYSQL_URL and
// skip with a notice when unset (same pattern as WFLHASH_HEAVY_TESTS).

use std::path::PathBuf;
use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Run WFL code and return the interpreter for inspecting globals.
async fn run_wfl(code: &str) -> Result<Interpreter, String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let ast = parser
        .parse()
        .map_err(|e| format!("Parse error: {:?}", e))?;

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&ast)
        .await
        .map_err(|e| format!("Runtime error: {:?}", e))?;
    Ok(interpreter)
}

fn get_global(interpreter: &Interpreter, name: &str) -> Value {
    interpreter
        .global_env()
        .borrow()
        .get(name)
        .unwrap_or_else(|| panic!("Variable '{name}' not found"))
}

fn expect_number(value: &Value) -> f64 {
    match value {
        Value::Number(n) => *n,
        other => panic!("Expected number, got {other:?}"),
    }
}

fn expect_text(value: &Value) -> String {
    match value {
        Value::Text(t) => t.to_string(),
        other => panic!("Expected text, got {other:?}"),
    }
}

fn expect_object_key(value: &Value, key: &str) -> Value {
    match value {
        Value::Object(obj) => obj
            .borrow()
            .get(key)
            .cloned()
            .unwrap_or_else(|| panic!("Object missing key '{key}'")),
        other => panic!("Expected object, got {other:?}"),
    }
}

fn expect_list(value: &Value) -> Vec<Value> {
    match value {
        Value::List(list) => list.borrow().clone(),
        other => panic!("Expected list, got {other:?}"),
    }
}

/// Unique temp-file SQLite URL per test (forward slashes for WFL strings).
fn sqlite_url(test_name: &str) -> (String, PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "wfl_db_test_{}_{}.db",
        test_name,
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let url = format!("sqlite://{}", path.display()).replace('\\', "/");
    (url, path)
}

mod sqlite_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_insert_select_roundtrip() {
        let (url, path) = sqlite_url("roundtrip");
        let code = format!(
            r#"
open database at "{url}" as db
store created as execute db with "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)"
store inserted as execute db with "INSERT INTO users (name, age) VALUES (?, ?)" and parameters ["Alice" and 30]
store rows as query db with "SELECT id, name, age FROM users"
close database db
"#
        );
        let interpreter = run_wfl(&code).await.expect("program should run");

        let inserted = get_global(&interpreter, "inserted");
        assert_eq!(
            expect_number(&expect_object_key(&inserted, "affected_rows")),
            1.0
        );
        assert_eq!(
            expect_number(&expect_object_key(&inserted, "last_insert_id")),
            1.0
        );

        let rows = expect_list(&get_global(&interpreter, "rows"));
        assert_eq!(rows.len(), 1);
        assert_eq!(expect_text(&expect_object_key(&rows[0], "name")), "Alice");
        assert_eq!(expect_number(&expect_object_key(&rows[0], "age")), 30.0);
        assert_eq!(expect_number(&expect_object_key(&rows[0], "id")), 1.0);

        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn test_in_memory_database() {
        let code = r#"
open database at "sqlite::memory:" as db
store created as execute db with "CREATE TABLE t (x INTEGER)"
store inserted as execute db with "INSERT INTO t (x) VALUES (?)" and parameters [42]
store rows as query db with "SELECT x FROM t"
close database db
"#;
        let interpreter = run_wfl(code).await.expect("program should run");
        let rows = expect_list(&get_global(&interpreter, "rows"));
        assert_eq!(rows.len(), 1);
        assert_eq!(expect_number(&expect_object_key(&rows[0], "x")), 42.0);
    }

    #[tokio::test]
    async fn test_null_becomes_nothing() {
        let code = r#"
open database at "sqlite::memory:" as db
store created as execute db with "CREATE TABLE t (x INTEGER, y TEXT)"
store inserted as execute db with "INSERT INTO t (x, y) VALUES (1, NULL)"
store rows as query db with "SELECT x, y FROM t"
close database db
"#;
        let interpreter = run_wfl(code).await.expect("program should run");
        let rows = expect_list(&get_global(&interpreter, "rows"));
        let y = expect_object_key(&rows[0], "y");
        assert!(
            matches!(y, Value::Null),
            "SQL NULL should map to nothing, got {y:?}"
        );
    }

    #[tokio::test]
    async fn test_bound_parameters_resist_injection() {
        let code = r#"
open database at "sqlite::memory:" as db
store created as execute db with "CREATE TABLE users (name TEXT)"
store evil as "x'; DROP TABLE users;--"
store inserted as execute db with "INSERT INTO users (name) VALUES (?)" and parameters [evil]
store rows as query db with "SELECT name FROM users"
store still_there as query db with "SELECT count(*) AS n FROM users"
close database db
"#;
        let interpreter = run_wfl(code).await.expect("program should run");
        let rows = expect_list(&get_global(&interpreter, "rows"));
        assert_eq!(
            expect_text(&expect_object_key(&rows[0], "name")),
            "x'; DROP TABLE users;--",
            "Bound parameter must be stored literally"
        );
        let still = expect_list(&get_global(&interpreter, "still_there"));
        assert_eq!(expect_number(&expect_object_key(&still[0], "n")), 1.0);
    }

    #[tokio::test]
    async fn test_update_affected_rows() {
        let code = r#"
open database at "sqlite::memory:" as db
store created as execute db with "CREATE TABLE t (x INTEGER)"
store i1 as execute db with "INSERT INTO t (x) VALUES (1)"
store i2 as execute db with "INSERT INTO t (x) VALUES (2)"
store updated as execute db with "UPDATE t SET x = x + 10"
close database db
"#;
        let interpreter = run_wfl(code).await.expect("program should run");
        let updated = get_global(&interpreter, "updated");
        assert_eq!(
            expect_number(&expect_object_key(&updated, "affected_rows")),
            2.0
        );
    }

    #[tokio::test]
    async fn test_real_and_bool_types() {
        let code = r#"
open database at "sqlite::memory:" as db
store created as execute db with "CREATE TABLE t (price REAL, active BOOLEAN)"
store inserted as execute db with "INSERT INTO t (price, active) VALUES (?, ?)" and parameters [9.5 and yes]
store rows as query db with "SELECT price, active FROM t"
close database db
"#;
        let interpreter = run_wfl(code).await.expect("program should run");
        let rows = expect_list(&get_global(&interpreter, "rows"));
        assert_eq!(expect_number(&expect_object_key(&rows[0], "price")), 9.5);
        let active = expect_object_key(&rows[0], "active");
        assert!(
            matches!(active, Value::Bool(true) | Value::Number(_)),
            "BOOLEAN column should decode as a boolean-ish value, got {active:?}"
        );
    }

    #[tokio::test]
    async fn test_wait_for_query_form() {
        let code = r#"
open database at "sqlite::memory:" as db
store created as execute db with "CREATE TABLE t (x INTEGER)"
store inserted as execute db with "INSERT INTO t (x) VALUES (7)"
wait for store rows as query db with "SELECT x FROM t"
close database db
"#;
        let interpreter = run_wfl(code).await.expect("program should run");
        let rows = expect_list(&get_global(&interpreter, "rows"));
        assert_eq!(expect_number(&expect_object_key(&rows[0], "x")), 7.0);
    }

    #[tokio::test]
    async fn test_return_query_with_parameters_from_action() {
        // Issue #559: `return query ... and parameters [...]` failed to parse.
        let code = r#"
open database at "sqlite::memory:" as db
store ig as execute db with "CREATE TABLE t (id INT, n INT)"
store ig2 as execute db with "INSERT INTO t (id, n) VALUES (1, 5)"
store ig3 as execute db with "INSERT INTO t (id, n) VALUES (2, 9)"

define action called get_n with parameters conn and id:
    return query conn with "SELECT n FROM t WHERE id = ?" and parameters [id]
end action

store rows as call get_n with db and 1
close database db
"#;
        let interpreter = run_wfl(code).await.expect("program should run");
        let rows = expect_list(&get_global(&interpreter, "rows"));
        assert_eq!(rows.len(), 1);
        assert_eq!(expect_number(&expect_object_key(&rows[0], "n")), 5.0);
    }

    #[tokio::test]
    async fn test_return_query_without_parameters_from_action() {
        let code = r#"
open database at "sqlite::memory:" as db
store ig as execute db with "CREATE TABLE t (n INT)"
store ig2 as execute db with "INSERT INTO t (n) VALUES (3)"

define action called get_all with parameters conn:
    return query conn with "SELECT n FROM t"
end action

store rows as call get_all with db
close database db
"#;
        let interpreter = run_wfl(code).await.expect("program should run");
        let rows = expect_list(&get_global(&interpreter, "rows"));
        assert_eq!(rows.len(), 1);
        assert_eq!(expect_number(&expect_object_key(&rows[0], "n")), 3.0);
    }

    #[tokio::test]
    async fn test_return_execute_with_parameters_from_action() {
        let code = r#"
open database at "sqlite::memory:" as db
store ig as execute db with "CREATE TABLE t (id INT)"

define action called add_row with parameters conn and id:
    return execute conn with "INSERT INTO t (id) VALUES (?)" and parameters [id]
end action

store result as call add_row with db and 42
store rows as query db with "SELECT id FROM t"
close database db
"#;
        let interpreter = run_wfl(code).await.expect("program should run");
        let result = get_global(&interpreter, "result");
        assert_eq!(
            expect_number(&expect_object_key(&result, "affected_rows")),
            1.0
        );
        let rows = expect_list(&get_global(&interpreter, "rows"));
        assert_eq!(expect_number(&expect_object_key(&rows[0], "id")), 42.0);
    }

    #[tokio::test]
    async fn test_connect_to_database_alias() {
        let code = r#"
connect to database at "sqlite::memory:" as db
store created as execute db with "CREATE TABLE t (x INTEGER)"
close database db
"#;
        run_wfl(code)
            .await
            .expect("connect to database should work");
    }

    #[tokio::test]
    async fn test_bad_scheme_is_catchable() {
        let code = r#"
store result as "no error"
try:
    open database at "oracle://localhost/db" as db
when error:
    store result as "caught"
end try
"#;
        let interpreter = run_wfl(code).await.expect("error should be catchable");
        assert_eq!(expect_text(&get_global(&interpreter, "result")), "caught");
    }

    #[tokio::test]
    async fn test_malformed_sql_is_catchable() {
        let code = r#"
open database at "sqlite::memory:" as db
store result as "no error"
try:
    store rows as query db with "SELEKT * FROM nowhere"
when error:
    store result as "caught"
end try
close database db
"#;
        let interpreter = run_wfl(code).await.expect("error should be catchable");
        assert_eq!(expect_text(&get_global(&interpreter, "result")), "caught");
    }

    #[tokio::test]
    async fn test_unknown_handle_errors() {
        let code = r#"
store result as "no error"
try:
    store rows as query missing_db with "SELECT 1"
when error:
    store result as "caught"
end try
"#;
        let interpreter = run_wfl(code).await.expect("error should be catchable");
        assert_eq!(expect_text(&get_global(&interpreter, "result")), "caught");
    }

    #[tokio::test]
    async fn test_query_after_close_errors() {
        let code = r#"
open database at "sqlite::memory:" as db
close database db
store result as "no error"
try:
    store rows as query db with "SELECT 1"
when error:
    store result as "caught"
end try
"#;
        let interpreter = run_wfl(code).await.expect("error should be catchable");
        assert_eq!(expect_text(&get_global(&interpreter, "result")), "caught");
    }
}

mod gated_backend_tests {
    use super::*;

    async fn run_crud_matrix(url: &str, placeholder: &dyn Fn(usize) -> String) {
        let table = format!("wfl_test_{}", std::process::id());
        let p1 = placeholder(1);
        let p2 = placeholder(2);
        let code = format!(
            r#"
open database at "{url}" as db
store dropped as execute db with "DROP TABLE IF EXISTS {table}"
store created as execute db with "CREATE TABLE {table} (id INTEGER, name VARCHAR(100), active BOOLEAN)"
store inserted as execute db with "INSERT INTO {table} (id, name, active) VALUES ({p1}, {p2}, TRUE)" and parameters [7 and "Bob"]
store rows as query db with "SELECT id, name, active FROM {table}"
store cleaned as execute db with "DROP TABLE {table}"
close database db
"#
        );
        let interpreter = run_wfl(&code).await.expect("program should run");

        let inserted = get_global(&interpreter, "inserted");
        assert_eq!(
            expect_number(&expect_object_key(&inserted, "affected_rows")),
            1.0
        );

        let rows = expect_list(&get_global(&interpreter, "rows"));
        assert_eq!(rows.len(), 1);
        assert_eq!(expect_number(&expect_object_key(&rows[0], "id")), 7.0);
        assert_eq!(expect_text(&expect_object_key(&rows[0], "name")), "Bob");
        let active = expect_object_key(&rows[0], "active");
        assert!(
            matches!(active, Value::Bool(true) | Value::Number(_)),
            "BOOLEAN should decode as a boolean-ish value, got {active:?}"
        );
    }

    #[tokio::test]
    async fn test_postgres_crud() {
        let Ok(url) = std::env::var("WFL_TEST_POSTGRES_URL") else {
            println!("Skipping PostgreSQL test: WFL_TEST_POSTGRES_URL not set");
            return;
        };
        run_crud_matrix(&url, &|n| format!("${n}")).await;
    }

    #[tokio::test]
    async fn test_postgres_returning_clause() {
        let Ok(url) = std::env::var("WFL_TEST_POSTGRES_URL") else {
            println!("Skipping PostgreSQL test: WFL_TEST_POSTGRES_URL not set");
            return;
        };
        let table = format!("wfl_ret_{}", std::process::id());
        let code = format!(
            r#"
open database at "{url}" as db
store dropped as execute db with "DROP TABLE IF EXISTS {table}"
store created as execute db with "CREATE TABLE {table} (id SERIAL PRIMARY KEY, name TEXT)"
store rows as query db with "INSERT INTO {table} (name) VALUES ($1) RETURNING id" and parameters ["Carol"]
store cleaned as execute db with "DROP TABLE {table}"
close database db
"#
        );
        let interpreter = run_wfl(&code).await.expect("program should run");
        let rows = expect_list(&get_global(&interpreter, "rows"));
        assert_eq!(expect_number(&expect_object_key(&rows[0], "id")), 1.0);
    }

    #[tokio::test]
    async fn test_mariadb_crud() {
        let Ok(url) = std::env::var("WFL_TEST_MYSQL_URL") else {
            println!("Skipping MariaDB test: WFL_TEST_MYSQL_URL not set");
            return;
        };
        run_crud_matrix(&url, &|_| "?".to_string()).await;
    }

    #[tokio::test]
    async fn test_mariadb_last_insert_id() {
        let Ok(url) = std::env::var("WFL_TEST_MYSQL_URL") else {
            println!("Skipping MariaDB test: WFL_TEST_MYSQL_URL not set");
            return;
        };
        let table = format!("wfl_lid_{}", std::process::id());
        let code = format!(
            r#"
open database at "{url}" as db
store dropped as execute db with "DROP TABLE IF EXISTS {table}"
store created as execute db with "CREATE TABLE {table} (id INT AUTO_INCREMENT PRIMARY KEY, name TEXT)"
store inserted as execute db with "INSERT INTO {table} (name) VALUES (?)" and parameters ["Dave"]
store cleaned as execute db with "DROP TABLE {table}"
close database db
"#
        );
        let interpreter = run_wfl(&code).await.expect("program should run");
        let inserted = get_global(&interpreter, "inserted");
        assert_eq!(
            expect_number(&expect_object_key(&inserted, "last_insert_id")),
            1.0
        );
    }
}
