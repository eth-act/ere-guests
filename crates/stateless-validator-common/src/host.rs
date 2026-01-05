//! Stateless validator common types and utilities for host.

use sha2::{Digest, Sha256};

use crate::guest::StatelessValidatorOutput;

#[rustfmt::skip]
pub use reth_stateless::StatelessInput;

impl StatelessValidatorOutput {
    /// Constructs a output from [`StatelessInput`] and an bool indicating
    /// whehter the stateless validation is successful or not.
    pub fn from_stateless_input(stateless_input: &StatelessInput, success: bool) -> Self {
        Self::new(
            stateless_input.block.hash_slow(),
            stateless_input.block.parent_hash,
            success,
        )
    }

    /// Returns sha256 digest of serialized output.
    pub fn sha256(&self) -> [u8; 32] {
        Sha256::digest(self.serialize()).into()
    }
}
