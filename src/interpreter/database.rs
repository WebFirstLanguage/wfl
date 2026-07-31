//! Database connectivity for WFL programs, backed by sqlx.
//!
//! Supports PostgreSQL (`postgres://`/`postgresql://`), MariaDB/MySQL
//! (`mariadb://`/`mysql://`), and SQLite (`sqlite://path`, `sqlite::memory:`).
//! Connections are pooled and addressed from WFL code through string handles
//! ("db1", "db2", ...) managed by the interpreter's IoClient, mirroring file
//! handles.
//!
//! All SQL runs through runtime `sqlx::query` with explicit `.bind()` calls —
//! parameter values are never interpolated into SQL text. Placeholders are
//! driver-native: `?` for SQLite/MariaDB, `$1` for PostgreSQL.
//!
//! The query text is a runtime WFL value, not a `&'static str`, so every
//! `sqlx::query` call below wraps it in [`sqlx::AssertSqlSafe`] to opt out of
//! sqlx 0.9's `SqlSafeStr` gate. That opt-out asserts nothing about the text's
//! own safety — only the bound parameter *values* are injection-safe (via
//! `.bind()`), and WFL programs must not build SQL text from untrusted input.

use super::value::Value;
use sqlx::mysql::{MySqlPoolOptions, MySqlRow};
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow};
use sqlx::{Column, Row, TypeInfo, ValueRef};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

const MAX_POOL_CONNECTIONS: u32 = 5;

/// A connection pool to one of the supported database backends.
#[derive(Clone)]
pub enum DbPool {
    Postgres(sqlx::PgPool),
    /// Also serves MariaDB, which speaks the MySQL protocol.
    MySql(sqlx::MySqlPool),
    Sqlite(sqlx::SqlitePool),
}

