//! Crypto provider selection for the guest.

#[cfg(feature = "openvm")]
mod openvm;
#[cfg(feature = "zkvm-interface")]
mod zkvm_interface;

use alloc::sync::Arc;

use ethrex_crypto::Crypto;
use stateless_validator_common::Sha256Hasher;

/// Returns the [`Crypto`] implementation for the active zkVM feature.
#[allow(unreachable_code)]
pub(crate) fn crypto() -> Arc<dyn Crypto> {
    #[cfg(feature = "openvm")]
    return openvm::crypto();
    #[cfg(feature = "zkvm-interface")]
    return zkvm_interface::crypto();
    #[cfg(not(any(feature = "openvm", feature = "zkvm-interface")))]
    return Arc::new(ethrex_guest_program::crypto::NativeCrypto);
}

/// Returns the [`Sha256Hasher`] implementation for the active zkVM feature.
pub(crate) fn sha256_hasher() -> impl Sha256Hasher {
    struct CryptoSha256Hasher(Arc<dyn Crypto>);

    impl Sha256Hasher for CryptoSha256Hasher {
        fn hash(&self, data: &[u8]) -> [u8; 32] {
            self.0.sha256(data)
        }
    }

    CryptoSha256Hasher(crypto())
}
