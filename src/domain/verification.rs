use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::identifiers::{ExecutionId, MarketId, TokenId};
use super::states::{PositionCheckResult, VerificationState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub verification_state: VerificationState,
    pub position_check: Option<PositionCheckResult>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionMismatch {
    pub market_id: Option<MarketId>,
    pub token_id: TokenId,
    pub expected_quantity: Decimal,
    pub observed_quantity: Decimal,
    pub difference: Decimal,
    pub execution_ids: Vec<ExecutionId>,
    pub observed_at: DateTime<Utc>,
    pub reason: String,
}

impl PositionMismatch {
    pub fn new(
        token_id: TokenId,
        expected_quantity: Decimal,
        observed_quantity: Decimal,
        execution_ids: Vec<ExecutionId>,
        reason: impl Into<String>,
    ) -> Self {
        let difference = expected_quantity - observed_quantity;
        Self {
            market_id: None,
            token_id,
            expected_quantity,
            observed_quantity,
            difference,
            execution_ids,
            observed_at: Utc::now(),
            reason: reason.into(),
        }
    }
}
