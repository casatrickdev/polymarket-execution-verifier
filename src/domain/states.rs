use serde::{Deserialize, Serialize};

/// Lifecycle state of a single execution observation stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionState {
    Intended,
    Submitted,
    Accepted,
    Matched,
    Partial,
    Filled,
    TxPending,
    Confirmed,
    Settled,
    PositionVerified,
    Rejected,
    Cancelled,
    Failed,
    Retrying,
    Unknown,
    Inconsistent,
    Reconciling,
}

impl ExecutionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Intended => "INTENDED",
            Self::Submitted => "SUBMITTED",
            Self::Accepted => "ACCEPTED",
            Self::Matched => "MATCHED",
            Self::Partial => "PARTIAL",
            Self::Filled => "FILLED",
            Self::TxPending => "TX_PENDING",
            Self::Confirmed => "CONFIRMED",
            Self::Settled => "SETTLED",
            Self::PositionVerified => "POSITION_VERIFIED",
            Self::Rejected => "REJECTED",
            Self::Cancelled => "CANCELLED",
            Self::Failed => "FAILED",
            Self::Retrying => "RETRYING",
            Self::Unknown => "UNKNOWN",
            Self::Inconsistent => "INCONSISTENT",
            Self::Reconciling => "RECONCILING",
        }
    }

    /// Terminal states that must not regress on stale/duplicate events.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Rejected
                | Self::Cancelled
                | Self::Failed
                | Self::PositionVerified
                | Self::Inconsistent
        )
    }

    pub fn is_verified_success(&self) -> bool {
        matches!(self, Self::PositionVerified)
    }
}

impl std::fmt::Display for ExecutionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Overall verification outcome (derived, not a raw exchange event).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationState {
    Pending,
    InProgress,
    Verified,
    Failed,
    Unknown,
    Inconsistent,
    Retrying,
}

impl VerificationState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::InProgress => "IN_PROGRESS",
            Self::Verified => "VERIFIED",
            Self::Failed => "FAILED",
            Self::Unknown => "UNKNOWN",
            Self::Inconsistent => "INCONSISTENT",
            Self::Retrying => "RETRYING",
        }
    }
}

impl std::fmt::Display for VerificationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PositionCheckResult {
    Match,
    Mismatch,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}