/// An open transaction holding one connection out of a [`DbPool`] for its
/// whole lifetime (issue #664).
///
/// `Pool::begin` hands back an owned `Transaction<'static, DB>`, so the
/// interpreter can park one of these in a map keyed by database handle and run
/// every statement inside `in transaction on db:` against it. Dropping it
/// without committing rolls back, which is what makes an abandoned transaction
/// safe rather than a leaked half-write.
pub enum DbTransaction {
    Postgres(sqlx::Transaction<'static, sqlx::Postgres>),
    MySql(sqlx::Transaction<'static, sqlx::MySql>),
    Sqlite(sqlx::Transaction<'static, sqlx::Sqlite>),
}

/// Where a statement runs: straight against the pool (any free connection), or
/// against the single connection an open transaction is holding.
pub enum DbTarget<'a> {
    Pool(&'a DbPool),
    Transaction(&'a mut DbTransaction),
}

/// An owned, Send-safe SQL bind parameter converted from a WFL `Value`.
#[derive(Debug, Clone)]
pub enum SqlParam {
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Binary(Vec<u8>),
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
    DateTime(chrono::NaiveDateTime),
    Null,
}

/// Convert a WFL value into a bindable SQL parameter.
pub fn value_to_sql_param(value: &Value) -> Result<SqlParam, String> {
    match value {
        // Whole numbers bind as integers so strict backends (PostgreSQL)
        // accept them in integer columns. Magnitudes beyond i64 range would
        // saturate in the cast, so they fall through to float binding.
        Value::Number(n)
            if n.fract() == 0.0
                && n.is_finite()
                && *n >= i64::MIN as f64
                && *n <= i64::MAX as f64 =>
        {
            Ok(SqlParam::Int(*n as i64))
        }
        Value::Number(n) => Ok(SqlParam::Float(*n)),
        Value::Text(t) => Ok(SqlParam::Text(t.to_string())),
        Value::Bool(b) => Ok(SqlParam::Bool(*b)),
        Value::Binary(b) => Ok(SqlParam::Binary(b.to_vec())),
        Value::Date(d) => Ok(SqlParam::Date(**d)),
        Value::Time(t) => Ok(SqlParam::Time(**t)),
        Value::DateTime(dt) => Ok(SqlParam::DateTime(**dt)),
        Value::Null | Value::Nothing => Ok(SqlParam::Null),
        other => Err(format!(
            "Cannot bind value of type {} as a query parameter",
            other.type_name()
        )),
    }
}

/// Open a pooled connection, routed by the URL scheme.
pub async fn connect(url: &str) -> Result<DbPool, String> {
    if url.starts_with("sqlite:") {
        let options = if url == "sqlite::memory:" || url == "sqlite://:memory:" {
            SqliteConnectOptions::new().in_memory(true)
        } else {
            let path = url
                .strip_prefix("sqlite://")
                .or_else(|| url.strip_prefix("sqlite:"))
                .unwrap_or(url);
            SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true)
        };
        // In-memory SQLite databases exist per connection, so the pool must
        // not hand out more than one.
        let max_connections = if url.contains(":memory:") {
            1
        } else {
            MAX_POOL_CONNECTIONS
        };
        SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect_with(options)
            .await
            .map(DbPool::Sqlite)
            .map_err(|e| format!("Failed to connect to SQLite database: {e}"))
    } else if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        PgPoolOptions::new()
            .max_connections(MAX_POOL_CONNECTIONS)
            .connect(url)
            .await
            .map(DbPool::Postgres)
            .map_err(|e| format!("Failed to connect to PostgreSQL database: {e}"))
    } else if url.starts_with("mysql://") || url.starts_with("mariadb://") {
        let url = url.replacen("mariadb://", "mysql://", 1);
        MySqlPoolOptions::new()
            .max_connections(MAX_POOL_CONNECTIONS)
            .connect(&url)
            .await
            .map(DbPool::MySql)
            .map_err(|e| format!("Failed to connect to MariaDB/MySQL database: {e}"))
    } else {
        Err(
            "Unsupported database URL scheme. Supported schemes: sqlite://, postgres://, postgresql://, mysql://, mariadb://"
                .to_string(),
        )
    }
}

/// Statement keywords that open, close, or checkpoint a transaction.
///
/// See [`reject_transaction_control_sql`].
const TRANSACTION_CONTROL_KEYWORDS: &[&str] = &[
    "begin",
    "commit",
    "rollback",
    "start",
    "savepoint",
    "release",
    "end",
];

/// Reject transaction-control SQL sent through `query`/`execute` (issue #664).
///
/// Each `query`/`execute` takes whichever pooled connection happens to be free,
/// so a hand-written `BEGIN` … `ROLLBACK` sequence lands on *different*
/// connections and the rollback silently does nothing — the writes it was meant
/// to undo survive, with no error anywhere. That is a data-integrity failure
/// that looks exactly like success, and it is invisible under in-memory SQLite
/// (capped at one connection) where most tests run.
///
/// Rather than keep failing silently, refuse the statement and name the
/// construct that actually works. Only the *leading* keyword is inspected, so a
/// column called `begin_at` or a string containing the word "commit" is
/// untouched.
///
/// Leading comments are skipped first (see [`skip_leading_comments`]): a guard
/// that stopped at the first non-alphabetic character would read an empty word
/// from `-- go\nBEGIN` and wave it through, which is the case it exists to stop.
/// Return `sql` with any leading whitespace and SQL comments removed, so the
/// caller sees the first real token.
///
/// Handles both comment forms (`-- to end of line` and `/* … */`), repeated and
/// interleaved. An unterminated block comment consumes the rest of the input and
/// yields an empty string — the statement is then left to the database to
/// reject, which is the right outcome for text that is not valid SQL anyway.
fn skip_leading_comments(sql: &str) -> &str {
    let mut rest = sql.trim_start();
    loop {
        if let Some(after) = rest.strip_prefix("--") {
            // A line comment runs to the newline, or to the end of the input.
            rest = match after.find('\n') {
                Some(newline) => after[newline + 1..].trim_start(),
                None => return "",
            };
        } else if let Some(after) = rest.strip_prefix("/*") {
            rest = match after.find("*/") {
                Some(end) => after[end + 2..].trim_start(),
                None => return "",
            };
        } else {
            return rest;
        }
    }
}

pub fn reject_transaction_control_sql(sql: &str) -> Result<(), String> {
    let first_word: String = skip_leading_comments(sql)
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();

    if first_word.is_empty() {
        return Ok(());
    }

    let lowered = first_word.to_ascii_lowercase();
    if TRANSACTION_CONTROL_KEYWORDS.contains(&lowered.as_str()) {
        return Err(format!(
            "'{first_word}' controls a transaction, and sending it through query/execute does \
             not work: each statement runs on its own pooled connection, so the transaction \
             would not cover the statements you meant it to. Use a transaction block instead:\n\
             \n    in transaction on db:\n        execute db with \"...\"\n    end transaction\n\
             \nThe block holds one connection for its whole body, commits when it finishes, \
             and rolls back if anything inside it fails."
        ));
    }

    Ok(())
}

/// Run a row-returning statement; rows become a list of objects keyed by
/// column name.
pub async fn run_query(
    target: DbTarget<'_>,
    sql: &str,
    params: &[SqlParam],
) -> Result<Value, String> {
    reject_transaction_control_sql(sql)?;

    // `AssertSqlSafe`: opt out of sqlx's `SqlSafeStr` gate for our runtime SQL
    // text (see the module docs for the safety contract).
    macro_rules! fetch {
        ($executor:expr, $bind:ident, $to_value:ident) => {{
            let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
            for param in params {
                query = $bind(query, param);
            }
            let rows = query
                .fetch_all($executor)
                .await
                .map_err(|e| format!("Query failed: {e}"))?;
            rows.iter()
                .map($to_value)
                .collect::<Result<Vec<Value>, String>>()?
        }};
    }

    let rows: Vec<Value> = match target {
        DbTarget::Pool(DbPool::Sqlite(pool)) => fetch!(pool, bind_sqlite, sqlite_row_to_value),
        DbTarget::Pool(DbPool::Postgres(pool)) => fetch!(pool, bind_postgres, pg_row_to_value),
        DbTarget::Pool(DbPool::MySql(pool)) => fetch!(pool, bind_mysql, mysql_row_to_value),
        DbTarget::Transaction(DbTransaction::Sqlite(tx)) => {
            fetch!(&mut **tx, bind_sqlite, sqlite_row_to_value)
        }
        DbTarget::Transaction(DbTransaction::Postgres(tx)) => {
            fetch!(&mut **tx, bind_postgres, pg_row_to_value)
        }
        DbTarget::Transaction(DbTransaction::MySql(tx)) => {
            fetch!(&mut **tx, bind_mysql, mysql_row_to_value)
        }
    };

    Ok(Value::List(Rc::new(RefCell::new(rows))))
}

/// Run a non-returning statement; the result is an object with
/// `affected_rows` and `last_insert_id` (nothing on PostgreSQL — use
/// `RETURNING` there instead).
pub async fn run_execute(
    target: DbTarget<'_>,
    sql: &str,
    params: &[SqlParam],
) -> Result<Value, String> {
    reject_transaction_control_sql(sql)?;

    // `AssertSqlSafe`: opt out of sqlx's `SqlSafeStr` gate for our runtime SQL
    // text (see the module docs for the safety contract).
    macro_rules! run {
        ($executor:expr, $bind:ident, $last_id:expr) => {{
            let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
            for param in params {
                query = $bind(query, param);
            }
            let result = query
                .execute($executor)
                .await
                .map_err(|e| format!("Execute failed: {e}"))?;
            #[allow(clippy::redundant_closure_call)]
            (result.rows_affected(), $last_id(&result))
        }};
    }

    let sqlite_id = |r: &sqlx::sqlite::SqliteQueryResult| Some(r.last_insert_rowid());
    let mysql_id = |r: &sqlx::mysql::MySqlQueryResult| Some(r.last_insert_id() as i64);
    let pg_id = |_: &sqlx::postgres::PgQueryResult| None;

    let (affected_rows, last_insert_id): (u64, Option<i64>) = match target {
        DbTarget::Pool(DbPool::Sqlite(pool)) => run!(pool, bind_sqlite, sqlite_id),
        DbTarget::Pool(DbPool::Postgres(pool)) => run!(pool, bind_postgres, pg_id),
        DbTarget::Pool(DbPool::MySql(pool)) => run!(pool, bind_mysql, mysql_id),
        DbTarget::Transaction(DbTransaction::Sqlite(tx)) => {
            run!(&mut **tx, bind_sqlite, sqlite_id)
        }
        DbTarget::Transaction(DbTransaction::Postgres(tx)) => {
            run!(&mut **tx, bind_postgres, pg_id)
        }
        DbTarget::Transaction(DbTransaction::MySql(tx)) => {
            run!(&mut **tx, bind_mysql, mysql_id)
        }
    };

    let mut object = HashMap::new();
    object.insert(
        "affected_rows".to_string(),
        Value::Number(affected_rows as f64),
    );
    object.insert(
        "last_insert_id".to_string(),
        match last_insert_id {
            Some(id) => Value::Number(id as f64),
            // Value::Null is the runtime value of WFL's `nothing` literal
            None => Value::Null,
        },
    );

    Ok(Value::Object(Rc::new(RefCell::new(object))))
}

/// Take one connection out of the pool and open a transaction on it.
pub async fn begin(pool: &DbPool) -> Result<DbTransaction, String> {
    match pool {
        DbPool::Sqlite(pool) => pool
            .begin()
            .await
            .map(DbTransaction::Sqlite)
            .map_err(|e| format!("Failed to start transaction: {e}")),
        DbPool::Postgres(pool) => pool
            .begin()
            .await
            .map(DbTransaction::Postgres)
            .map_err(|e| format!("Failed to start transaction: {e}")),
        DbPool::MySql(pool) => pool
            .begin()
            .await
            .map(DbTransaction::MySql)
            .map_err(|e| format!("Failed to start transaction: {e}")),
    }
}

/// Commit a transaction, returning its connection to the pool.
pub async fn commit(tx: DbTransaction) -> Result<(), String> {
    match tx {
        DbTransaction::Sqlite(tx) => tx.commit().await,
        DbTransaction::Postgres(tx) => tx.commit().await,
        DbTransaction::MySql(tx) => tx.commit().await,
    }
    .map_err(|e| format!("Failed to commit transaction: {e}"))
}

/// Roll a transaction back, returning its connection to the pool.
///
/// Dropping a `DbTransaction` without calling this also rolls back; this exists
/// so the interpreter can report a rollback that itself fails, rather than
/// discarding it silently.
pub async fn rollback(tx: DbTransaction) -> Result<(), String> {
    match tx {
        DbTransaction::Sqlite(tx) => tx.rollback().await,
        DbTransaction::Postgres(tx) => tx.rollback().await,
        DbTransaction::MySql(tx) => tx.rollback().await,
    }
    .map_err(|e| format!("Failed to roll back transaction: {e}"))
}

/// Close the pool, ending all connections.
pub async fn close(pool: DbPool) {
    match pool {
        DbPool::Postgres(pool) => pool.close().await,
        DbPool::MySql(pool) => pool.close().await,
        DbPool::Sqlite(pool) => pool.close().await,
    }
}

macro_rules! bind_param {
    ($fn_name:ident, $db:ty) => {
        fn $fn_name<'q>(
            query: sqlx::query::Query<'q, $db, <$db as sqlx::Database>::Arguments>,
            param: &SqlParam,
        ) -> sqlx::query::Query<'q, $db, <$db as sqlx::Database>::Arguments> {
            match param {
                SqlParam::Int(v) => query.bind(*v),
                SqlParam::Float(v) => query.bind(*v),
                SqlParam::Text(v) => query.bind(v.clone()),
                SqlParam::Bool(v) => query.bind(*v),
                SqlParam::Binary(v) => query.bind(v.clone()),
                SqlParam::Date(v) => query.bind(*v),
                SqlParam::Time(v) => query.bind(*v),
                SqlParam::DateTime(v) => query.bind(*v),
                SqlParam::Null => query.bind(Option::<String>::None),
            }
        }
    };
}

