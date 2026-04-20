//! [`Guest`] implementation for the block encoding length calculation.

use alloc::vec::Vec;
use core::ops::Deref;

use guest::codec::impl_codec_by_bincode_legacy;
use libssz::SszEncode;
use reth_ethereum_primitives::Block;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[rustfmt::skip]
pub use guest::*;

mod block_ssz;

/// The encoding format used for the block encoding length calculation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum BlockEncodingFormat {
    /// RLP encoding format
    Rlp,
    /// SSZ encoding format
    Ssz,
}

/// Block wrapper that supports bincode serialization
#[serde_as]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BincodeBlock(
    #[serde_as(
        as = "reth_primitives_traits::serde_bincode_compat::Block<reth_ethereum_primitives::TransactionSigned, alloy_consensus::Header>"
    )]
    pub reth_ethereum_primitives::Block,
);

impl Deref for BincodeBlock {
    type Target = reth_ethereum_primitives::Block;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Input for the block encoding length calculation guest program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockEncodingLengthInput {
    /// The block to calculate the encoding length for.
    pub block: BincodeBlock,
    /// The number of times to repeat the encoding length calculation.
    pub loop_count: u16,
    /// The encoding format to use.
    pub format: BlockEncodingFormat,
}

impl_codec_by_bincode_legacy!(BlockEncodingLengthInput);

/// [`Guest`] implementation for the block encoding length calculation.
#[derive(Debug, Clone)]
pub struct BlockEncodingLengthGuest;

impl Guest for BlockEncodingLengthGuest {
    type Input = BlockEncodingLengthInput;
    type Output = ();

    fn compute<P: Platform>(input: Self::Input) -> Self::Output {
        match input.format {
            BlockEncodingFormat::Rlp => {
                P::cycle_scope("block_encoding_length_calculation", || {
                    for _ in 0..input.loop_count {
                        Block::rlp_length_for(&input.block.header, &input.block.body);
                    }
                });
            }
            BlockEncodingFormat::Ssz => {
                let block: block_ssz::Block =
                    P::cycle_scope("block_format_conversion", || input.block.0.into());

                P::cycle_scope("block_encoding_length_calculation", || {
                    for _ in 0..input.loop_count {
                        block.encoded_len();
                    }
                });
            }
        }
    }
}
