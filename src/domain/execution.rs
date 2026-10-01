use std::collections::HashSet;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::identifiers::{
    ConditionId, EventId, ExecutionId, MarketId, PolymarketOrderId, StrategyDecisionId, TokenId,
    TradeId, TransactionHash,
};
use super::states::{ExecutionState, Side, VerificationState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub internal_execution_id: ExecutionId,
    pub strategy_decision_id: Option<StrategyDecisionId>,
    pub polymarket_order_id: Option<PolymarketOrderId>,
    pub trade_ids: Vec<TradeId>,
    pub market_id: Option<MarketId>,
    pub condition_id: Option<ConditionId>,
    pub token_id: TokenId,
    pub side: Side,
    pub requested_quantity: Decimal,
    pub matched_quantity: Decimal,
    pub remaining_quantity: Decimal,
    pub average_price: Option<Decimal>,
    pub transaction_hash: Option<TransactionHash>,
    pub order_created_at: Option<DateTime<Utc>>,
    pub fill_observed_at: Option<DateTime<Utc>>,
    pub transaction_observed_at: Option<DateTime<Utc>>,
    pub confirmation_observed_at: Option<DateTime<Utc>>,
    pub settlement_observed_at: Option<DateTime<Utc>>,
    pub position_verified_at: Option<DateTime<Utc>>,
    pub current_execution_state: ExecutionState,
    pub verification_state: VerificationState,
    pub last_observed_at: DateTime<Utc>,
    pub last_event_id: Option<EventId>,
    pub verification_attempts: u32,
    pub last_error: Option<String>,
    pub uncertainty_reason: Option<String>,
    /// Idempotency: processed fill/trade/event identities.
    #[serde(default)]
    pub processed_event_ids: HashSet<EventId>,
    #[serde(default)]
    pub processed_trade_ids: HashSet<TradeId>,
}

impl ExecutionRecord {
    pub fn new_intended(
        token_id: TokenId,
        side: Side,
        requested_quantity: Decimal,
        strategy_decision_id: Option<StrategyDecisionId>,
    ) -> Self {
        let now = Utc::now();
        Self {
            internal_execution_id: ExecutionId::new(),
            strategy_decision_id,
            polymarket_order_id: None,
            trade_ids: Vec::new(),
            market_id: None,
            condition_id: None,
            token_id,
            side,
            requested_quantity,
            matched_quantity: Decimal::ZERO,
            remaining_quantity: requested_quantity,
            average_price: None,
            transaction_hash: None,
            order_created_at: Some(now),
            fill_observed_at: None,
            transaction_observed_at: None,
            confirmation_observed_at: None,
            settlement_observed_at: None,
            position_verified_at: None,
            current_execution_state: ExecutionState::Intended,
            verification_state: VerificationState::Pending,
            last_observed_at: now,
            last_event_id: None,
            verification_attempts: 0,
            last_error: None,
            uncertainty_reason: None,
            processed_event_ids: HashSet::new(),
            processed_trade_ids: HashSet::new(),
        }
    }

    pub fn recompute_remaining(&mut self) {
        self.remaining_quantity = self.requested_quantity - self.matched_quantity;
        if self.remaining_quantity < Decimal::ZERO {
            self.remaining_quantity = Decimal::ZERO;
        }
    }

    pub fn refresh_verification_state(&mut self) {
        self.verification_state = match self.current_execution_state {
            ExecutionState::PositionVerified => VerificationState::Verified,
            ExecutionState::Inconsistent => VerificationState::Inconsistent,
            ExecutionState::Rejected | ExecutionState::Cancelled | ExecutionState::Failed => {
                VerificationState::Failed
            }
            ExecutionState::Unknown => VerificationState::Unknown,
            ExecutionState::Retrying => VerificationState::Retrying,
            ExecutionState::Reconciling => VerificationState::InProgress,
            ExecutionState::Intended | ExecutionState::Submitted => VerificationState::Pending,
            ExecutionState::Accepted
            | ExecutionState::Matched
            | ExecutionState::Partial
            | ExecutionState::Filled
            | ExecutionState::TxPending
            | ExecutionState::Confirmed
            | ExecutionState::Settled => VerificationState::InProgress,
        };
    }
}