bind_param!(bind_sqlite, sqlx::Sqlite);
bind_param!(bind_postgres, sqlx::Postgres);
bind_param!(bind_mysql, sqlx::MySql);

fn sqlite_int(row: &SqliteRow, index: usize) -> Result<Value, sqlx::Error> {
    row.try_get::<i64, _>(index)
        .map(|v| Value::Number(v as f64))
}

fn pg_int(row: &PgRow, index: usize) -> Result<Value, sqlx::Error> {
    // PostgreSQL decoding is strict about integer widths (INT2/INT4/INT8).
    row.try_get::<i64, _>(index)
        .map(|v| Value::Number(v as f64))
        .or_else(|_| {
            row.try_get::<i32, _>(index)
                .map(|v| Value::Number(v as f64))
        })
        .or_else(|_| {
            row.try_get::<i16, _>(index)
                .map(|v| Value::Number(v as f64))
        })
}

fn mysql_int(row: &MySqlRow, index: usize) -> Result<Value, sqlx::Error> {
    // MySQL/MariaDB UNSIGNED columns decode as u64 only.
    row.try_get::<i64, _>(index)
        .map(|v| Value::Number(v as f64))
        .or_else(|_| {
            row.try_get::<u64, _>(index)
                .map(|v| Value::Number(v as f64))
        })
        .or_else(|_| {
            row.try_get::<i32, _>(index)
                .map(|v| Value::Number(v as f64))
        })
        .or_else(|_| {
            row.try_get::<i16, _>(index)
                .map(|v| Value::Number(v as f64))
        })
}

