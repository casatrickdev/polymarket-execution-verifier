use tracing::info;

use crate::domain::events::TelemetryEvent;
use crate::domain::identifiers::{ExecutionId, PolymarketOrderId};
use crate::domain::states::ExecutionState;

pub fn log_transition(
    event: TelemetryEvent,
    execution_id: &ExecutionId,
    order_id: Option<&PolymarketOrderId>,
    from_state: ExecutionState,
    to_state: ExecutionState,
    reason: &str,
) {
    info!(
        telemetry_event = event.as_str(),
        execution_id = %execution_id.0,
        order_id = order_id.map(|o| o.0.as_str()),
        from_state = %from_state,
        to_state = %to_state,
        reason = reason,
        "verifier transition"
    );
}
