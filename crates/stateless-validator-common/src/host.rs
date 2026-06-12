//! Host-side utilities for the canonical stateless validation flow.

use libssz::SszEncode;
pub use libssz_merkle::Sha2Hasher;
use sha2::{Digest, Sha256};

use crate::guest::StatelessValidationResult;

#[cfg(feature = "legacy")]
pub mod legacy;

impl StatelessValidationResult {
    /// Returns the SHA-256 digest of the serialized result.
    pub fn sha256(&self) -> [u8; 32] {
        Sha256::digest(self.to_ssz()).into()
    }
}
