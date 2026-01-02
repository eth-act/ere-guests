//! Stateless validator guest program.

use alloc::format;
use core::fmt::Debug;

use ere_io::{
    Io,
    rkyv::{
        IoRkyv,
        rkyv::{Archive, Deserialize, Serialize},
    },
};
use ethrex_common::types::block_execution_witness::ExecutionWitness;
use ethrex_guest_program::{execution::execution_program, input::ProgramInput};
use guest::Platform;

#[rustfmt::skip]
pub use guest::Guest;

/// Input for the Ethrex stateless validator guest program.
#[derive(Serialize, Deserialize, Archive)]
pub struct StatelessValidatorEthrexInput(pub ProgramInput);

impl Clone for StatelessValidatorEthrexInput {
    fn clone(&self) -> Self {
        Self(ProgramInput {
            blocks: self.0.blocks.clone(),
            execution_witness: self.0.execution_witness.clone(),
            elasticity_multiplier: self.0.elasticity_multiplier,
            fee_configs: self.0.fee_configs.clone(),
            #[cfg(feature = "l2")]
            blob_commitment: self.0.blob_commitment,
            #[cfg(feature = "l2")]
            blob_proof: self.0.blob_proof,
        })
    }
}

impl Debug for StatelessValidatorEthrexInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        struct DebugExecutionWitness<'a>(&'a ExecutionWitness);

        impl Debug for DebugExecutionWitness<'_> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_struct("ExecutionWitness")
                    .field("codes", &self.0.codes)
                    .field("block_headers_bytes", &self.0.block_headers_bytes)
                    .field("first_block_number", &self.0.first_block_number)
                    .field("chain_config", &self.0.chain_config)
                    .field("state_trie_root", &self.0.state_trie_root)
                    .field("storage_trie_roots", &self.0.storage_trie_roots)
                    .field("keys", &self.0.keys)
                    .finish()
            }
        }

        f.debug_struct("StatelessValidatorEthrexInput")
            .field("blocks", &self.0.blocks)
            .field(
                "execution_witness",
                &DebugExecutionWitness(&self.0.execution_witness),
            )
            .field("elasticity_multiplier", &self.0.elasticity_multiplier)
            .field("fee_configs", &self.0.fee_configs)
            .finish()
    }
}

/// The public inputs are:
/// - `block_hash` - `[u8; 32]`
/// - `parent_hash` - `[u8; 32]`
/// - `successful_block_validation` - `bool`
pub type StatelessValidatorEthrexOutput = ([u8; 32], [u8; 32], bool);

/// [`Guest`] implementation for Ethrex stateless validator.
#[derive(Debug, Clone)]
pub struct StatelessValidatorEthrexGuest;

impl Guest for StatelessValidatorEthrexGuest {
    type Io = IoRkyv<StatelessValidatorEthrexInput, StatelessValidatorEthrexOutput>;

    fn compute<P: Platform>(
        StatelessValidatorEthrexInput(input): <Self::Io as Io>::Input,
    ) -> <Self::Io as Io>::Output {
        let (header, parent_hash) = P::cycle_scope("public_inputs_preparation", || {
            (
                input.blocks[0].header.clone(),
                input.blocks[0].header.parent_hash,
            )
        });

        if input.blocks.len() != 1 {
            return (header.compute_block_hash().0, parent_hash.0, false);
        }

        let res = P::cycle_scope("validation", || execution_program(input));

        match res {
            Ok(out) => (out.last_block_hash.0, parent_hash.0, true),
            Err(err) => {
                P::print(&format!(
                    "Block {} validation failed: {err}\n",
                    header.number
                ));
                (header.compute_block_hash().0, parent_hash.0, false)
            }
        }
    }
}
