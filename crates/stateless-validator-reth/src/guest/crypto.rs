//! Crypto provider selection for the guest.

#[cfg(all(feature = "openvm", feature = "zkvm-interface"))]
compile_error!("enable at most one of the openvm and zkvm-interface features");

#[cfg(feature = "openvm")]
pub(crate) mod openvm;
#[cfg(feature = "zkvm-interface")]
pub mod zkvm_interface;

use stateless_validator_common::Sha256Hasher;

/// Returns the [`Sha256Hasher`] implementation for the active zkVM feature.
#[allow(unreachable_code)]
pub(crate) fn sha256_hasher() -> impl Sha256Hasher {
    #[cfg(feature = "openvm")]
    return openvm::OpenVMSha256Hasher;
    #[cfg(feature = "zkvm-interface")]
    return zkvm_interface::sha256_hasher();
    #[cfg(not(any(feature = "openvm", feature = "zkvm-interface")))]
    return stateless_validator_common::Sha2Hasher;
}

/// SHA256 hashes the data using the [`sha256_hasher`].
pub(crate) fn sha256(data: &[u8]) -> [u8; 32] {
    sha256_hasher().hash(data)
}
