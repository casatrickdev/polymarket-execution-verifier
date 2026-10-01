use chrono::Utc;
use rust_decimal::Decimal;

use crate::domain::events::{ExecutionEvent, TransactionStatus};
use crate::domain::execution::ExecutionRecord;
use crate::domain::identifiers::EventId;
use crate::domain::states::ExecutionState;
use crate::error::{VerifierError, VerifierResult};
use crate::gateway::{ExecutionSource, PositionSource, TransactionSource};
use crate::reconciliation::position::evaluate_position;
use crate::verifier::engine::ExecutionVerifier;

#[derive(Debug)]
pub struct ReconciliationExecutor {
    verifier: ExecutionVerifier,
}

impl Default for ReconciliationExecutor {
    fn default() -> Self {
        Self {
            verifier: ExecutionVerifier,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecoveryResult {
    pub execution: ExecutionRecord,
    pub recovered: bool,
    pub notes: Vec<String>,
}

impl ReconciliationExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn recover_execution<E, T, P>(
        &self,
        mut record: ExecutionRecord,
        execution_source: &E,
        transaction_source: &T,
        position_source: &P,
        position_baseline: Decimal,
    ) -> VerifierResult<RecoveryResult>
    where
        E: ExecutionSource + ?Sized,
        T: TransactionSource + ?Sized,
        P: PositionSource + ?Sized,
    {
        let mut notes = Vec::new();
        let start = ExecutionEvent::RecoveryStarted {
            execution_id: record.internal_execution_id.clone(),
            event_id: EventId(format!("recovery-start-{}", Utc::now().timestamp_millis())),
            observed_at: Utc::now(),
        };
        record = self.verifier.apply_event(record, start)?.execution;

        let order_id = record
            .polymarket_order_id
            .clone()
            .ok_or_else(|| VerifierError::Reconciliation("missing order id for recovery".into()))?;

        if let Some(order) = execution_source.get_order(&order_id).await? {
            notes.push(format!("observed order state: {}", order.state_label));
        } else {
            notes.push("order not found at source".into());
            let unknown = ExecutionEvent::MarkUnknown {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId(format!("recovery-unknown-{}", Utc::now().timestamp_millis())),
                reason: "authoritative order missing".into(),
                observed_at: Utc::now(),
            };
            record = self.verifier.apply_event(record, unknown)?.execution;
            return Ok(RecoveryResult {
                execution: record,
                recovered: false,
                notes,
            });
        }

        let trades = execution_source.get_trades(&order_id).await?;
        for (i, trade) in trades.iter().enumerate() {
            let ev = ExecutionEvent::FillObserved {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId(format!("recovery-fill-{i}-{}", trade.trade_id.0)),
                trade_id: trade.trade_id.clone(),
                fill_quantity: trade.quantity,
                fill_price: trade.price,
                observed_at: Utc::now(),
            };
            record = self.verifier.apply_event(record, ev)?.execution;
        }

        if record.current_execution_state == ExecutionState::Filled
            || record.current_execution_state == ExecutionState::Partial
        {
            if record.transaction_hash.is_none() {
                let pending = ExecutionEvent::TransactionPending {
                    execution_id: record.internal_execution_id.clone(),
                    event_id: EventId(format!("recovery-tx-pending-{}", Utc::now().timestamp_millis())),
                    observed_at: Utc::now(),
                };
                record = self.verifier.apply_event(record, pending)?.execution;
            } else if let Some(hash) = record.transaction_hash.clone() {
                match transaction_source.get_transaction_status(&hash).await? {
                    TransactionStatus::Confirmed => {
                        let ev = ExecutionEvent::TransactionConfirmed {
                            execution_id: record.internal_execution_id.clone(),
                            event_id: EventId(format!("recovery-tx-conf-{}", hash.0)),
                            observed_at: Utc::now(),
                        };
                        record = self.verifier.apply_event(record, ev)?.execution;
                    }
                    TransactionStatus::Pending => {
                        notes.push("transaction still pending".into());
                    }
                    TransactionStatus::Unknown => {
                        let ev = ExecutionEvent::MarkUnknown {
                            execution_id: record.internal_execution_id.clone(),
                            event_id: EventId(format!("recovery-tx-unknown-{}", hash.0)),
                            reason: "transaction status unknown".into(),
                            observed_at: Utc::now(),
                        };
                        record = self.verifier.apply_event(record, ev)?.execution;
                    }
                    TransactionStatus::Failed => {
                        let ev = ExecutionEvent::MarkUnknown {
                            execution_id: record.internal_execution_id.clone(),
                            event_id: EventId(format!("recovery-tx-failed-{}", hash.0)),
                            reason: "transaction failed".into(),
                            observed_at: Utc::now(),
                        };
                        record = self.verifier.apply_event(record, ev)?.execution;
                    }
                }
            }
        }

        if record.current_execution_state == ExecutionState::Confirmed {
            let settle = ExecutionEvent::SettlementObserved {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId(format!("recovery-settle-{}", Utc::now().timestamp_millis())),
                observed_at: Utc::now(),
            };
            record = self.verifier.apply_event(record, settle)?.execution;
        }

        let observed_position = position_source.get_position(&record.token_id).await?;
        match evaluate_position(&record, position_baseline, observed_position) {
            Ok(_) => {
                if record.current_execution_state == ExecutionState::Settled {
                    let ev = ExecutionEvent::PositionVerified {
                        execution_id: record.internal_execution_id.clone(),
                        event_id: EventId(format!("recovery-pos-{}", Utc::now().timestamp_millis())),
                        observed_at: Utc::now(),
                    };
                    record = self.verifier.apply_event(record, ev)?.execution;
                }
            }
            Err(mismatch) => {
                let ev = ExecutionEvent::PositionMismatchObserved {
                    execution_id: record.internal_execution_id.clone(),
                    event_id: EventId(format!("recovery-mismatch-{}", Utc::now().timestamp_millis())),
                    reason: mismatch.reason,
                    observed_at: Utc::now(),
                };
                record = self.verifier.apply_event(record, ev)?.execution;
            }
        }

        let complete = ExecutionEvent::RecoveryCompleted {
            execution_id: record.internal_execution_id.clone(),
            event_id: EventId(format!("recovery-done-{}", Utc::now().timestamp_millis())),
            observed_at: Utc::now(),
        };
        record = self.verifier.apply_event(record, complete)?.execution;

        let recovered = record.current_execution_state.is_verified_success();
        Ok(RecoveryResult {
            execution: record,
            recovered,
            notes,
        })
    }
}
