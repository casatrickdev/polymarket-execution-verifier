use chrono::Utc;
use polymarket_execution_verifier::domain::events::ExecutionEvent;
use polymarket_execution_verifier::domain::identifiers::{
    EventId, PolymarketOrderId, TokenId, TradeId, TransactionHash,
};
use polymarket_execution_verifier::domain::states::{ExecutionState, Side, VerificationState};
use polymarket_execution_verifier::gateway::mock::MockGateway;
use polymarket_execution_verifier::gateway::TransactionSource;
use polymarket_execution_verifier::repository::memory::InMemoryExecutionRepository;
use polymarket_execution_verifier::repository::ExecutionRepository;
use polymarket_execution_verifier::reconciliation::ReconciliationExecutor;
use polymarket_execution_verifier::verifier::transitions::validate_transition;
use polymarket_execution_verifier::verifier::ExecutionVerifier;
use polymarket_execution_verifier::VerifierError;
use rust_decimal::Decimal;

fn dec(n: i64) -> Decimal {
    Decimal::from(n)
}

fn bootstrap_through_matched(verifier: &ExecutionVerifier, record: polymarket_execution_verifier::domain::execution::ExecutionRecord) -> polymarket_execution_verifier::domain::execution::ExecutionRecord {
    let id = record.internal_execution_id.clone();
    let steps = [
        ExecutionEvent::OrderSubmitted {
            execution_id: id.clone(),
            event_id: EventId("e-sub".into()),
            order_id: PolymarketOrderId("order-1".into()),
            observed_at: Utc::now(),
        },
        ExecutionEvent::OrderAccepted {
            execution_id: id.clone(),
            event_id: EventId("e-acc".into()),
            observed_at: Utc::now(),
        },
        ExecutionEvent::OrderMatched {
            execution_id: id.clone(),
            event_id: EventId("e-mat".into()),
            observed_at: Utc::now(),
        },
    ];
    let mut r = record;
    for s in steps {
        r = verifier.apply_event(r, s).unwrap().execution;
    }
    r
}

#[test]
fn invalid_transition_rejected() {
    let err = validate_transition(ExecutionState::PositionVerified, ExecutionState::Submitted)
        .unwrap_err();
    assert!(matches!(err, VerifierError::IllegalTransition { .. }));
}

