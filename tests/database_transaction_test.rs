// TDD tests for the `in transaction on <db>:` block (issue #664).
//
// These MUST use file-backed SQLite. `open database` hands out a pool of
// MAX_POOL_CONNECTIONS (5) connections, and the bug in #664 is that each
// statement lands on a different one. In-memory SQLite is special-cased to a
// single connection (src/interpreter/database.rs), which hides the defect
// entirely — a program can pass its in-memory tests and still lose data in
// production. Every atomicity assertion here therefore runs against a temp file.

use std::path::{Path, PathBuf};
use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Run WFL code and return the interpreter for inspecting globals.
async fn run_wfl(code: &str) -> Result<Interpreter, String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().map_err(|e| format!("Parse error: {e:?}"))?;

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&ast)
        .await
        .map_err(|e| format!("Runtime error: {e:?}"))?;
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

fn expect_list(value: &Value) -> Vec<Value> {
    match value {
        Value::List(list) => list.borrow().clone(),
        other => panic!("Expected list, got {other:?}"),
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

/// A temp-file SQLite database that removes itself on drop.
///
/// File-backed on purpose — see the module comment.
struct TempDb {
    url: String,
    path: PathBuf,
}

impl TempDb {
    fn new(test_name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "wfl_tx_test_{}_{}.db",
            test_name,
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let url = format!("sqlite://{}", path.display()).replace('\\', "/");
        Self { url, path }
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        // SQLite may leave a -wal / -shm sidecar next to the database.
        for suffix in ["-wal", "-shm"] {
            let mut sidecar = self.path.clone().into_os_string();
            sidecar.push(suffix);
            let _ = std::fs::remove_file(Path::new(&sidecar));
        }
    }
}

/// Count rows in `projects` with a fresh connection, after the program is done.
async fn surviving_rows(url: &str) -> f64 {
    let code = format!(
        r#"
open database at "{url}" as db
store rows as query db with "SELECT slug FROM projects"
store n as length of rows
close database db
"#
    );
    let interpreter = run_wfl(&code).await.expect("count program should run");
    expect_number(&get_global(&interpreter, "n"))
}

// ---------------------------------------------------------------------------
// The reproduction from issue #664, verbatim in spirit: a rollback must roll back.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn transaction_block_rolls_back_on_error() {
    let db = TempDb::new("rollback_on_error");
    let url = &db.url;
    let code = format!(
        r#"
open database at "{url}" as db
store made as execute db with "CREATE TABLE projects (slug TEXT)"

store failed as no
try:
    in transaction on db:
        store ins as execute db with "INSERT INTO projects (slug) VALUES ('should-vanish')"
        store boom as execute db with "INSERT INTO nonexistent_table (x) VALUES (1)"
    end transaction
when error:
    change failed to yes
end try

close database db
"#
    );
    let interpreter = run_wfl(&code).await.expect("program should run");
    assert_eq!(
        get_global(&interpreter, "failed"),
        Value::Bool(true),
        "the failing statement inside the block must surface as an error"
    );

    assert_eq!(
        surviving_rows(url).await,
        0.0,
        "issue #664: rows written inside a rolled-back transaction must not survive"
    );
}

#[tokio::test]
async fn transaction_block_commits_on_normal_exit() {
    let db = TempDb::new("commit_on_exit");
    let url = &db.url;
    let code = format!(
        r#"
open database at "{url}" as db
store made as execute db with "CREATE TABLE projects (slug TEXT)"

in transaction on db:
    store a as execute db with "INSERT INTO projects (slug) VALUES ('kept-one')"
    store b as execute db with "INSERT INTO projects (slug) VALUES ('kept-two')"
end transaction

close database db
"#
    );
    run_wfl(&code).await.expect("program should run");

    assert_eq!(
        surviving_rows(url).await,
        2.0,
        "both writes must be visible after the block commits"
    );
}

#[tokio::test]
async fn transaction_block_is_atomic_all_or_nothing() {
    let db = TempDb::new("all_or_nothing");
    let url = &db.url;
    let code = format!(
        r#"
open database at "{url}" as db
store made as execute db with "CREATE TABLE projects (slug TEXT UNIQUE)"
store seed as execute db with "INSERT INTO projects (slug) VALUES ('taken')"

store failed as no
try:
    in transaction on db:
        store a as execute db with "INSERT INTO projects (slug) VALUES ('new-one')"
        store b as execute db with "INSERT INTO projects (slug) VALUES ('taken')"
    end transaction
when error:
    change failed to yes
end try

close database db
"#
    );
    let interpreter = run_wfl(&code).await.expect("program should run");
    assert_eq!(get_global(&interpreter, "failed"), Value::Bool(true));

    assert_eq!(
        surviving_rows(url).await,
        1.0,
        "the successful first insert must roll back with the failed second one"
    );
}

#[tokio::test]
async fn reads_inside_transaction_see_uncommitted_writes() {
    let db = TempDb::new("read_own_writes");
    let url = &db.url;
    let code = format!(
        r#"
open database at "{url}" as db
store made as execute db with "CREATE TABLE projects (slug TEXT)"

in transaction on db:
    store ins as execute db with "INSERT INTO projects (slug) VALUES ('pending')"
    store rows as query db with "SELECT slug FROM projects"
    store seen as length of rows
end transaction

close database db
"#
    );
    let interpreter = run_wfl(&code).await.expect("program should run");
    assert_eq!(
        expect_number(&get_global(&interpreter, "seen")),
        1.0,
        "a query inside the block must run on the transaction's own connection \
         and see its uncommitted write"
    );
}

// ---------------------------------------------------------------------------
// Lifecycle and misuse (R3: lifecycle + negative paths)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn nested_transaction_on_same_handle_is_a_clear_error() {
    let db = TempDb::new("nested");
    let url = &db.url;
    let code = format!(
        r#"
open database at "{url}" as db
store made as execute db with "CREATE TABLE projects (slug TEXT)"
in transaction on db:
    in transaction on db:
        store a as execute db with "INSERT INTO projects (slug) VALUES ('x')"
    end transaction
end transaction
close database db
"#
    );
    let err = run_wfl(&code)
        .await
        .err()
        .expect("nesting a transaction on the same handle must fail");
    let lower = err.to_lowercase();
    assert!(
        lower.contains("transaction"),
        "error should name the transaction, got: {err}"
    );
    assert!(
        lower.contains("nest") || lower.contains("already"),
        "error should explain that a transaction is already open, got: {err}"
    );
}

