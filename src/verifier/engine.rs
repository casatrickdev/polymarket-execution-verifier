use chrono::Utc;
use rust_decimal::Decimal;
use tracing::{info, warn};

use crate::domain::events::{ExecutionEvent, TelemetryEvent};
use crate::domain::execution::ExecutionRecord;
use crate::domain::identifiers::{EventId, ExecutionId, PolymarketOrderId};
use crate::domain::states::{ExecutionState, Side};
use crate::domain::verification::VerificationResult;
use crate::error::{VerifierError, VerifierResult};
use crate::telemetry;

use super::policies::{fill_derived_state, position_check_result};
use super::transitions::validate_transition;

#[derive(Debug, Default)]
pub struct ExecutionVerifier;

#[derive(Debug, Clone)]
pub struct ApplyEventResult {
    pub execution: ExecutionRecord,
    pub state_changed: bool,
    pub event_accepted: bool,
}

impl ExecutionVerifier {
    pub fn create_execution(
        &self,
        token_id: crate::domain::identifiers::TokenId,
        side: Side,
        requested_quantity: Decimal,
        strategy_decision_id: Option<crate::domain::identifiers::StrategyDecisionId>,
    ) -> ExecutionRecord {
        let record = ExecutionRecord::new_intended(
            token_id,
            side,
            requested_quantity,
            strategy_decision_id,
        );
        telemetry::log_transition(
            TelemetryEvent::ExecutionCreated,
            &record.internal_execution_id,
            None,
            ExecutionState::Intended,
            ExecutionState::Intended,
            "execution created",
        );
        record
    }

