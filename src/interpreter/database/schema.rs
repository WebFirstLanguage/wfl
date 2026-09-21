//! SQLite schema transactions own their connection from setup through cleanup.
//!
//! The foreign-key switch must happen before BEGIN. A checked commit validates
//! all references before making the schema and its migration ledger durable.
use super::SQLITE_LOCK_WAIT;
use sqlx::pool::PoolConnection;
use sqlx::{Row, Sqlite, SqliteConnection, SqlitePool};

pub struct SchemaTransaction {
    connection: Option<PoolConnection<Sqlite>>,
}

/// If cleanup itself is cancelled, never return an unconfigured connection.
struct CleanupConnection(Option<PoolConnection<Sqlite>>);

impl Drop for CleanupConnection {
    fn drop(&mut self) {
        if let Some(connection) = self.0.as_mut() {
            connection.close_on_drop();
        }
    }
}

impl SchemaTransaction {
    pub async fn begin(pool: &SqlitePool) -> Result<Self, String> {
        let begin = async {
            let connection = pool.acquire().await?;
            // Install the cancellation guard before touching connection state.
            let mut transaction = Self {
                connection: Some(connection),
            };
            sqlx::query("PRAGMA busy_timeout = 5000")
                .execute(transaction.connection())
                .await?;
            sqlx::query("PRAGMA foreign_keys = OFF")
                .execute(transaction.connection())
                .await?;
            sqlx::query("BEGIN IMMEDIATE")
                .execute(transaction.connection())
                .await?;
            Ok::<_, sqlx::Error>(transaction)
        };
        match tokio::time::timeout(SQLITE_LOCK_WAIT, begin).await {
            Ok(Ok(transaction)) => Ok(transaction),
            Ok(Err(error)) => Err(format!(
                "Cannot start schema transaction: {error}. Another writer may hold the database; finish that operation and retry the migration. Lock acquisition waits at most 5 seconds."
            )),
            Err(_) => Err(
                "Cannot start schema transaction within 5 seconds. Another writer or transaction may hold the database. Finish that operation, then retry the migration; its body has not run."
                    .to_string(),
            ),
        }
    }

    pub fn connection(&mut self) -> &mut SqliteConnection {
        self.connection
            .as_mut()
            .expect("schema transaction owns its connection until completion")
    }

    pub async fn commit(mut self) -> Result<(), String> {
        let validation = sqlx::query("PRAGMA foreign_key_check")
            .fetch_optional(self.connection())
            .await;
        let failure = match validation {
            Ok(None) => None,
            Ok(Some(row)) => {
                let table: String = row.try_get("table").unwrap_or_default();
                Some(format!(
                    "Schema transaction would leave a foreign-key violation in table '{table}'. Restore the referenced rows or repair the migration before retrying; the transaction was rolled back."
                ))
            }
            Err(error) => Some(format!(
                "Cannot validate schema transaction foreign keys: {error}. Repair the schema before retrying; the transaction was rolled back."
            )),
        };
        if let Some(failure) = failure {
            return match self.rollback().await {
                Ok(()) => Err(failure),
                Err(cleanup) => Err(format!("{failure} Cleanup also failed: {cleanup}")),
            };
        }
        if let Err(error) = sqlx::query("COMMIT").execute(self.connection()).await {
            return match self.rollback().await {
                Ok(()) => Err(format!("Failed to commit schema transaction: {error}")),
                Err(cleanup) => Err(format!(
                    "Failed to commit schema transaction: {error}. Cleanup also failed: {cleanup}"
                )),
            };
        }
        restore_connection(self.connection.take().expect("owned connection")).await
    }

    pub async fn rollback(mut self) -> Result<(), String> {
        if let Err(error) = sqlx::query("ROLLBACK").execute(self.connection()).await {
            // Drop closes it if rollback fails; it must never reenter the pool.
            let mut connection = self.connection.take().expect("owned connection");
            connection.close_on_drop();
            return Err(format!(
                "Failed to roll back schema transaction: {error}. The connection was discarded."
            ));
        }
        restore_connection(self.connection.take().expect("owned connection")).await
    }
}

impl Drop for SchemaTransaction {
    fn drop(&mut self) {
        let Some(mut connection) = self.connection.take() else {
            return;
        };
        // The worker processes rollback after any cancelled BEGIN/COMMIT await.
        // The guard closes the connection if this cleanup task cannot finish.
        match tokio::runtime::Handle::try_current() {
            Ok(runtime) => {
                let cleanup = CleanupConnection(Some(connection));
                runtime.spawn(async move {
                    let _ = restore_guarded_connection(cleanup, true).await;
                });
            }
            Err(_) => connection.close_on_drop(),
        }
    }
}

async fn restore_connection(connection: PoolConnection<Sqlite>) -> Result<(), String> {
    restore_guarded_connection(CleanupConnection(Some(connection)), false).await
}

async fn restore_guarded_connection(
    mut cleanup: CleanupConnection,
    rollback_first: bool,
) -> Result<(), String> {
    let connection = cleanup.0.as_mut().expect("cleanup owns connection");
    let restore = async {
        if rollback_first {
            // BEGIN may not have completed, or COMMIT may already have completed,
            // when the future was cancelled. A no-transaction error is harmless.
            // Verification below refuses to pool a still-open transaction.
            let _ = sqlx::query("ROLLBACK").execute(&mut **connection).await;
        }
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&mut **connection)
            .await?;
        let row = sqlx::query("PRAGMA foreign_keys")
            .fetch_one(&mut **connection)
            .await?;
        row.try_get::<i64, _>(0)
    };
    match restore.await {
        Ok(1) => {
            // Only a verified, restored connection can return to its pool.
            drop(cleanup.0.take());
            Ok(())
        }
        Ok(_) => Err("Foreign-key enforcement could not be restored after the schema transaction. The connection was discarded; close this database handle and reopen it before retrying.".to_string()),
        Err(error) => Err(format!("Failed to restore foreign-key enforcement after schema transaction: {error}. The connection was discarded; close this database handle and reopen it before retrying.")),
    }
}