/// Decode a column by its reported type name into a WFL value. Shared
/// decision tree; the integer decoder differs per driver.
macro_rules! row_to_value {
    ($fn_name:ident, $row:ty, $int_fn:path) => {
        fn $fn_name(row: &$row) -> Result<Value, String> {
            let mut object = HashMap::new();

            for (index, column) in row.columns().iter().enumerate() {
                let name = column.name().to_string();

                let is_null = row
                    .try_get_raw(index)
                    .map(|raw| raw.is_null())
                    .unwrap_or(true);
                if is_null {
                    // Value::Null is the runtime value of WFL's `nothing`
                    // literal, so `is nothing` comparisons work on NULLs.
                    object.insert(name, Value::Null);
                    continue;
                }

                let type_name = column.type_info().name().to_uppercase();
                let value = if type_name.contains("BOOL") {
                    row.try_get::<bool, _>(index)
                        .map(Value::Bool)
                        .or_else(|_| row.try_get::<i64, _>(index).map(|v| Value::Bool(v != 0)))
                        .map_err(|e| decode_error(&name, &type_name, e))?
                } else if type_name.contains("INT") || type_name == "SERIAL" {
                    $int_fn(row, index).map_err(|e| decode_error(&name, &type_name, e))?
                } else if type_name.contains("REAL")
                    || type_name.contains("FLOAT")
                    || type_name.contains("DOUBLE")
                {
                    row.try_get::<f64, _>(index)
                        .map(Value::Number)
                        .map_err(|e| decode_error(&name, &type_name, e))?
                } else if type_name.contains("NUMERIC") || type_name.contains("DECIMAL") {
                    // PostgreSQL NUMERIC cannot decode as f64 directly; go
                    // through the text representation.
                    row.try_get::<f64, _>(index)
                        .map(Value::Number)
                        .or_else(|_| {
                            row.try_get::<String, _>(index).map(|s| {
                                s.parse::<f64>()
                                    .map(Value::Number)
                                    .unwrap_or_else(|_| Value::Text(Arc::from(s.as_str())))
                            })
                        })
                        .map_err(|e| decode_error(&name, &type_name, e))?
                } else if type_name.contains("BLOB")
                    || type_name.contains("BYTEA")
                    || type_name.contains("BINARY")
                {
                    row.try_get::<Vec<u8>, _>(index)
                        .map(|v| Value::Binary(Arc::from(v.as_slice())))
                        .map_err(|e| decode_error(&name, &type_name, e))?
                } else if type_name.contains("TIMESTAMP") || type_name == "DATETIME" {
                    row.try_get::<chrono::NaiveDateTime, _>(index)
                        .map(|v| Value::DateTime(Rc::new(v)))
                        .or_else(|_| {
                            row.try_get::<chrono::DateTime<chrono::Utc>, _>(index)
                                .map(|v| Value::DateTime(Rc::new(v.naive_utc())))
                        })
                        .map_err(|e| decode_error(&name, &type_name, e))?
                } else if type_name == "DATE" {
                    row.try_get::<chrono::NaiveDate, _>(index)
                        .map(|v| Value::Date(Rc::new(v)))
                        .map_err(|e| decode_error(&name, &type_name, e))?
                } else if type_name == "TIME" {
                    row.try_get::<chrono::NaiveTime, _>(index)
                        .map(|v| Value::Time(Rc::new(v)))
                        .map_err(|e| decode_error(&name, &type_name, e))?
                } else {
                    // TEXT, VARCHAR, CHAR, and anything unrecognized (e.g.
                    // SQLite expression columns like count(*)): try text,
                    // then numeric decodings.
                    row.try_get::<String, _>(index)
                        .map(|v| Value::Text(Arc::from(v.as_str())))
                        .or_else(|_| $int_fn(row, index))
                        .or_else(|_| row.try_get::<f64, _>(index).map(Value::Number))
                        .or_else(|_| row.try_get::<bool, _>(index).map(Value::Bool))
                        .unwrap_or(Value::Null)
                };

                object.insert(name, value);
            }

            Ok(Value::Object(Rc::new(RefCell::new(object))))
        }
    };
}

