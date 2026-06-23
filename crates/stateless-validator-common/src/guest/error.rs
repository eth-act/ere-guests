//! Common errors for the stateless input guest.

use libssz::DecodeError;
use thiserror::Error;

/// Common errors for the stateless input guest.
#[derive(Debug, Error)]
pub enum Error {
    /// The input is shorter than the schema identifier prefix.
    #[error("stateless input is missing the schema id")]
    MissingSchemaId,
    /// The schema identifier prefix does not match the supported schema id.
    #[error("unsupported stateless input schema id {0:#06x}")]
    UnsupportedSchemaId(u16),
    /// The SSZ body failed to decode.
    #[error("SSZ decode error {0:?}")]
    Ssz(DecodeError),
    /// The fork activation has neither block_number nor timestamp set, mirroring the spec
    /// `InvalidForkActivationError`.
    #[error("Fork activation must set block_number or timestamp")]
    InvalidForkActivation,
    /// The configured active fork is not active for the payload, mirroring the spec
    /// `InactiveForkConfigError`.
    #[error("ChainConfig active_fork is not active for the target payload")]
    InactiveForkConfig,
    /// The configured active fork does not match the payload shape. The spec executes only
    /// Amsterdam and so has no counterpart; this replaces the spec `UnsupportedForkConfigError`
    /// for multi-fork inputs.
    #[error("ChainConfig active_fork is not matching the target payload version")]
    ForkNotMatchingPayload,
}

impl From<DecodeError> for Error {
    fn from(err: DecodeError) -> Self {
        Self::Ssz(err)
    }
}