    pub fn apply_event(
        &self,
        mut record: ExecutionRecord,
        event: ExecutionEvent,
    ) -> VerifierResult<ApplyEventResult> {
        if record.processed_event_ids.contains(event.event_id()) {
            return Ok(ApplyEventResult {
                execution: record,
                state_changed: false,
                event_accepted: false,
            });
        }

        let from_state = record.current_execution_state;
        let mut state_changed = false;

        match &event {
            ExecutionEvent::OrderSubmitted {
                order_id,
                observed_at,
                ..
            } => {
                self.transition(&mut record, ExecutionState::Submitted)?;
                record.polymarket_order_id = Some(order_id.clone());
                record.order_created_at = Some(*observed_at);
                state_changed = true;
                telemetry::log_transition(
                    TelemetryEvent::OrderObserved,
                    &record.internal_execution_id,
                    record.polymarket_order_id.as_ref(),
                    from_state,
                    record.current_execution_state,
                    "order submitted",
                );
            }
            ExecutionEvent::OrderAccepted { observed_at, .. } => {
                self.transition(&mut record, ExecutionState::Accepted)?;
                record.last_observed_at = *observed_at;
                state_changed = true;
            }
            ExecutionEvent::OrderRejected { reason, .. } => {
                self.transition(&mut record, ExecutionState::Rejected)?;
                record.last_error = Some(reason.clone());
                state_changed = true;
            }
            ExecutionEvent::OrderCancelled { .. } => {
                self.transition(&mut record, ExecutionState::Cancelled)?;
                state_changed = true;
            }
            ExecutionEvent::OrderMatched { .. } => {
                self.transition(&mut record, ExecutionState::Matched)?;
                state_changed = true;
            }
            ExecutionEvent::FillObserved {
                trade_id,
                fill_quantity,
                fill_price,
                observed_at,
                ..
            } => {
                if record.processed_trade_ids.contains(trade_id) {
                    record.processed_event_ids.insert(event.event_id().clone());
                    return Ok(ApplyEventResult {
                        execution: record,
                        state_changed: false,
                        event_accepted: false,
                    });
                }
                record.processed_trade_ids.insert(trade_id.clone());
                record.trade_ids.push(trade_id.clone());
                record.matched_quantity += *fill_quantity;
                record.recompute_remaining();
                if let (Some(avg), Some(price)) = (record.average_price, fill_price) {
                    let prev_matched = record.matched_quantity - *fill_quantity;
                    let total = prev_matched + *fill_quantity;
                    if total > Decimal::ZERO {
                        record.average_price = Some(
                            (avg * prev_matched + price * *fill_quantity) / total,
                        );
                    }
                } else if fill_price.is_some() {
                    record.average_price = *fill_price;
                }
                record.fill_observed_at = Some(*observed_at);
                let next = fill_derived_state(&record);
                self.transition(&mut record, next)?;
                state_changed = true;
                let tel = if next == ExecutionState::Partial {
                    TelemetryEvent::PartialFill
                } else {
                    TelemetryEvent::FillObserved
                };
                telemetry::log_transition(
                    tel,
                    &record.internal_execution_id,
                    record.polymarket_order_id.as_ref(),
                    from_state,
                    record.current_execution_state,
                    "fill observed",
                );
            }
            ExecutionEvent::TransactionPending { observed_at, .. } => {
                self.transition(&mut record, ExecutionState::TxPending)?;
                record.transaction_observed_at = Some(*observed_at);
                state_changed = true;
                telemetry::log_transition(
                    TelemetryEvent::TxPending,
                    &record.internal_execution_id,
                    record.polymarket_order_id.as_ref(),
                    from_state,
                    record.current_execution_state,
                    "transaction pending",
                );
            }
            ExecutionEvent::TransactionObserved {
                transaction_hash,
                observed_at,
                ..
            } => {
                record.transaction_hash = Some(transaction_hash.clone());
                record.transaction_observed_at = Some(*observed_at);
                if matches!(
                    record.current_execution_state,
                    ExecutionState::Filled | ExecutionState::Partial
                ) {
                    self.transition(&mut record, ExecutionState::TxPending)?;
                    state_changed = true;
                }
            }
            ExecutionEvent::TransactionConfirmed { observed_at, .. } => {
                self.transition(&mut record, ExecutionState::Confirmed)?;
                record.confirmation_observed_at = Some(*observed_at);
                state_changed = true;
                telemetry::log_transition(
                    TelemetryEvent::TxConfirmed,
                    &record.internal_execution_id,
                    record.polymarket_order_id.as_ref(),
                    from_state,
                    record.current_execution_state,
                    "transaction confirmed",
                );
            }
            ExecutionEvent::SettlementObserved { observed_at, .. } => {
                self.transition(&mut record, ExecutionState::Settled)?;
                record.settlement_observed_at = Some(*observed_at);
                state_changed = true;
                telemetry::log_transition(
                    TelemetryEvent::SettlementObserved,
                    &record.internal_execution_id,
                    record.polymarket_order_id.as_ref(),
                    from_state,
                    record.current_execution_state,
                    "settlement observed",
                );
            }
            ExecutionEvent::PositionVerified { observed_at, .. } => {
                self.transition(&mut record, ExecutionState::PositionVerified)?;
                record.position_verified_at = Some(*observed_at);
                state_changed = true;
                telemetry::log_transition(
                    TelemetryEvent::PositionVerified,
                    &record.internal_execution_id,
                    record.polymarket_order_id.as_ref(),
                    from_state,
                    record.current_execution_state,
                    "position verified",
                );
            }
            ExecutionEvent::PositionMismatchObserved { reason, .. } => {
                self.transition(&mut record, ExecutionState::Inconsistent)?;
                record.uncertainty_reason = Some(reason.clone());
                state_changed = true;
                telemetry::log_transition(
                    TelemetryEvent::PositionMismatch,
                    &record.internal_execution_id,
                    record.polymarket_order_id.as_ref(),
                    from_state,
                    record.current_execution_state,
                    reason,
                );
            }
            ExecutionEvent::EventGap { reason, .. } | ExecutionEvent::MarkUnknown { reason, .. } => {
                self.transition(&mut record, ExecutionState::Unknown)?;
                record.uncertainty_reason = Some(reason.clone());
                state_changed = true;
                telemetry::log_transition(
                    TelemetryEvent::ExecutionUnknown,
                    &record.internal_execution_id,
                    record.polymarket_order_id.as_ref(),
                    from_state,
                    record.current_execution_state,
                    reason,
                );
            }
            ExecutionEvent::MarkRetrying { reason, .. } => {
                self.transition(&mut record, ExecutionState::Retrying)?;
                record.uncertainty_reason = Some(reason.clone());
                state_changed = true;
            }
            ExecutionEvent::ReconciliationStarted { .. } | ExecutionEvent::RecoveryStarted { .. } => {
                self.transition(&mut record, ExecutionState::Reconciling)?;
                record.verification_attempts += 1;
                state_changed = true;
                let tel = if matches!(event, ExecutionEvent::RecoveryStarted { .. }) {
                    TelemetryEvent::RecoveryStarted
                } else {
                    TelemetryEvent::ReconciliationStarted
                };
                telemetry::log_transition(
                    tel,
                    &record.internal_execution_id,
                    record.polymarket_order_id.as_ref(),
                    from_state,
                    record.current_execution_state,
                    "reconciliation/recovery started",
                );
            }
            ExecutionEvent::ReconciliationCompleted { .. } | ExecutionEvent::RecoveryCompleted { .. } => {
                let tel = if matches!(event, ExecutionEvent::RecoveryCompleted { .. }) {
                    TelemetryEvent::RecoveryCompleted
                } else {
                    TelemetryEvent::ReconciliationCompleted
                };
                telemetry::log_transition(
                    tel,
                    &record.internal_execution_id,
                    record.polymarket_order_id.as_ref(),
                    from_state,
                    record.current_execution_state,
                    "reconciliation/recovery completed",
                );
            }
            ExecutionEvent::ExecutionCreated { .. } => {}
        }

        if let Some(hint) = event.target_state_hint() {
            if !state_changed && hint != from_state {
                if validate_transition(from_state, hint).is_ok() {
                    self.transition(&mut record, hint)?;
                    state_changed = true;
                } else {
                    warn!(
                        execution_id = %record.internal_execution_id.0,
                        from = %from_state,
                        hint = %hint,
                        "ignored stale or out-of-order state hint"
                    );
                }
            }
        }

        record.processed_event_ids.insert(event.event_id().clone());
        record.last_event_id = Some(event.event_id().clone());
        record.last_observed_at = Utc::now();
        record.refresh_verification_state();

        Ok(ApplyEventResult {
            execution: record,
            state_changed,
            event_accepted: true,
        })
    }

