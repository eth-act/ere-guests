//! Implementations for host environment.

use alloc::{format, vec::Vec};

use anyhow::Context;
use ere_zkvm_interface::Input;
use guest::{GuestIo, Io};
use reth_ethereum_primitives::TransactionSigned;
use reth_stateless::UncompressedPublicKey;

use crate::guest::{StatelessValidatorRethGuest, StatelessValidatorRethInput};

#[rustfmt::skip]
pub use stateless_validator_common::host::StatelessInput;

impl StatelessValidatorRethInput {
    /// Construct [`StatelessValidatorRethInput`] given [`StatelessInput`].
    pub fn new(stateless_input: &StatelessInput) -> anyhow::Result<Self> {
        let signers = recover_signers(&stateless_input.block.body.transactions)?;

        Ok(Self {
            stateless_input: stateless_input.clone(),
            public_keys: signers,
        })
    }

    /// Returns [`Input`] to [`zkVM`] methods.
    ///
    /// [`zkVM`]: ere_zkvm_interface::zkVM
    pub fn to_zkvm_input(&self) -> anyhow::Result<Input> {
        let stdin = GuestIo::<StatelessValidatorRethGuest>::serialize_input(self)?;
        Ok(Input::new().with_prefixed_stdin(stdin))
    }
}

/// Recover public keys from transaction signatures.
pub fn recover_signers<'a, I>(txs: I) -> anyhow::Result<Vec<UncompressedPublicKey>>
where
    I: IntoIterator<Item = &'a TransactionSigned>,
{
    txs.into_iter()
        .enumerate()
        .map(|(i, tx)| {
            tx.signature()
                .recover_from_prehash(&tx.signature_hash())
                .map(|key| key.to_encoded_point(false).as_bytes().try_into().unwrap())
                .map(UncompressedPublicKey)
                .with_context(|| format!("failed to recover signature for tx #{i}"))
        })
        .collect()
}

#[cfg(test)]
mod test {
    use crate::guest::{Io, StatelessValidatorOutput, StatelessValidatorRethIo};

    #[test]
    fn serialize_output() {
        for output in [
            StatelessValidatorOutput::new([0x00; 32], [0x00; 32], false),
            StatelessValidatorOutput::new([0xff; 32], [0xff; 32], true),
        ] {
            assert_eq!(
                StatelessValidatorRethIo::serialize_output(&output).unwrap(),
                output.serialize()
            );
        }
    }
}
