//! Implementations for host environment.

use ere_zkvm_interface::Input;
use guest::{GuestIo, Io};
use reth_ethereum_primitives::Block;

use crate::guest::{
    BincodeBlock, BlockEncodingFormat, BlockEncodingLengthGuest, BlockEncodingLengthInput,
};

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

    /// Returns [`Input`] to [`zkVM`] methods.
    ///
    /// [`zkVM`]: ere_zkvm_interface::zkVM
    pub fn to_zkvm_input(&self) -> anyhow::Result<Input> {
        let stdin = GuestIo::<BlockEncodingLengthGuest>::serialize_input(self)?;
        Ok(Input::new().with_prefixed_stdin(stdin))
    }
}