    fn transition(&self, record: &mut ExecutionRecord, to: ExecutionState) -> VerifierResult<()> {
        let from = record.current_execution_state;
        validate_transition(from, to)?;
        if from != to {
            info!(
                execution_id = %record.internal_execution_id.0,
                order_id = ?record.polymarket_order_id.as_ref().map(|o| &o.0),
                from_state = %from,
                to_state = %to,
                "execution state transition"
            );
            record.current_execution_state = to;
        }
        Ok(())
    }

    pub fn verification_snapshot(&self, record: &ExecutionRecord) -> VerificationResult {
        VerificationResult {
            verification_state: record.verification_state,
            position_check: None,
            reason: record.uncertainty_reason.clone().or(record.last_error.clone()),
        }
    }

    pub fn verify_position_for_record(
        &self,
        record: &ExecutionRecord,
        expected: Decimal,
        observed: Option<Decimal>,
    ) -> (PositionCheckResult, Option<ExecutionEvent>) {
        let check = position_check_result(expected, observed);
        let event = match check {
            PositionCheckResult::Match if record.current_execution_state == ExecutionState::Settled => {
                Some(ExecutionEvent::PositionVerified {
                    execution_id: record.internal_execution_id.clone(),
                    event_id: EventId(format!("pos-verified-{}", record.internal_execution_id.0)),
                    observed_at: Utc::now(),
                })
            }
            PositionCheckResult::Mismatch => Some(ExecutionEvent::PositionMismatchObserved {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId(format!("pos-mismatch-{}", record.internal_execution_id.0)),
                reason: format!(
                    "expected {} observed {:?}",
                    expected,
                    observed.unwrap_or(Decimal::ZERO)
                ),
                observed_at: Utc::now(),
            }),
            _ => None,
        };
        (check, event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::identifiers::{EventId, TokenId, TradeId};
    use rust_decimal::Decimal;

    fn sample_record() -> ExecutionRecord {
        ExecutionVerifier.create_execution(
            TokenId("token-1".into()),
            Side::Buy,
            Decimal::from(100),
            None,
        )
    }

    fn eid(s: &str) -> EventId {
        EventId(s.into())
    }

    #[test]
    fn partial_fill_aggregation() {
        let verifier = ExecutionVerifier;
        let mut record = sample_record();
        record = verifier
            .apply_event(
                record,
                ExecutionEvent::OrderSubmitted {
                    execution_id: record.internal_execution_id.clone(),
                    event_id: eid("sub"),
                    order_id: PolymarketOrderId("ord-1".into()),
                    observed_at: Utc::now(),
                },
            )
            .unwrap()
            .execution;
        record = verifier
            .apply_event(
                record,
                ExecutionEvent::OrderAccepted {
                    execution_id: record.internal_execution_id.clone(),
                    event_id: eid("acc"),
                    observed_at: Utc::now(),
                },
            )
            .unwrap()
            .execution;
        record = verifier
            .apply_event(
                record,
                ExecutionEvent::OrderMatched {
                    execution_id: record.internal_execution_id.clone(),
                    event_id: eid("mat"),
                    observed_at: Utc::now(),
                },
            )
            .unwrap()
            .execution;
        for (i, qty) in [(1, 40), (2, 20), (3, 40)] {
            record = verifier
                .apply_event(
                    record,
                    ExecutionEvent::FillObserved {
                        execution_id: record.internal_execution_id.clone(),
                        event_id: EventId(format!("fill-{i}")),
                        trade_id: TradeId(format!("t-{i}")),
                        fill_quantity: Decimal::from(qty),
                        fill_price: Some(Decimal::new(50, 2)),
                        observed_at: Utc::now(),
                    },
                )
                .unwrap()
                .execution;
        }
        assert_eq!(record.matched_quantity, Decimal::from(100));
        assert_eq!(record.remaining_quantity, Decimal::ZERO);
        assert_eq!(record.current_execution_state, ExecutionState::Filled);
    }

    #[test]
    fn duplicate_fill_ignored() {
        let verifier = ExecutionVerifier;
        let mut record = sample_record();
        let steps = [
            ExecutionEvent::OrderSubmitted {
                execution_id: record.internal_execution_id.clone(),
                event_id: eid("sub"),
                order_id: PolymarketOrderId("o".into()),
                observed_at: Utc::now(),
            },
            ExecutionEvent::OrderAccepted {
                execution_id: record.internal_execution_id.clone(),
                event_id: eid("acc"),
                observed_at: Utc::now(),
            },
            ExecutionEvent::OrderMatched {
                execution_id: record.internal_execution_id.clone(),
                event_id: eid("mat"),
                observed_at: Utc::now(),
            },
        ];
        for ev in steps {
            record = verifier.apply_event(record, ev).unwrap().execution;
        }
        let fill = ExecutionEvent::FillObserved {
            execution_id: record.internal_execution_id.clone(),
            event_id: eid("f1"),
            trade_id: TradeId("t1".into()),
            fill_quantity: Decimal::from(40),
            fill_price: None,
            observed_at: Utc::now(),
        };
        record = verifier.apply_event(record, fill.clone()).unwrap().execution;
        record = verifier.apply_event(record, fill).unwrap().execution;
        assert_eq!(record.matched_quantity, Decimal::from(40));
    }

    #[test]
    fn duplicate_event_id_ignored() {
        let verifier = ExecutionVerifier;
        let record = sample_record();
        let ev = ExecutionEvent::MarkUnknown {
            execution_id: record.internal_execution_id.clone(),
            event_id: eid("u1"),
            reason: "gap".into(),
            observed_at: Utc::now(),
        };
        let r1 = verifier.apply_event(record.clone(), ev.clone()).unwrap();
        let r2 = verifier.apply_event(r1.execution, ev).unwrap();
        assert!(r2.event_accepted == false || !r2.state_changed);
    }
}