#[tokio::test]
async fn closing_database_inside_transaction_is_a_clear_error() {
    let db = TempDb::new("close_inside");
    let url = &db.url;
    let code = format!(
        r#"
open database at "{url}" as db
store made as execute db with "CREATE TABLE projects (slug TEXT)"
in transaction on db:
    close database db
end transaction
"#
    );
    let err = run_wfl(&code)
        .await
        .err()
        .expect("closing a database mid-transaction must fail");
    assert!(
        err.to_lowercase().contains("transaction"),
        "error should name the open transaction, got: {err}"
    );
}

#[tokio::test]
async fn open_transaction_rolls_back_when_program_ends() {
    // A program that never reaches `end transaction` (the error escapes the
    // block and the whole program) must not leave the write committed.
    let db = TempDb::new("abandoned");
    let url = &db.url;
    let code = format!(
        r#"
open database at "{url}" as db
store made as execute db with "CREATE TABLE projects (slug TEXT)"
in transaction on db:
    store ins as execute db with "INSERT INTO projects (slug) VALUES ('abandoned')"
    store boom as execute db with "THIS IS NOT SQL"
end transaction
close database db
"#
    );
    run_wfl(&code)
        .await
        .err()
        .expect("the invalid statement should fail the program");

    assert_eq!(
        surviving_rows(url).await,
        0.0,
        "an abandoned transaction must roll back, not leak a committed write"
    );
}

// ---------------------------------------------------------------------------
// Raw transaction-control SQL must fail loudly instead of silently no-op'ing.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn raw_begin_through_execute_is_rejected() {
    let db = TempDb::new("raw_begin");
    let url = &db.url;
    let code = format!(
        r#"
open database at "{url}" as db
store made as execute db with "CREATE TABLE projects (slug TEXT)"
store t1 as execute db with "BEGIN"
close database db
"#
    );
    let err = run_wfl(&code)
        .await
        .err()
        .expect("raw BEGIN must be rejected, not silently ignored");
    assert!(
        err.contains("in transaction on"),
        "the error must point at the transaction block, got: {err}"
    );
}

