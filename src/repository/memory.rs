use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use super::{ExecutionRepository, RepositoryResult};
use crate::domain::execution::ExecutionRecord;
use crate::domain::identifiers::ExecutionId;
use crate::error::RepositoryError;

#[derive(Debug, Default)]
pub struct InMemoryExecutionRepository {
    inner: Arc<RwLock<HashMap<String, ExecutionRecord>>>,
}

impl InMemoryExecutionRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ExecutionRepository for InMemoryExecutionRepository {
    async fn save(&self, record: &ExecutionRecord) -> RepositoryResult<()> {
        let mut g = self.inner.write().await;
        g.insert(
            record.internal_execution_id.0.clone(),
            record.clone(),
        );
        Ok(())
    }

    async fn get(&self, id: &ExecutionId) -> RepositoryResult<Option<ExecutionRecord>> {
        let g = self.inner.read().await;
        Ok(g.get(&id.0).cloned())
    }

    async fn list(&self) -> RepositoryResult<Vec<ExecutionRecord>> {
        let g = self.inner.read().await;
        Ok(g.values().cloned().collect())
    }
}

impl InMemoryExecutionRepository {
    pub async fn clear(&self) -> RepositoryResult<()> {
        let mut g = self.inner.write().await;
        g.clear();
        Ok(())
    }

    pub async fn len(&self) -> RepositoryResult<usize> {
        let g = self.inner.read().await;
        Ok(g.len())
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
    async fn persist_roundtrip() {
        let repo = InMemoryExecutionRepository::new();
        let verifier = ExecutionVerifier;
        let record = verifier.create_execution(
            TokenId("t".into()),
            Side::Buy,
            Decimal::from(10),
            None,
        );
        repo.save(&record).await.unwrap();
        let loaded = repo.get(&record.internal_execution_id).await.unwrap().unwrap();
        assert_eq!(loaded.requested_quantity, Decimal::from(10));
    }
}
