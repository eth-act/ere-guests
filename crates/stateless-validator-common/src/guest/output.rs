//! Canonical stateless validation output types.
//!
//! The types mirror `StatelessValidationResult` and its SSZ schema in [`stateless.py`].
//! The serialized form is the plain SSZ encoding without a schema prefix.
//!
//! [`stateless.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v21.0.7/src/ethereum/forks/amsterdam/stateless.py

use alloc::vec::Vec;

use libssz_derive::{HashTreeRoot, SszDecode, SszEncode};

/// Canonical result returned by stateless validation.
///
/// It is a progressive container, so its hash tree root matches the consensus-layer
/// `PublicInput`.
///
/// A zero `schema_id` means the guest could not decode the input or produce a validation result.
/// The other fields then hold their zero defaults, so the whole result equals [`Default`].
#[derive(Debug, Clone, Default, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
#[ssz(progressive_container)]
pub struct StatelessValidationResult {
    /// The SSZ hash tree root of the validated payload request.
    pub new_payload_request_root: [u8; 32],
    /// Whether the stateless validation succeeded.
    pub successful_validation: bool,
    /// The chain identifier echoed from the decoded input.
    pub chain_id: u64,
    /// The exact schema identifier decoded and executed by the guest.
    pub schema_id: u16,
}

#[cfg(test)]
mod tests {
    use libssz::{SszDecode as _, SszEncode as _};
    use libssz_merkle::{HashTreeRoot as _, Sha2Hasher};

    use super::*;

    fn validation_result() -> StatelessValidationResult {
        StatelessValidationResult {
            new_payload_request_root: [0xaa; 32],
            successful_validation: true,
            chain_id: 0x0102_0304_0506_0708,
            schema_id: 0x1501,
        }
    }

    #[test]
    fn validation_result_has_fixed_v08_layout() {
        let result = validation_result();
        let encoded = result.to_ssz();

        assert_eq!(encoded.len(), 43);
        assert_eq!(&encoded[..32], &[0xaa; 32]);
        assert_eq!(encoded[32], 1);
        assert_eq!(&encoded[33..41], &0x0102_0304_0506_0708_u64.to_le_bytes());
        assert_eq!(&encoded[41..], &0x1501_u16.to_le_bytes());
        assert_eq!(
            StatelessValidationResult::from_ssz_bytes(&encoded).unwrap(),
            result
        );
    }

    #[test]
    fn default_validation_result_is_zero_sentinel() {
        assert_eq!(StatelessValidationResult::default().to_ssz(), [0; 43]);
        assert_eq!(
            StatelessValidationResult::from_ssz_bytes(&[0; 43]).unwrap(),
            StatelessValidationResult::default()
        );
    }

    #[test]
    fn progressive_container_roots_match_reference_vectors() {
        for (result, expected) in [
            (
                validation_result(),
                "82aa1c81acf82a5b4033775fe90b5365348b23acf9d1d8ba9e8f55aace5a8dbd",
            ),
            (
                StatelessValidationResult::default(),
                "ef9da2c4e49e0acc0933a072ab744573e1c1e20806904bf0dd4b7eef0a9da836",
            ),
        ] {
            assert_eq!(
                const_hex::encode(result.hash_tree_root(&Sha2Hasher)),
                expected
            );
        }
    }

    #[test]
    fn rejects_malformed_validation_results() {
        for bytes in [&[0; 42][..], &[0; 44][..]] {
            assert!(StatelessValidationResult::from_ssz_bytes(bytes).is_err());
        }

        let mut invalid_boolean = StatelessValidationResult::default().to_ssz();
        invalid_boolean[32] = 2;
        assert!(StatelessValidationResult::from_ssz_bytes(&invalid_boolean).is_err());
    }
}
