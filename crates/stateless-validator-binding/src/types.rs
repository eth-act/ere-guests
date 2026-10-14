//! Gloas consensus containers the prologue hashes or decodes. The execution payload and the
//! execution requests are reused from [`stateless_validator_common`], since the prologue hands
//! them to the execution engine unchanged.

#![allow(missing_docs)]

use alloc::vec::Vec;

use libssz_derive::{HashTreeRoot, SszDecode, SszEncode};
use libssz_types::ProgressiveList;
use stateless_validator_common::guest::input::new_payload_request::{
    Address, Bytes48, ExecutionPayloadV4, ExecutionRequestsGloas, Hash32,
};

pub type Root = [u8; 32];
pub type BlobKZGCommitments = ProgressiveList<Bytes48>;

/// An EIP-7688 immutable type: it keeps plain container merkleization.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct BeaconBlockHeader {
    pub slot: u64,
    pub proposer_index: u64,
    pub parent_root: Root,
    pub state_root: Root,
    pub body_root: Root,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
#[ssz(progressive_container)]
pub struct ExecutionPayloadBid {
    pub parent_block_hash: Hash32,
    pub parent_block_root: Root,
    pub block_hash: Hash32,
    pub prev_randao: Hash32,
    pub fee_recipient: Address,
    pub gas_limit: u64,
    pub builder_index: u64,
    pub slot: u64,
    pub value: u64,
    pub execution_payment: u64,
    pub blob_kzg_commitments: BlobKZGCommitments,
    pub execution_requests_root: Root,
}

/// The unsigned envelope: the builder signature authenticates the sender of a gossip message,
/// while the payload itself is bound to the beacon block through the bid.
#[derive(Debug, Clone, PartialEq, Eq, SszEncode, SszDecode)]
#[ssz(progressive_container)]
pub struct ExecutionPayloadEnvelope {
    pub payload: ExecutionPayloadV4,
    pub execution_requests: ExecutionRequestsGloas,
    pub builder_index: u64,
    pub beacon_block_root: Root,
    pub parent_beacon_block_root: Root,
}

/// The EIP-8025 public input. The proof system takes `hash_tree_root(public_input)`, so a verifier
/// reconstructs this container from the proven beacon block root and the two constants.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
#[ssz(progressive_container)]
pub struct PublicInput {
    pub beacon_block_root: Root,
    /// Always `true`: a proof exists only for a successful validation.
    pub successful_validation: bool,
    pub chain_id: u64,
    pub schema_id: u16,
}
