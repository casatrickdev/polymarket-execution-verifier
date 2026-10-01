use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use rust_decimal::Decimal;
use tokio::sync::RwLock;

use super::{ExecutionSource, PositionSource, TransactionSource};
use crate::domain::events::{ObservedOrder, ObservedTrade, TransactionStatus};
use crate::domain::identifiers::{
    PolymarketOrderId, TokenId, TradeId, TransactionHash,
};
use crate::error::{VerifierError, VerifierResult};

#[derive(Debug, Clone, Default)]
pub struct MockGateway {
    inner: Arc<RwLock<MockState>>,
}

#[derive(Debug, Default)]
struct MockState {
    orders: HashMap<String, ObservedOrder>,
    trades: HashMap<String, Vec<ObservedTrade>>,
    transactions: HashMap<String, TransactionStatus>,
    positions: HashMap<String, Decimal>,
}

impl MockGateway {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn set_order(&self, order: ObservedOrder) {
        let mut g = self.inner.write().await;
        g.orders.insert(order.order_id.0.clone(), order);
    }

    pub async fn set_trades(&self, order_id: &PolymarketOrderId, trades: Vec<ObservedTrade>) {
        let mut g = self.inner.write().await;
        g.trades.insert(order_id.0.clone(), trades);
    }

    pub async fn set_transaction_status(&self, hash: &TransactionHash, status: TransactionStatus) {
        let mut g = self.inner.write().await;
        g.transactions.insert(hash.0.clone(), status);
    }

    pub async fn set_position(&self, token_id: &TokenId, quantity: Decimal) {
        let mut g = self.inner.write().await;
        g.positions.insert(token_id.0.clone(), quantity);
    }
}

#[async_trait]
impl ExecutionSource for MockGateway {
    async fn get_order(&self, order_id: &PolymarketOrderId) -> VerifierResult<Option<ObservedOrder>> {
        let g = self.inner.read().await;
        Ok(g.orders.get(&order_id.0).cloned())
    }

    async fn get_trades(&self, order_id: &PolymarketOrderId) -> VerifierResult<Vec<ObservedTrade>> {
        let g = self.inner.read().await;
        Ok(g.trades.get(&order_id.0).cloned().unwrap_or_default())
    }
}

#[async_trait]
impl TransactionSource for MockGateway {
    async fn get_transaction_status(
        &self,
        hash: &TransactionHash,
    ) -> VerifierResult<TransactionStatus> {
        let g = self.inner.read().await;
        Ok(g.transactions
            .get(&hash.0)
            .cloned()
            .unwrap_or(TransactionStatus::Unknown))
    }
}

#[async_trait]
impl PositionSource for MockGateway {
    async fn get_position(&self, token_id: &TokenId) -> VerifierResult<Option<Decimal>> {
        let g = self.inner.read().await;
        Ok(g.positions.get(&token_id.0).copied())
    }
}

/// Deterministic scripted events for demos/tests.
pub fn scenario_full_fill_trades(order_id: &str) -> Vec<ObservedTrade> {
    vec![ObservedTrade {
        trade_id: TradeId(format!("{order_id}-t1")),
        quantity: Decimal::from(100),
        price: Some(Decimal::new(50, 2)),
    }]
}

pub fn scenario_partial_trades(order_id: &str, qty: i64) -> Vec<ObservedTrade> {
    vec![ObservedTrade {
        trade_id: TradeId(format!("{order_id}-t1")),
        quantity: Decimal::from(qty),
        price: Some(Decimal::new(50, 2)),
    }]
}

pub fn not_found_err(msg: impl Into<String>) -> VerifierError {
    VerifierError::Gateway(msg.into())
}
