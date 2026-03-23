// Copied and modified from https://github.com/0xPolygonHermez/zisk-eth-client/blob/v0.7.0/crates/guest-reth/src/crypto/mod.rs.

use std::sync::Arc;

use alloy_consensus::crypto::install_default_provider;
use revm::install_crypto;
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
use revm::precompile::DefaultCrypto;

#[rustfmt::skip]
mod ffi;
#[rustfmt::skip]
mod impls;

#[derive(Debug)]
pub struct CustomEvmCrypto {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    default_crypto: DefaultCrypto,
}

impl Default for CustomEvmCrypto {
    fn default() -> Self {
        Self {
            #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
            default_crypto: DefaultCrypto,
        }
    }
}

/// Install ZisK crypto implementations globally
pub fn install_zisk_crypto() -> Result<bool, Box<dyn std::error::Error>> {
    let installed = install_crypto(CustomEvmCrypto::default());

    install_default_provider(Arc::new(CustomEvmCrypto::default())).unwrap();

    Ok(installed)
}