#[tokio::test]
async fn raw_commit_and_rollback_through_execute_are_rejected() {
    let db = TempDb::new("raw_commit");
    let url = &db.url;
    for sql in ["COMMIT", "ROLLBACK", "START TRANSACTION", "  begin  "] {
        let code = format!(
            r#"
open database at "{url}" as db
store t as execute db with "{sql}"
close database db
"#
        );
        let err = match run_wfl(&code).await {
            Ok(_) => panic!("raw {sql} must be rejected, not silently ignored"),
            Err(e) => e,
        };
        assert!(
            err.contains("in transaction on"),
            "the error for {sql} must point at the transaction block, got: {err}"
        );
    }
}

#[tokio::test]
async fn ordinary_sql_beginning_with_a_transaction_word_is_not_rejected() {
    // The rejection matches the leading *statement* keyword. A column or table
    // named `begin_at`, or a SELECT that merely mentions COMMIT, must still run.
    let db = TempDb::new("not_overreaching");
    let url = &db.url;
    let code = format!(
        r#"
open database at "{url}" as db
store made as execute db with "CREATE TABLE audit (begin_at TEXT, commit_note TEXT)"
store ins as execute db with "INSERT INTO audit (begin_at, commit_note) VALUES ('t0', 'rollback plan')"
store rows as query db with "SELECT begin_at, commit_note FROM audit"
store n as length of rows
close database db
"#
    );
    let interpreter = run_wfl(&code)
        .await
        .expect("ordinary SQL that merely contains transaction words must still run");
    assert_eq!(expect_number(&get_global(&interpreter, "n")), 1.0);
}

// ---------------------------------------------------------------------------
// Concurrency (§11.3): a transaction pins one connection, and must not wedge
// unrelated work on the rest of the pool.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn transaction_does_not_block_other_handles_on_the_same_file() {
    let db = TempDb::new("no_wedge");
    let url = &db.url;
    let code = format!(
        r#"
open database at "{url}" as db
store made as execute db with "CREATE TABLE projects (slug TEXT)"
store seed as execute db with "INSERT INTO projects (slug) VALUES ('already-there')"

open database at "{url}" as other
in transaction on db:
    store ins as execute db with "INSERT INTO projects (slug) VALUES ('in-flight')"
    store rows as query other with "SELECT slug FROM projects"
    store visible as length of rows
end transaction
close database other
close database db
"#
    );
    let interpreter = run_wfl(&code)
        .await
        .expect("a second handle must stay usable while a transaction is open");
    assert_eq!(
        expect_number(&get_global(&interpreter, "visible")),
        1.0,
        "the other handle must see only the committed row, and must not deadlock"
    );

    assert_eq!(surviving_rows(url).await, 2.0);
}

#[tokio::test]
async fn execute_result_shape_is_unchanged_inside_a_transaction() {
    // Backward compatibility: `execute` returns the same object whether or not
    // it runs inside a transaction block.
    let db = TempDb::new("result_shape");
    let url = &db.url;
    let code = format!(
        r#"
open database at "{url}" as db
store made as execute db with "CREATE TABLE t (x INTEGER)"
in transaction on db:
    store inserted as execute db with "INSERT INTO t (x) VALUES (?)" and parameters [7]
end transaction
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
}

#[tokio::test]
async fn query_inside_transaction_returns_rows_as_usual() {
    let db = TempDb::new("query_shape");
    let url = &db.url;
    let code = format!(
        r#"
open database at "{url}" as db
store made as execute db with "CREATE TABLE t (x INTEGER)"
store seed as execute db with "INSERT INTO t (x) VALUES (5)"
in transaction on db:
    store rows as query db with "SELECT x FROM t"
end transaction
close database db
"#
    );
    let interpreter = run_wfl(&code).await.expect("program should run");
    let rows = expect_list(&get_global(&interpreter, "rows"));
    assert_eq!(rows.len(), 1);
    assert_eq!(expect_number(&expect_object_key(&rows[0], "x")), 5.0);
}
