//! Canonical stateless validation output types.
//!
//! The types mirror `StatelessValidationResult` in [`stateless.py`] and its SSZ schema in
//! [`stateless_ssz.py`]. The serialized form is the plain SSZ encoding without a schema prefix.
//!
//! [`stateless.py`]: https://github.com/ethereum/execution-specs/blob/projects/zkevm/src/ethereum/forks/amsterdam/stateless.py
//! [`stateless_ssz.py`]: https://github.com/ethereum/execution-specs/blob/projects/zkevm/src/ethereum/forks/amsterdam/stateless_ssz.py

use alloc::vec::Vec;

use libssz_derive::{SszDecode, SszEncode};

use crate::guest::input::{ChainConfig, ForkActivation, ForkConfig, ProtocolFork};

/// Canonical result returned by stateless validation.
///
/// The [`Default`] value is the sentinel result for undecodable input, mirroring
/// `_default_failed_stateless_output` in [`stateless_guest.py`].
///
/// [`stateless_guest.py`]: https://github.com/ethereum/execution-specs/blob/projects/zkevm/src/ethereum/forks/amsterdam/stateless_guest.py
#[derive(Debug, Clone, PartialEq, Eq, SszEncode, SszDecode)]
pub struct StatelessValidationResult {
    /// The SSZ hash tree root of the validated payload request.
    pub new_payload_request_root: [u8; 32],
    /// Whether the stateless validation succeeded.
    pub successful_validation: bool,
    /// The chain configuration echoed from the decoded input.
    pub chain_config: ChainConfig,
}

impl StatelessValidationResult {
    /// Constructs a new [`StatelessValidationResult`].
    pub fn new(
        new_payload_request_root: [u8; 32],
        successful_validation: bool,
        chain_config: ChainConfig,
    ) -> Self {
        Self {
            new_payload_request_root,
            successful_validation,
            chain_config,
        }
    }
}

impl Default for StatelessValidationResult {
    fn default() -> Self {
        Self::new(
            [0; 32],
            false,
            ChainConfig {
                chain_id: 0,
                active_fork: ForkConfig::new(
                    ProtocolFork::Frontier,
                    ForkActivation::default(),
                    None,
                ),
            },
        )
    }
}
