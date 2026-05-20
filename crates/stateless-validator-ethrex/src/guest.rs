//! Stateless validator guest program.

use alloc::{sync::Arc, vec::Vec};

use ethrex_crypto::Crypto;
use ethrex_guest_program::execution::execution_program;

#[cfg(feature = "zkvm-interface")]
mod zkvm_interface;

#[rustfmt::skip]
pub use {
    guest::*,
    stateless_validator_common::guest::StatelessValidatorOutput,
};

/// [`Guest`] implementation for Ethrex stateless validator.
#[derive(Debug, Clone)]
pub struct StatelessValidatorEthrexGuest;

impl Guest for StatelessValidatorEthrexGuest {
    type Input = Vec<u8>;
    type Output = StatelessValidatorOutput;

    fn compute<P: Platform>(input_bytes: Self::Input) -> Self::Output {
        Self::compute_inner::<P>(&input_bytes, crypto())
    }
}

impl StatelessValidatorEthrexGuest {
    fn compute_inner<P: Platform>(
        input_bytes: &[u8],
        crypto: Arc<dyn Crypto>,
    ) -> GuestOutput<Self> {
        let output = P::cycle_scope("run_validation", || {
            execution_program(input_bytes, crypto)
                .unwrap_or_else(|err| panic!("invalid EIP-8025 input: {err}"))
        });

        StatelessValidatorOutput::new(
            output.new_payload_request_root,
            output.valid,
            output.chain_id,
        )
    }
}

#[allow(unreachable_code)]
fn crypto() -> Arc<dyn Crypto> {
    #[cfg(feature = "zkvm-interface")]
    return zkvm_interface::crypto();
    #[cfg(feature = "risc0")]
    return Arc::new(ethrex_guest_program::crypto::risc0::Risc0Crypto);
    #[cfg(feature = "sp1")]
    return Arc::new(ethrex_guest_program::crypto::sp1::Sp1Crypto);
    #[cfg(not(any(feature = "zkvm-interface", feature = "risc0", feature = "sp1")))]
    return Arc::new(ethrex_guest_program::crypto::NativeCrypto);
}
