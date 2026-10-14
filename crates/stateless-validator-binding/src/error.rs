#![allow(missing_docs)]

use libssz::DecodeError;

/// Every variant fails the proof and the guest commits no output, unlike the bare stateless
/// validator, whose output carries a `successful_validation` verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    MissingSchemaId,
    UnsupportedSchemaId(u16),
    Ssz(DecodeError),
    StateFieldBranch(u64),
    BeaconBlockRoot,
    ParentBeaconBlockRoot,
    BuilderIndex,
    PrevRandao,
    GasLimit,
    BlockHash,
    ExecutionRequestsRoot,
    SlotNumber,
    ParentHash,
    Timestamp,
    WithdrawalsRoot,
    TooManyWithdrawals,
    TooManyWithdrawalRequests,
    TooManyConsolidationRequests,
    TooManyBuilderDepositRequests,
    TooManyBuilderExitRequests,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl From<DecodeError> for Error {
    fn from(err: DecodeError) -> Self {
        Self::Ssz(err)
    }
}
