use rust_decimal::Decimal;

use crate::domain::execution::ExecutionRecord;
use crate::domain::states::{ExecutionState, PositionCheckResult};
use crate::domain::verification::PositionMismatch;

/// Whether a fill observation implies PARTIAL vs FILLED execution state.
pub fn fill_derived_state(record: &ExecutionRecord) -> ExecutionState {
    if record.matched_quantity <= Decimal::ZERO {
        ExecutionState::Matched
    } else if record.matched_quantity < record.requested_quantity {
        ExecutionState::Partial
    } else {
        ExecutionState::Filled
    }
}

pub fn compare_position(
    record: &ExecutionRecord,
    expected: Decimal,
    observed: Option<Decimal>,
) -> Result<(), PositionMismatch> {
    let observed = match observed {
        Some(q) => q,
        None => {
            return Err(PositionMismatch::new(
                record.token_id.clone(),
                expected,
                Decimal::ZERO,
                vec![record.internal_execution_id.clone()],
                "position data unavailable",
            ));
        }
    };

    if observed == expected {
        Ok(())
    } else {
        Err(PositionMismatch::new(
            record.token_id.clone(),
            expected,
            observed,
            vec![record.internal_execution_id.clone()],
            "expected position does not match observed position",
        ))
    }
}

pub fn position_check_result(
    expected: Decimal,
    observed: Option<Decimal>,
) -> PositionCheckResult {
    match observed {
        None => PositionCheckResult::Unknown,
        Some(o) if o == expected => PositionCheckResult::Match,
        Some(_) => PositionCheckResult::Mismatch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::identifiers::TokenId;
    use crate::domain::states::Side;
    use crate::verifier::ExecutionVerifier;

    #[test]
    fn position_match_and_unknown() {
        let verifier = ExecutionVerifier;
        let record = verifier.create_execution(TokenId("t".into()), Side::Buy, Decimal::from(100), None);
        assert_eq!(
            position_check_result(Decimal::from(40), Some(Decimal::from(40))),
            PositionCheckResult::Match
        );
        assert_eq!(
            position_check_result(Decimal::from(40), Some(Decimal::from(60))),
            PositionCheckResult::Mismatch
        );
        assert_eq!(
            position_check_result(Decimal::from(40), None),
            PositionCheckResult::Unknown
        );
        let err = compare_position(&record, Decimal::from(100), Some(Decimal::from(60))).unwrap_err();
        assert!(err.difference != Decimal::ZERO);
    }
}
