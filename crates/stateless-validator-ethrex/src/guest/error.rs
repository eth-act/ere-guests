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
    /// The blob target exceeded the ethrex `u32` bound.
    #[error("blob target out of bounds")]
    BlobTargetOutOfBounds,
    /// The blob max exceeded the ethrex `u32` bound.
    #[error("blob max out of bounds")]
    BlobMaxOutOfBounds,
    /// The payload variant has no ethrex execution path.
    #[error("unsupported payload")]
    UnsupportedPayload,
    /// The block access list exceeded the ethrex bound.
    #[error("block access list out of bounds")]
    BlockAccessListOutOfBounds,
    /// The witness state exceeded the ethrex bounds.
    #[error("witness state out of bounds")]
    WitnessStateOutOfBounds,
    /// The witness codes exceeded the ethrex bounds.
    #[error("witness codes out of bounds")]
    WitnessCodesOutOfBounds,
    /// The witness headers did not decode.
    #[error("witness headers decode failed")]
    WitnessHeadersDecode,
    /// The witness headers did not form a chain.
    #[error("witness headers chain invalid")]
    WitnessHeadersChain,
    /// The witness did not build into the state tries.
    #[error("witness build failed")]
    WitnessBuild,
    /// The ethrex execution path rejected the payload.
    #[error("execution failed")]
    Execution,
}
