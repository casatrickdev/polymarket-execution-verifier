use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::identifiers::{
    EventId, ExecutionId, PolymarketOrderId, TokenId, TradeId, TransactionHash,
};
use super::states::ExecutionState;

/// Normalized observations from gateway/replay (not raw SDK payloads).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionEvent {
    ExecutionCreated {
        execution_id: ExecutionId,
        event_id: EventId,
        observed_at: DateTime<Utc>,
    },
    OrderSubmitted {
        execution_id: ExecutionId,
        event_id: EventId,
        order_id: PolymarketOrderId,
        observed_at: DateTime<Utc>,
    },
    OrderAccepted {
        execution_id: ExecutionId,
        event_id: EventId,
        observed_at: DateTime<Utc>,
    },
    OrderRejected {
        execution_id: ExecutionId,
        event_id: EventId,
        reason: String,
        observed_at: DateTime<Utc>,
    },
    OrderCancelled {
        execution_id: ExecutionId,
        event_id: EventId,
        observed_at: DateTime<Utc>,
    },
    OrderMatched {
        execution_id: ExecutionId,
        event_id: EventId,
        observed_at: DateTime<Utc>,
    },
    FillObserved {
        execution_id: ExecutionId,
        event_id: EventId,
        trade_id: TradeId,
        fill_quantity: Decimal,
        fill_price: Option<Decimal>,
        observed_at: DateTime<Utc>,
    },
    TransactionPending {
        execution_id: ExecutionId,
        event_id: EventId,
        observed_at: DateTime<Utc>,
    },
    TransactionObserved {
        execution_id: ExecutionId,
        event_id: EventId,
        transaction_hash: TransactionHash,
        observed_at: DateTime<Utc>,
    },
    TransactionConfirmed {
        execution_id: ExecutionId,
        event_id: EventId,
        observed_at: DateTime<Utc>,
    },
    SettlementObserved {
        execution_id: ExecutionId,
        event_id: EventId,
        observed_at: DateTime<Utc>,
    },
    PositionVerified {
        execution_id: ExecutionId,
        event_id: EventId,
        observed_at: DateTime<Utc>,
    },
    PositionMismatchObserved {
        execution_id: ExecutionId,
        event_id: EventId,
        reason: String,
        observed_at: DateTime<Utc>,
    },
    EventGap {
        execution_id: ExecutionId,
        event_id: EventId,
        reason: String,
        observed_at: DateTime<Utc>,
    },
    ReconciliationStarted {
        execution_id: ExecutionId,
        event_id: EventId,
        observed_at: DateTime<Utc>,
    },
    ReconciliationCompleted {
        execution_id: ExecutionId,
        event_id: EventId,
        observed_at: DateTime<Utc>,
    },
    RecoveryStarted {
        execution_id: ExecutionId,
        event_id: EventId,
        observed_at: DateTime<Utc>,
    },
    RecoveryCompleted {
        execution_id: ExecutionId,
        event_id: EventId,
        observed_at: DateTime<Utc>,
    },
    MarkUnknown {
        execution_id: ExecutionId,
        event_id: EventId,
        reason: String,
        observed_at: DateTime<Utc>,
    },
    MarkRetrying {
        execution_id: ExecutionId,
        event_id: EventId,
        reason: String,
        observed_at: DateTime<Utc>,
    },
}

impl ExecutionEvent {
    pub fn execution_id(&self) -> &ExecutionId {
        match self {
            Self::ExecutionCreated { execution_id, .. }
            | Self::OrderSubmitted { execution_id, .. }
            | Self::OrderAccepted { execution_id, .. }
            | Self::OrderRejected { execution_id, .. }
            | Self::OrderCancelled { execution_id, .. }
            | Self::OrderMatched { execution_id, .. }
            | Self::FillObserved { execution_id, .. }
            | Self::TransactionPending { execution_id, .. }
            | Self::TransactionObserved { execution_id, .. }
            | Self::TransactionConfirmed { execution_id, .. }
            | Self::SettlementObserved { execution_id, .. }
            | Self::PositionVerified { execution_id, .. }
            | Self::PositionMismatchObserved { execution_id, .. }
            | Self::EventGap { execution_id, .. }
            | Self::ReconciliationStarted { execution_id, .. }
            | Self::ReconciliationCompleted { execution_id, .. }
            | Self::RecoveryStarted { execution_id, .. }
            | Self::RecoveryCompleted { execution_id, .. }
            | Self::MarkUnknown { execution_id, .. }
            | Self::MarkRetrying { execution_id, .. } => execution_id,
        }
    }

