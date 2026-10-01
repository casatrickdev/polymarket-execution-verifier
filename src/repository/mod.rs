use async_trait::async_trait;

use crate::domain::execution::ExecutionRecord;
use crate::domain::identifiers::ExecutionId;
use crate::error::RepositoryError;

pub mod memory;
pub mod sqlite;

pub type RepositoryResult<T> = Result<T, RepositoryError>;

#[async_trait]
pub trait ExecutionRepository: Send + Sync {
    async fn save(&self, record: &ExecutionRecord) -> RepositoryResult<()>;

    async fn get(&self, id: &ExecutionId) -> RepositoryResult<Option<ExecutionRecord>>;

    async fn list(&self) -> RepositoryResult<Vec<ExecutionRecord>>;
}
