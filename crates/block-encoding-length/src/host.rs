//! Implementations for host environment.

use ere_prover_core::{Input, codec::Encode};
use reth_ethereum_primitives::Block;

use crate::guest::{BincodeBlock, BlockEncodingFormat, BlockEncodingLengthInput};

impl BlockEncodingLengthInput {
    /// Construct [`BlockEncodingLengthInput`] given block, loop count and the
    /// encoding format.
    pub fn new(
        block: &Block,
        loop_count: u16,
        format: BlockEncodingFormat,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            block: BincodeBlock(block.clone()),
            loop_count,
            format,
        })
    }

    /// Returns [`Input`] to [`zkVMProver`] methods.
    ///
    /// [`zkVMProver`]: ere_prover_core::zkVMProver
    pub fn to_zkvm_input(&self) -> anyhow::Result<Input> {
        Ok(Input::new().with_prefixed_stdin(self.encode_to_vec()?))
    }
}
