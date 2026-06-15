//! Errors for the stateless input guest.

use thiserror::Error;

/// Errors for the stateless input guest. Each variant tags the point at which
/// conversion or validation fails rather than carrying diagnostic detail.
#[derive(Debug, Error)]
pub enum Error {
    /// Shared guest validation failed.
    #[error(transparent)]
    Common(#[from] stateless_validator_common::guest::Error),
    /// A Cancun-onward active fork carried no blob schedule.
    #[error("missing blob schedule")]
    MissingBlobSchedule,
    /// The payload was not well formed.
    #[error("payload validation failed")]
    PayloadValidation,
    /// The reth execution path rejected the payload.
    #[error("execution failed")]
    Execution,
}
