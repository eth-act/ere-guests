// Copied and modified and modified from https://github.com/0xPolygonHermez/zisk-eth-client/blob/develop-0.8.0/crates/guest-ethrex/src/crypto/mod.rs.

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
use ethrex_crypto::NativeCrypto;

mod impls;
mod ffi;

#[derive(Debug)]
pub(crate) struct ZiskCrypto {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    native_crypto: NativeCrypto,
}

impl Default for ZiskCrypto {
    fn default() -> Self {
        Self {
            #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
            native_crypto: NativeCrypto,
        }
    }
}
