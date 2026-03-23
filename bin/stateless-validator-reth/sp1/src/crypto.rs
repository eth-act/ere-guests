use kzg_rs::{Bytes32, Bytes48, KzgProof, KzgSettings};
use revm::precompile::{Crypto, PrecompileError};

/// Install SP1 crypto implementations
pub fn install_crypto() {
    assert!(revm::install_crypto(CustomCrypto::default()));
}

/// Copied and modified from https://github.com/succinctlabs/rsp/blob/main/crates/executor/client/src/custom.rs#L160.
#[derive(Debug)]
struct CustomCrypto {
    kzg_settings: KzgSettings,
}

impl Default for CustomCrypto {
    fn default() -> Self {
        Self {
            kzg_settings: KzgSettings::load_trusted_setup_file().unwrap(),
        }
    }
}

impl Crypto for CustomCrypto {
    fn verify_kzg_proof(
        &self,
        z: &[u8; 32],
        y: &[u8; 32],
        commitment: &[u8; 48],
        proof: &[u8; 48],
    ) -> Result<(), PrecompileError> {
        if !KzgProof::verify_kzg_proof(
            &Bytes48(*commitment),
            &Bytes32(*z),
            &Bytes32(*y),
            &Bytes48(*proof),
            &self.kzg_settings,
        )
        .map_err(|err| PrecompileError::other(err.to_string()))?
        {
            return Err(PrecompileError::BlobVerifyKzgProofFailed);
        }

        Ok(())
    }
}
