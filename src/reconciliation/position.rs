use rust_decimal::Decimal;

use crate::domain::execution::ExecutionRecord;
use crate::domain::states::PositionCheckResult;
use crate::domain::verification::PositionMismatch;
use crate::verifier::policies::compare_position;

pub fn expected_position_delta(record: &ExecutionRecord) -> Decimal {
    match record.side {
        crate::domain::states::Side::Buy => record.matched_quantity,
        crate::domain::states::Side::Sell => -record.matched_quantity,
    }
}

pub fn evaluate_position(
    record: &ExecutionRecord,
    baseline: Decimal,
    observed: Option<Decimal>,
) -> Result<PositionCheckResult, PositionMismatch> {
    let expected = baseline + expected_position_delta(record);
    compare_position(record, expected, observed)?;
    Ok(PositionCheckResult::Match)
}
