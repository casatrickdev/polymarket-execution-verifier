use chrono::Utc;
use polymarket_execution_verifier::domain::events::ExecutionEvent;
use polymarket_execution_verifier::domain::identifiers::{EventId, PolymarketOrderId, TokenId, TradeId};
use polymarket_execution_verifier::domain::states::{ExecutionState, Side};
use polymarket_execution_verifier::verifier::ExecutionVerifier;
use rust_decimal::Decimal;
use tracing_subscriber::EnvFilter;

fn dec(n: i64) -> Decimal {
    Decimal::from(n)
}

fn print_header(title: &str) {
    println!("\n=== {title} ===");
}

fn print_state(label: &str, state: ExecutionState, matched: Decimal, remaining: Decimal) {
    println!(
        "{label}: execution_state={state}, matched={matched}, remaining={remaining}"
    );
}

fn bootstrap(
    verifier: &ExecutionVerifier,
    mut record: polymarket_execution_verifier::domain::execution::ExecutionRecord,
    order_id: &str,
) -> polymarket_execution_verifier::domain::execution::ExecutionRecord {
    let id = record.internal_execution_id.clone();
    for (eid, ev) in [
        (
            "sub",
            ExecutionEvent::OrderSubmitted {
                execution_id: id.clone(),
                event_id: EventId("a-sub".into()),
                order_id: PolymarketOrderId(order_id.into()),
                observed_at: Utc::now(),
            },
        ),
        (
            "acc",
            ExecutionEvent::OrderAccepted {
                execution_id: id.clone(),
                event_id: EventId("a-acc".into()),
                observed_at: Utc::now(),
            },
        ),
        (
            "mat",
            ExecutionEvent::OrderMatched {
                execution_id: id.clone(),
                event_id: EventId("a-mat".into()),
                observed_at: Utc::now(),
            },
        ),
    ] {
        record = verifier.apply_event(record, ev).unwrap().execution;
        let _ = eid;
    }
    record
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let verifier = ExecutionVerifier;

    print_header("Scenario A: partial fill, tx pending, position unknown");
    let mut a = verifier.create_execution(TokenId("demo-token".into()), Side::Buy, dec(100), None);
    a = bootstrap(&verifier, a, "order-a");
    a = verifier
        .apply_event(
            a,
            ExecutionEvent::FillObserved {
                execution_id: a.internal_execution_id.clone(),
                event_id: EventId("a-fill".into()),
                trade_id: TradeId("a-t1".into()),
                fill_quantity: dec(40),
                fill_price: None,
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    a = verifier
        .apply_event(
            a,
            ExecutionEvent::TransactionPending {
                execution_id: a.internal_execution_id.clone(),
                event_id: EventId("a-txp".into()),
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    print_state("A", a.current_execution_state, a.matched_quantity, a.remaining_quantity);

    print_header("Scenario B: partial fill, tx confirmed, position verified");
    let mut b = verifier.create_execution(TokenId("demo-token".into()), Side::Buy, dec(100), None);
    b = bootstrap(&verifier, b, "order-b");
    b = verifier
        .apply_event(
            b,
            ExecutionEvent::FillObserved {
                execution_id: b.internal_execution_id.clone(),
                event_id: EventId("b-fill".into()),
                trade_id: TradeId("b-t1".into()),
                fill_quantity: dec(40),
                fill_price: None,
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    for ev in [
        ExecutionEvent::TransactionPending {
            execution_id: b.internal_execution_id.clone(),
            event_id: EventId("b-txp".into()),
            observed_at: Utc::now(),
        },
        ExecutionEvent::TransactionConfirmed {
            execution_id: b.internal_execution_id.clone(),
            event_id: EventId("b-txc".into()),
            observed_at: Utc::now(),
        },
        ExecutionEvent::SettlementObserved {
            execution_id: b.internal_execution_id.clone(),
            event_id: EventId("b-set".into()),
            observed_at: Utc::now(),
        },
        ExecutionEvent::PositionVerified {
            execution_id: b.internal_execution_id.clone(),
            event_id: EventId("b-pos".into()),
            observed_at: Utc::now(),
        },
    ] {
        b = verifier.apply_event(b, ev).unwrap().execution;
    }
    print_state("B", b.current_execution_state, b.matched_quantity, b.remaining_quantity);

    print_header("Scenario C: full fill, confirmed, position mismatch");
    let mut c = verifier.create_execution(TokenId("demo-token".into()), Side::Buy, dec(100), None);
    c = bootstrap(&verifier, c, "order-c");
    c = verifier
        .apply_event(
            c,
            ExecutionEvent::FillObserved {
                execution_id: c.internal_execution_id.clone(),
                event_id: EventId("c-fill".into()),
                trade_id: TradeId("c-t1".into()),
                fill_quantity: dec(100),
                fill_price: None,
                observed_at: Utc::now(),
            },
        )
        .unwrap()
        .execution;
    for ev in [
        ExecutionEvent::TransactionPending {
            execution_id: c.internal_execution_id.clone(),
            event_id: EventId("c-txp".into()),
            observed_at: Utc::now(),
        },
        ExecutionEvent::TransactionConfirmed {
            execution_id: c.internal_execution_id.clone(),
            event_id: EventId("c-txc".into()),
            observed_at: Utc::now(),
        },
        ExecutionEvent::SettlementObserved {
            execution_id: c.internal_execution_id.clone(),
            event_id: EventId("c-set".into()),
            observed_at: Utc::now(),
        },
        ExecutionEvent::PositionMismatchObserved {
            execution_id: c.internal_execution_id.clone(),
            event_id: EventId("c-mis".into()),
            reason: "expected position 100, observed 60".into(),
            observed_at: Utc::now(),
        },
    ] {
        c = verifier.apply_event(c, ev).unwrap().execution;
    }
    print_state("C", c.current_execution_state, c.matched_quantity, c.remaining_quantity);
}