#[test]
fn single_partial_fill() {
    let verifier = ExecutionVerifier;
    let record = verifier.create_execution(TokenId("tok".into()), Side::Buy, dec(100), None);
    let record = bootstrap_through_matched(&verifier, record);
    let record = verifier
        .apply_event(
            record,
            ExecutionEvent::FillObserved {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId("fill-1".into()),
                trade_id: TradeId("t1".into()),
                fill_quantity: dec(40),
                fill_price: None,
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    assert_eq!(record.matched_quantity, dec(40));
    assert_eq!(record.remaining_quantity, dec(60));
    assert_eq!(record.current_execution_state, ExecutionState::Partial);
}

#[test]
fn filled_to_tx_pending() {
    let verifier = ExecutionVerifier;
    let record = verifier.create_execution(TokenId("tok".into()), Side::Buy, dec(100), None);
    let mut record = bootstrap_through_matched(&verifier, record);
    record = verifier
        .apply_event(
            record,
            ExecutionEvent::FillObserved {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId("fill".into()),
                trade_id: TradeId("t".into()),
                fill_quantity: dec(100),
                fill_price: None,
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    assert_eq!(record.current_execution_state, ExecutionState::Filled);
    record = verifier
        .apply_event(
            record,
            ExecutionEvent::TransactionPending {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId("tx-p".into()),
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    assert_eq!(record.current_execution_state, ExecutionState::TxPending);
}

#[test]
fn tx_pending_to_confirmed() {
    let verifier = ExecutionVerifier;
    let record = verifier.create_execution(TokenId("tok".into()), Side::Buy, dec(100), None);
    let mut record = bootstrap_through_matched(&verifier, record);
    record = verifier
        .apply_event(
            record,
            ExecutionEvent::FillObserved {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId("fill".into()),
                trade_id: TradeId("t".into()),
                fill_quantity: dec(100),
                fill_price: None,
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    record = verifier
        .apply_event(
            record,
            ExecutionEvent::TransactionPending {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId("tx-p".into()),
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    record = verifier
        .apply_event(
            record,
            ExecutionEvent::TransactionConfirmed {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId("tx-c".into()),
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    assert_eq!(record.current_execution_state, ExecutionState::Confirmed);
}

#[test]
fn temporary_unknown_does_not_become_failed() {
    let verifier = ExecutionVerifier;
    let record = verifier.create_execution(TokenId("tok".into()), Side::Buy, dec(100), None);
    let record = verifier
        .apply_event(
            record,
            ExecutionEvent::MarkUnknown {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId("unk".into()),
                reason: "lookup timeout".into(),
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    assert_eq!(record.current_execution_state, ExecutionState::Unknown);
    assert_eq!(record.verification_state, VerificationState::Unknown);
}

#[test]
fn position_mismatch_inconsistent() {
    let verifier = ExecutionVerifier;
    let record = verifier.create_execution(TokenId("tok".into()), Side::Buy, dec(100), None);
    let mut record = bootstrap_through_matched(&verifier, record);
    record = verifier
        .apply_event(
            record,
            ExecutionEvent::FillObserved {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId("fill".into()),
                trade_id: TradeId("t".into()),
                fill_quantity: dec(100),
                fill_price: None,
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    for (eid, ev) in [
        (
            "tx-p",
            ExecutionEvent::TransactionPending {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId("tx-p".into()),
                observed_at: Utc::now(),
            },
        ),
        (
            "tx-c",
            ExecutionEvent::TransactionConfirmed {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId("tx-c".into()),
                observed_at: Utc::now(),
            },
        ),
        (
            "settle",
            ExecutionEvent::SettlementObserved {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId("settle".into()),
                observed_at: Utc::now(),
            },
        ),
    ] {
        record = verifier.apply_event(record, ev).unwrap().execution;
        let _ = eid;
    }
    record = verifier
        .apply_event(
            record,
            ExecutionEvent::PositionMismatchObserved {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId("pm".into()),
                reason: "observed 60 expected 100".into(),
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    assert_eq!(record.current_execution_state, ExecutionState::Inconsistent);
}

#[tokio::test]
async fn recovery_after_event_gap() {
    let verifier = ExecutionVerifier;
    let gateway = MockGateway::new();
    let mut record = verifier.create_execution(TokenId("tok".into()), Side::Buy, dec(100), None);
    record = bootstrap_through_matched(&verifier, record);
    record = verifier
        .apply_event(
            record,
            ExecutionEvent::FillObserved {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId("fill-pre-gap".into()),
                trade_id: TradeId("t1".into()),
                fill_quantity: dec(100),
                fill_price: None,
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    record = verifier
        .apply_event(
            record,
            ExecutionEvent::TransactionObserved {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId("tx-hash".into()),
                transaction_hash: TransactionHash("0xabc".into()),
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    record = verifier
        .apply_event(
            record,
            ExecutionEvent::EventGap {
                execution_id: record.internal_execution_id.clone(),
                event_id: EventId("gap".into()),
                reason: "websocket disconnect".into(),
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    assert_eq!(record.current_execution_state, ExecutionState::Unknown);

    gateway
        .set_order(polymarket_execution_verifier::domain::events::ObservedOrder {
            order_id: PolymarketOrderId("order-1".into()),
            token_id: TokenId("tok".into()),
            matched_quantity: dec(100),
            state_label: "FILLED".into(),
        })
        .await;
    gateway
        .set_trades(
            &PolymarketOrderId("order-1".into()),
            vec![polymarket_execution_verifier::domain::events::ObservedTrade {
                trade_id: TradeId("t1".into()),
                quantity: dec(100),
                price: None,
            }],
        )
        .await;
    gateway.set_position(&TokenId("tok".into()), dec(100)).await;
    gateway
        .set_transaction_status(
            &TransactionHash("0xabc".into()),
            polymarket_execution_verifier::domain::events::TransactionStatus::Confirmed,
        )
        .await;

    let exec = ReconciliationExecutor::new();
    let result = exec
        .recover_execution(&record, &gateway, &gateway, &gateway, Decimal::ZERO)
        .await
        .unwrap();
    assert!(result.recovered);
    assert_eq!(
        result.execution.current_execution_state,
        ExecutionState::PositionVerified
    );
}

#[tokio::test]
async fn idempotent_event_processing() {
    let verifier = ExecutionVerifier;
    let repo = InMemoryExecutionRepository::new();
    let record = verifier.create_execution(TokenId("tok".into()), Side::Buy, dec(50), None);
    repo.save(&record).await.unwrap();
    let ev = ExecutionEvent::MarkUnknown {
        execution_id: record.internal_execution_id.clone(),
        event_id: EventId("dup".into()),
        reason: "x".into(),
        observed_at: Utc::now(),
    };
    let r1 = verifier.apply_event(record.clone(), ev.clone()).unwrap().execution;
    let r2 = verifier.apply_event(r1.clone(), ev).unwrap().execution;
    assert_eq!(r1.current_execution_state, r2.current_execution_state);
    repo.save(&r2).await.unwrap();
    let loaded = repo.get(&r2.internal_execution_id).await.unwrap().unwrap();
    assert_eq!(loaded.current_execution_state, ExecutionState::Unknown);
}

#[test]
fn rejected_and_cancelled_orders() {
    let verifier = ExecutionVerifier;
    let record = verifier.create_execution(TokenId("tok".into()), Side::Buy, dec(10), None);
    let id = record.internal_execution_id.clone();
    let rejected = verifier
        .apply_event(
            record.clone(),
            ExecutionEvent::OrderSubmitted {
                execution_id: id.clone(),
                event_id: EventId("rs".into()),
                order_id: PolymarketOrderId("o".into()),
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    let rejected = verifier
        .apply_event(
            rejected,
            ExecutionEvent::OrderRejected {
                execution_id: id.clone(),
                event_id: EventId("rj".into()),
                reason: "insufficient balance".into(),
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    assert_eq!(rejected.current_execution_state, ExecutionState::Rejected);

    let cancelled = verifier
        .apply_event(
            record,
            ExecutionEvent::OrderSubmitted {
                execution_id: id.clone(),
                event_id: EventId("cs".into()),
                order_id: PolymarketOrderId("o2".into()),
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    let cancelled = verifier
        .apply_event(
            cancelled,
            ExecutionEvent::OrderAccepted {
                execution_id: id,
                event_id: EventId("ca".into()),
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    let cancelled = verifier
        .apply_event(
            cancelled,
            ExecutionEvent::OrderCancelled {
                execution_id: cancelled.internal_execution_id.clone(),
                event_id: EventId("cc".into()),
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    assert_eq!(cancelled.current_execution_state, ExecutionState::Cancelled);
}

#[tokio::test]
async fn sqlite_persistence_reload() {
    use polymarket_execution_verifier::repository::sqlite::SqliteExecutionRepository;
    let repo = SqliteExecutionRepository::open_in_memory().unwrap();
    let verifier = ExecutionVerifier;
    let record = verifier.create_execution(TokenId("tok".into()), Side::Buy, dec(25), None);
    repo.save(&record).await.unwrap();
    let loaded = repo.get(&record.internal_execution_id).await.unwrap().unwrap();
    assert_eq!(loaded.requested_quantity, dec(25));
}

#[tokio::test]
async fn transaction_unknown_stays_uncertain() {
    let gateway = MockGateway::new();
    gateway
        .set_transaction_status(
            &TransactionHash("0xabc".into()),
            polymarket_execution_verifier::domain::events::TransactionStatus::Unknown,
        )
        .await;
    let status = gateway
        .get_transaction_status(&TransactionHash("0xabc".into()))
        .await
        .unwrap();
    assert!(matches!(
        status,
        polymarket_execution_verifier::domain::events::TransactionStatus::Unknown
    ));
}
