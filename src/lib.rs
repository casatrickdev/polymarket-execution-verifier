//! Polymarket execution verification core library.
//!
//! Verifies execution across order → fill → transaction → confirmation → settlement → position
//! without placing live orders.

pub mod domain;
pub mod error;
pub mod gateway;
pub mod reconciliation;
pub mod repository;
pub mod telemetry;
pub mod verifier;

pub use error::{RepositoryError, VerifierError, VerifierResult};
pub use reconciliation::ReconciliationExecutor;
pub use repository::ExecutionRepository;
pub use verifier::ExecutionVerifier;
