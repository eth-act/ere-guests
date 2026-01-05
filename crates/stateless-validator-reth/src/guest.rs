//! [`Guest`] implementation for Reth stateless validator.

use alloc::{format, sync::Arc, vec::Vec};

use ere_io::serde::{IoSerde, bincode::BincodeLegacy};
use reth_chainspec::ChainSpec;
use reth_evm_ethereum::EthEvmConfig;
use reth_primitives_traits::Block;
use reth_stateless::{
    Genesis, StatelessInput, UncompressedPublicKey, stateless_validation_with_trie,
};
use serde::{Deserialize, Serialize};
use sparsestate::SparseState;

#[rustfmt::skip]
pub use {
    guest::*,
    stateless_validator_common::guest::StatelessValidatorOutput,
};

/// Input for the stateless validator guest program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatelessValidatorRethInput {
    /// The stateless input for the stateless validation function.
    pub stateless_input: StatelessInput,
    /// The recovered signers for the transactions in the block.
    pub public_keys: Vec<UncompressedPublicKey>,
}

/// [`Io`] implementation of Reth stateless validator.
pub type StatelessValidatorRethIo =
    IoSerde<StatelessValidatorRethInput, StatelessValidatorOutput, BincodeLegacy>;

/// [`Guest`] implementation for Reth stateless validator.
#[derive(Debug, Clone)]
pub struct StatelessValidatorRethGuest;

impl Guest for StatelessValidatorRethGuest {
    type Io = StatelessValidatorRethIo;

    fn compute<P: Platform>(input: GuestInput<Self>) -> GuestOutput<Self> {
        let genesis = Genesis {
            config: input.stateless_input.chain_config.clone(),
            ..Default::default()
        };
        let chain_spec: Arc<ChainSpec> = Arc::new(genesis.into());
        let evm_config = EthEvmConfig::new(chain_spec.clone());

        let (header, parent_hash) = P::cycle_scope("public_inputs_preparation", || {
            (
                input.stateless_input.block.header().clone(),
                input.stateless_input.block.parent_hash,
            )
        });

        let res = P::cycle_scope("validation", || {
            stateless_validation_with_trie::<SparseState, _, _>(
                input.stateless_input.block,
                input.public_keys,
                input.stateless_input.witness,
                chain_spec,
                evm_config,
            )
            .map(|(block_hash, _)| block_hash)
        });

        match res {
            Ok(block_hash) => StatelessValidatorOutput::new(block_hash, parent_hash, true),
            Err(err) => {
                P::print(&format!("Block validation failed: {err}\n"));
                StatelessValidatorOutput::new(header.hash_slow(), parent_hash, false)
            }
        }
    }
}
