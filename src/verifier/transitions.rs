use crate::domain::states::ExecutionState;
use crate::error::{VerifierError, VerifierResult};

/// Validates whether `from` may transition to `to`.
pub fn validate_transition(from: ExecutionState, to: ExecutionState) -> VerifierResult<()> {
    if from == to {
        return Ok(());
    }

    if from.is_terminal() {
        return Err(VerifierError::IllegalTransition {
            from,
            to,
            reason: "terminal state cannot change".into(),
        });
    }

    let allowed = allowed_targets(from);
    if allowed.contains(&to) {
        Ok(())
    } else {
        Err(VerifierError::IllegalTransition {
            from,
            to,
            reason: format!("transition not in allow-list: {:?}", allowed),
        })
    }
}

fn allowed_targets(from: ExecutionState) -> Vec<ExecutionState> {
    use ExecutionState::*;
    match from {
        Intended => vec![Submitted, Failed, Unknown],
        Submitted => vec![Accepted, Rejected, Cancelled, Failed, Unknown, Retrying],
        Accepted => vec![Matched, Cancelled, Failed, Unknown, Retrying],
        Matched => vec![Partial, Filled, Failed, Unknown, Retrying],
        Partial => vec![Partial, Filled, TxPending, Unknown, Retrying],
        Filled => vec![TxPending, Unknown, Retrying, Failed],
        TxPending => vec![Confirmed, Unknown, Retrying, Failed],
        Confirmed => vec![Settled, Inconsistent, Unknown, Retrying],
        Settled => vec![PositionVerified, Inconsistent, Unknown, Retrying],
        Rejected | Cancelled | Failed | PositionVerified | Inconsistent => vec![],
        Retrying => vec![
            Submitted,
            Accepted,
            Matched,
            Partial,
            Filled,
            TxPending,
            Confirmed,
            Settled,
            Unknown,
            Reconciling,
        ],
        Unknown => vec![
            Reconciling,
            Submitted,
            Accepted,
            Matched,
            Partial,
            Filled,
            TxPending,
            Confirmed,
            Settled,
            Retrying,
            Failed,
        ],
        Reconciling => vec![
            Submitted,
            Accepted,
            Matched,
            Partial,
            Filled,
            TxPending,
            Confirmed,
            Settled,
            PositionVerified,
            Inconsistent,
            Unknown,
            Retrying,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::states::ExecutionState;

    #[test]
    fn valid_forward_chain() {
        let chain = [
            ExecutionState::Intended,
            ExecutionState::Submitted,
            ExecutionState::Accepted,
            ExecutionState::Matched,
            ExecutionState::Filled,
            ExecutionState::TxPending,
            ExecutionState::Confirmed,
            ExecutionState::Settled,
            ExecutionState::PositionVerified,
        ];
        for w in chain.windows(2) {
            validate_transition(w[0], w[1]).expect("valid chain step");
        }
    }

    #[test]
    fn invalid_position_verified_to_submitted() {
        let err = validate_transition(ExecutionState::PositionVerified, ExecutionState::Submitted)
            .unwrap_err();
        assert!(matches!(err, VerifierError::IllegalTransition { .. }));
    }

    #[test]
    fn partial_path() {
        validate_transition(ExecutionState::Matched, ExecutionState::Partial).unwrap();
        validate_transition(ExecutionState::Partial, ExecutionState::Filled).unwrap();
    }

    #[test]
    fn filled_to_unknown() {
        validate_transition(ExecutionState::Filled, ExecutionState::Unknown).unwrap();
    }

    #[test]
    fn confirmed_to_inconsistent() {
        validate_transition(ExecutionState::Confirmed, ExecutionState::Inconsistent).unwrap();
    }
}