    pub fn event_id(&self) -> &EventId {
        match self {
            Self::ExecutionCreated { event_id, .. }
            | Self::OrderSubmitted { event_id, .. }
            | Self::OrderAccepted { event_id, .. }
            | Self::OrderRejected { event_id, .. }
            | Self::OrderCancelled { event_id, .. }
            | Self::OrderMatched { event_id, .. }
            | Self::FillObserved { event_id, .. }
            | Self::TransactionPending { event_id, .. }
            | Self::TransactionObserved { event_id, .. }
            | Self::TransactionConfirmed { event_id, .. }
            | Self::SettlementObserved { event_id, .. }
            | Self::PositionVerified { event_id, .. }
            | Self::PositionMismatchObserved { event_id, .. }
            | Self::EventGap { event_id, .. }
            | Self::ReconciliationStarted { event_id, .. }
            | Self::ReconciliationCompleted { event_id, .. }
            | Self::RecoveryStarted { event_id, .. }
            | Self::RecoveryCompleted { event_id, .. }
            | Self::MarkUnknown { event_id, .. }
            | Self::MarkRetrying { event_id, .. } => event_id,
        }
    }

    pub fn target_state_hint(&self) -> Option<ExecutionState> {
        match self {
            Self::OrderSubmitted { .. } => Some(ExecutionState::Submitted),
            Self::OrderAccepted { .. } => Some(ExecutionState::Accepted),
            Self::OrderRejected { .. } => Some(ExecutionState::Rejected),
            Self::OrderCancelled { .. } => Some(ExecutionState::Cancelled),
            Self::OrderMatched { .. } => Some(ExecutionState::Matched),
            Self::FillObserved { .. } => None,
            Self::TransactionPending { .. } => Some(ExecutionState::TxPending),
            Self::TransactionConfirmed { .. } => Some(ExecutionState::Confirmed),
            Self::SettlementObserved { .. } => Some(ExecutionState::Settled),
            Self::PositionVerified { .. } => Some(ExecutionState::PositionVerified),
            Self::PositionMismatchObserved { .. } => Some(ExecutionState::Inconsistent),
            Self::EventGap { .. } | Self::MarkUnknown { .. } => Some(ExecutionState::Unknown),
            Self::MarkRetrying { .. } => Some(ExecutionState::Retrying),
            Self::ReconciliationStarted { .. } | Self::RecoveryStarted { .. } => {
                Some(ExecutionState::Reconciling)
            }
            _ => None,
        }
    }
}

/// Structured telemetry labels for tracing subscribers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryEvent {
    ExecutionCreated,
    OrderObserved,
    FillObserved,
    PartialFill,
    TxPending,
    TxConfirmed,
    SettlementObserved,
    PositionVerified,
    PositionMismatch,
    ExecutionUnknown,
    ReconciliationStarted,
    ReconciliationCompleted,
    RecoveryStarted,
    RecoveryCompleted,
}

impl TelemetryEvent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExecutionCreated => "EXECUTION_CREATED",
            Self::OrderObserved => "ORDER_OBSERVED",
            Self::FillObserved => "FILL_OBSERVED",
            Self::PartialFill => "PARTIAL_FILL",
            Self::TxPending => "TX_PENDING",
            Self::TxConfirmed => "TX_CONFIRMED",
            Self::SettlementObserved => "SETTLEMENT_OBSERVED",
            Self::PositionVerified => "POSITION_VERIFIED",
            Self::PositionMismatch => "POSITION_MISMATCH",
            Self::ExecutionUnknown => "EXECUTION_UNKNOWN",
            Self::ReconciliationStarted => "RECONCILIATION_STARTED",
            Self::ReconciliationCompleted => "RECONCILIATION_COMPLETED",
            Self::RecoveryStarted => "RECOVERY_STARTED",
            Self::RecoveryCompleted => "RECOVERY_COMPLETED",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedOrder {
    pub order_id: PolymarketOrderId,
    pub token_id: TokenId,
    pub matched_quantity: Decimal,
    pub state_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedTrade {
    pub trade_id: TradeId,
    pub quantity: Decimal,
    pub price: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
    Unknown,
}
