use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use rusqlite::{params, Connection};
use tokio::sync::Mutex;

use super::{ExecutionRepository, RepositoryResult};
use crate::domain::execution::ExecutionRecord;
use crate::domain::identifiers::ExecutionId;
use crate::error::RepositoryError;

const MIGRATION: &str = "
CREATE TABLE IF NOT EXISTS executions (
    execution_id TEXT PRIMARY KEY,
    payload TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
";

#[derive(Debug)]
pub struct SqliteExecutionRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteExecutionRepository {
    pub fn open(path: impl AsRef<Path>) -> RepositoryResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(MIGRATION)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn open_in_memory() -> RepositoryResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(MIGRATION)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl ExecutionRepository for SqliteExecutionRepository {
    async fn save(&self, record: &ExecutionRecord) -> RepositoryResult<()> {
        let payload = serde_json::to_string(record).map_err(|e| {
            RepositoryError::Serialization(e.to_string())
        })?;
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO executions (execution_id, payload, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(execution_id) DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at",
            params![
                record.internal_execution_id.0,
                payload,
                record.last_observed_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    async fn get(&self, id: &ExecutionId) -> RepositoryResult<Option<ExecutionRecord>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT payload FROM executions WHERE execution_id = ?1",
        )?;
        let mut rows = stmt.query(params![id.0])?;
        if let Some(row) = rows.next()? {
            let payload: String = row.get(0)?;
            let record: ExecutionRecord = serde_json::from_str(&payload).map_err(|e| {
                RepositoryError::Serialization(e.to_string())
            })?;
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    async fn list(&self) -> RepositoryResult<Vec<ExecutionRecord>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT payload FROM executions")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let payload: String = row.get(0)?;
            let record: ExecutionRecord = serde_json::from_str(&payload).map_err(|e| {
                RepositoryError::Serialization(e.to_string())
            })?;
            out.push(record);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::identifiers::TokenId;
    use crate::domain::states::Side;
    use crate::verifier::ExecutionVerifier;
    use rust_decimal::Decimal;

    #[tokio::test]
    async fn sqlite_reload() {
        let repo = SqliteExecutionRepository::open_in_memory().unwrap();
        let verifier = ExecutionVerifier;
        let record = verifier.create_execution(
            TokenId("t".into()),
            Side::Buy,
            Decimal::from(100),
            None,
        );
        repo.save(&record).await.unwrap();
        let loaded = repo.get(&record.internal_execution_id).await.unwrap().unwrap();
        assert_eq!(loaded.internal_execution_id, record.internal_execution_id);
    }
}