fn decode_error(column: &str, type_name: &str, error: sqlx::Error) -> String {
    format!("Failed to decode column '{column}' ({type_name}): {error}")
}

row_to_value!(sqlite_row_to_value, SqliteRow, sqlite_int);
row_to_value!(pg_row_to_value, PgRow, pg_int);
row_to_value!(mysql_row_to_value, MySqlRow, mysql_int);

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unsupported_database_url_does_not_echo_credentials() {
        const SECRET: &str = "WFL_TEST_SECRET_unsupported_password_d5e0ec";
        let url = format!("oracle://wfl:{SECRET}@database.example/app");

        let error = match connect(&url).await {
            Ok(_) => panic!("unsupported database URL unexpectedly connected"),
            Err(error) => error,
        };

        assert!(error.contains("Unsupported database URL scheme"));
        assert!(!error.contains(SECRET));
        assert!(!error.contains(url.as_str()));
    }

    #[test]
    fn whole_numbers_bind_as_integers() {
        assert!(matches!(
            value_to_sql_param(&Value::Number(42.0)),
            Ok(SqlParam::Int(42))
        ));
        assert!(matches!(
            value_to_sql_param(&Value::Number(-7.0)),
            Ok(SqlParam::Int(-7))
        ));
    }

    #[test]
    fn fractional_numbers_bind_as_floats() {
        assert!(matches!(
            value_to_sql_param(&Value::Number(9.5)),
            Ok(SqlParam::Float(f)) if f == 9.5
        ));
    }

    #[test]
    fn whole_numbers_beyond_i64_range_bind_as_floats() {
        // 1e19 > i64::MAX; an unchecked cast would saturate silently.
        assert!(matches!(
            value_to_sql_param(&Value::Number(1e19)),
            Ok(SqlParam::Float(f)) if f == 1e19
        ));
        assert!(matches!(
            value_to_sql_param(&Value::Number(-1e19)),
            Ok(SqlParam::Float(f)) if f == -1e19
        ));
    }

    #[test]
    fn nothing_binds_as_null() {
        assert!(matches!(
            value_to_sql_param(&Value::Null),
            Ok(SqlParam::Null)
        ));
        assert!(matches!(
            value_to_sql_param(&Value::Nothing),
            Ok(SqlParam::Null)
        ));
    }
}
