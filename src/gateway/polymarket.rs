//! Read-oriented Polymarket adapter (feature `polymarket-sdk`).
//!
//! Field mapping (internal ← external) must follow actual SDK types from
//! `polymarket_client_sdk_v2`. This module compiles only with the feature enabled
//! and documents mappings as SDK structs are wired in.
//!
//! | Internal field | SDK source (to wire) |
//! |----------------|----------------------|
//! | `PolymarketOrderId` | CLOB order id string from order responses |
//! | `TradeId` | Trade id from Data API / user trade events |
//! | `matched_quantity` | Sum of trade sizes (Decimal), never from requested size |
//! | `TokenId` | Asset / token id from market metadata |
//! | `TransactionHash` | On-chain tx hash when exposed on matched order / trade payload |
//!
//! No order submission APIs are called from this repository.

use async_trait::async_trait;

use super::{ExecutionSource, PositionSource, TransactionSource};
use crate::domain::events::{ObservedOrder, ObservedTrade, TransactionStatus};
use crate::domain::identifiers::{
    PolymarketOrderId, TokenId, TransactionHash,
};
use crate::error::{VerifierError, VerifierResult};

/// Placeholder adapter shell: construct with SDK clients configured by the host application.
pub struct PolymarketGateway {
    _private: (),
}

impl PolymarketGateway {
    pub fn new_unconfigured() -> Self {
        Self { _private: () }
    }
}

#[async_trait]
impl ExecutionSource for PolymarketGateway {
    async fn get_order(&self, _order_id: &PolymarketOrderId) -> VerifierResult<Option<ObservedOrder>> {
        Err(VerifierError::Gateway(
            "PolymarketGateway: configure SDK client before use (see module docs)".into(),
        ))
    }

    async fn get_trades(&self, _order_id: &PolymarketOrderId) -> VerifierResult<Vec<ObservedTrade>> {
        Err(VerifierError::Gateway(
            "PolymarketGateway: configure SDK client before use (see module docs)".into(),
        ))
    }
}

#[async_trait]
impl TransactionSource for PolymarketGateway {
    async fn get_transaction_status(
        &self,
        _hash: &TransactionHash,
    ) -> VerifierResult<TransactionStatus> {
        Err(VerifierError::Gateway(
            "PolymarketGateway: transaction lookup not configured".into(),
        ))
    }
}

#[async_trait]
impl PositionSource for PolymarketGateway {
    async fn get_position(&self, _token_id: &TokenId) -> VerifierResult<Option<rust_decimal::Decimal>> {
        Err(VerifierError::Gateway(
            "PolymarketGateway: Data API position client not configured".into(),
        ))
    }
}
