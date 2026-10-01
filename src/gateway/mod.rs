use async_trait::async_trait;

use crate::domain::events::{ObservedOrder, ObservedTrade, TransactionStatus};
use crate::domain::identifiers::{PolymarketOrderId, TokenId, TradeId, TransactionHash};
use crate::error::VerifierResult;

pub mod mock;

#[cfg(feature = "polymarket-sdk")]
pub mod polymarket;

#[async_trait]
pub trait ExecutionSource: Send + Sync {
    async fn get_order(&self, order_id: &PolymarketOrderId) -> VerifierResult<Option<ObservedOrder>>;

    async fn get_trades(&self, order_id: &PolymarketOrderId) -> VerifierResult<Vec<ObservedTrade>>;
}

#[async_trait]
pub trait TransactionSource: Send + Sync {
    async fn get_transaction_status(
        &self,
        hash: &TransactionHash,
    ) -> VerifierResult<TransactionStatus>;
}

#[async_trait]
pub trait PositionSource: Send + Sync {
    async fn get_position(&self, token_id: &TokenId) -> VerifierResult<Option<rust_decimal::Decimal>>;
}
