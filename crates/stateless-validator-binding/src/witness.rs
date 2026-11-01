//! Beacon state witness: the state fields the prologue reads, each proven against the state root.
//!
//! The Gloas `BeaconState` is a `ProgressiveContainer` with 46 active fields, so its root is
//! `hash(merkleize_progressive(field_roots), pack_bits(active_fields))`. The progressive spine
//! roots its subtrees at generalized indices 4, 10, 22 and 46, covering chunks `0`, `1..5`,
//! `5..21` and `21..85`, so a field's generalized index is
//! `subtree_root_gindex * subtree_capacity + offset_in_subtree`. The indices are stable across
//! forks by EIP-7495, so appending a state field changes the branch contents but not their shape.

#![allow(missing_docs)]

#[cfg(any(test, feature = "host"))]
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use libssz_derive::{SszDecode, SszEncode};
#[cfg(any(test, feature = "host"))]
use libssz_merkle::ZERO_HASHES;
use libssz_merkle::{HashTreeRoot, Node, Sha256Hasher, hash_nodes};
use libssz_types::ProgressiveList;
use stateless_validator_common::guest::input::new_payload_request::Hash32;

use crate::{
    error::Error,
    types::{BeaconBlockHeader, ExecutionPayloadBid, Root},
};

/// Merkle branch of a single state field, ordered from the leaf's sibling upwards.
pub type Branch = ProgressiveList<Node>;

/// Generalized indices of the read state fields, which are state fields 0, 2, 4, 24, 43 and 44.
pub const GINDEX_GENESIS_TIME: u64 = 4;
pub const GINDEX_SLOT: u64 = 41;
pub const GINDEX_LATEST_BLOCK_HEADER: u64 = 43;
pub const GINDEX_LATEST_BLOCK_HASH: u64 = 2947;
pub const GINDEX_LATEST_EXECUTION_PAYLOAD_BID: u64 = 2966;
pub const GINDEX_PAYLOAD_EXPECTED_WITHDRAWALS: u64 = 2967;

pub fn gindex_of_field(chunk: usize) -> u64 {
    let mut start = 0;
    let mut capacity = 1;
    let mut subtree_root = 4;
    while chunk >= start + capacity {
        start += capacity;
        capacity *= 4;
        subtree_root = 2 * subtree_root + 2;
    }
    subtree_root * capacity as u64 + (chunk - start) as u64
}

#[derive(Debug, Clone, PartialEq, Eq, SszEncode, SszDecode)]
pub struct StateBranches {
    pub genesis_time: Branch,
    pub slot: Branch,
    pub latest_block_header: Branch,
    pub latest_block_hash: Branch,
    pub latest_execution_payload_bid: Branch,
    pub payload_expected_withdrawals: Branch,
}

/// `state_root` is not trusted on its own: the prologue recomputes the beacon block root from it
/// and the block header, and the result must equal the envelope's `beacon_block_root`, which the
/// public input commits to.
#[derive(Debug, Clone, PartialEq, Eq, SszEncode, SszDecode)]
pub struct BeaconStateWitness {
    pub state_root: Root,
    pub genesis_time: u64,
    pub slot: u64,
    /// Its own `state_root` is zero, as the state holds it.
    pub latest_block_header: BeaconBlockHeader,
    pub latest_block_hash: Hash32,
    pub latest_execution_payload_bid: ExecutionPayloadBid,
    pub payload_expected_withdrawals_root: Root,
    pub branches: StateBranches,
}

impl BeaconStateWitness {
    pub fn verify(&self, hasher: &impl Sha256Hasher) -> Result<(), Error> {
        let fields = [
            (
                GINDEX_GENESIS_TIME,
                self.genesis_time.hash_tree_root(hasher),
                &self.branches.genesis_time,
            ),
            (
                GINDEX_SLOT,
                self.slot.hash_tree_root(hasher),
                &self.branches.slot,
            ),
            (
                GINDEX_LATEST_BLOCK_HEADER,
                self.latest_block_header.hash_tree_root(hasher),
                &self.branches.latest_block_header,
            ),
            (
                GINDEX_LATEST_BLOCK_HASH,
                self.latest_block_hash.hash_tree_root(hasher),
                &self.branches.latest_block_hash,
            ),
            (
                GINDEX_LATEST_EXECUTION_PAYLOAD_BID,
                self.latest_execution_payload_bid.hash_tree_root(hasher),
                &self.branches.latest_execution_payload_bid,
            ),
            (
                GINDEX_PAYLOAD_EXPECTED_WITHDRAWALS,
                self.payload_expected_withdrawals_root,
                &self.branches.payload_expected_withdrawals,
            ),
        ];
        for (gindex, leaf, branch) in fields {
            if !verify_branch(hasher, leaf, branch, gindex, self.state_root) {
                return Err(Error::StateFieldBranch(gindex));
            }
        }
        Ok(())
    }
}

fn verify_branch(
    hasher: &impl Sha256Hasher,
    leaf: Node,
    branch: &[Node],
    gindex: u64,
    root: Root,
) -> bool {
    if branch.len() != gindex.ilog2() as usize {
        return false;
    }
    let mut node = leaf;
    let mut index = gindex;
    for sibling in branch {
        node = if index.is_multiple_of(2) {
            hash_nodes(hasher, &node, sibling)
        } else {
            hash_nodes(hasher, sibling, &node)
        };
        index /= 2;
    }
    node == root
}

/// Every state field the prologue does not read is set to `filler`, so the branches are the only
/// thing tying the read fields to the returned `state_root`. A prover builds the same witness from
/// a real state, where the unread fields hold their real roots.
#[cfg(any(test, feature = "host"))]
pub fn synthesize_witness(
    hasher: &impl Sha256Hasher,
    fields: StateFields,
    filler: Node,
) -> BeaconStateWitness {
    let mut chunks = [filler; STATE_FIELD_COUNT];
    chunks[0] = fields.genesis_time.hash_tree_root(hasher);
    chunks[2] = fields.slot.hash_tree_root(hasher);
    chunks[4] = fields.latest_block_header.hash_tree_root(hasher);
    chunks[24] = fields.latest_block_hash;
    chunks[43] = fields.latest_execution_payload_bid.hash_tree_root(hasher);
    chunks[44] = fields.payload_expected_withdrawals_root;

    let tree = state_tree(hasher, &chunks);
    let branch = |chunk: usize| -> Branch {
        let mut nodes = Vec::new();
        let mut gindex = gindex_of_field(chunk);
        while gindex > 1 {
            nodes.push(tree[&(gindex ^ 1)]);
            gindex /= 2;
        }
        nodes.into()
    };

    BeaconStateWitness {
        state_root: tree[&1],
        genesis_time: fields.genesis_time,
        slot: fields.slot,
        latest_block_header: fields.latest_block_header,
        latest_block_hash: fields.latest_block_hash,
        latest_execution_payload_bid: fields.latest_execution_payload_bid,
        payload_expected_withdrawals_root: fields.payload_expected_withdrawals_root,
        branches: StateBranches {
            genesis_time: branch(0),
            slot: branch(2),
            latest_block_header: branch(4),
            latest_block_hash: branch(24),
            latest_execution_payload_bid: branch(43),
            payload_expected_withdrawals: branch(44),
        },
    }
}

#[cfg(any(test, feature = "host"))]
pub const STATE_FIELD_COUNT: usize = 46;

#[cfg(any(test, feature = "host"))]
#[derive(Debug, Clone)]
pub struct StateFields {
    pub genesis_time: u64,
    pub slot: u64,
    /// Its own `state_root` is zero, as the state holds it.
    pub latest_block_header: BeaconBlockHeader,
    pub latest_block_hash: Hash32,
    pub latest_execution_payload_bid: ExecutionPayloadBid,
    pub payload_expected_withdrawals_root: Root,
}

/// Every node of the state tree, keyed by generalized index.
#[cfg(any(test, feature = "host"))]
fn state_tree(hasher: &impl Sha256Hasher, chunks: &[Node]) -> BTreeMap<u64, Node> {
    let mut tree = BTreeMap::new();
    let progressive_root = progressive_subtree(hasher, &mut tree, chunks, 1, 2);
    let active_fields = active_fields_node(chunks.len());
    tree.insert(3, active_fields);
    tree.insert(1, hash_nodes(hasher, &progressive_root, &active_fields));
    tree
}

/// Fills the progressive spine node at `gindex`: a balanced subtree of `capacity` leaves on the
/// left, the remaining chunks on the right, terminated by a zero node.
#[cfg(any(test, feature = "host"))]
fn progressive_subtree(
    hasher: &impl Sha256Hasher,
    tree: &mut BTreeMap<u64, Node>,
    chunks: &[Node],
    capacity: usize,
    gindex: u64,
) -> Node {
    if chunks.is_empty() {
        tree.insert(gindex, ZERO_HASHES[0]);
        return ZERO_HASHES[0];
    }
    let take = core::cmp::min(capacity, chunks.len());
    let node = hash_nodes(
        hasher,
        &balanced_subtree(hasher, tree, &chunks[..take], capacity, 2 * gindex),
        &progressive_subtree(hasher, tree, &chunks[take..], capacity * 4, 2 * gindex + 1),
    );
    tree.insert(gindex, node);
    node
}

#[cfg(any(test, feature = "host"))]
fn balanced_subtree(
    hasher: &impl Sha256Hasher,
    tree: &mut BTreeMap<u64, Node>,
    leaves: &[Node],
    capacity: usize,
    gindex: u64,
) -> Node {
    let mut layer = leaves.to_vec();
    layer.resize(capacity, ZERO_HASHES[0]);
    let mut first = gindex * capacity as u64;
    loop {
        for (offset, node) in layer.iter().enumerate() {
            tree.insert(first + offset as u64, *node);
        }
        if let [root] = layer[..] {
            return root;
        }
        layer = layer
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| hash_nodes(hasher, &pair[0], &pair[1]))
            .collect();
        first /= 2;
    }
}

#[cfg(any(test, feature = "host"))]
fn active_fields_node(fields: usize) -> Node {
    let mut node = [0u8; 32];
    for field in 0..fields {
        node[field / 8] |= 1 << (field % 8);
    }
    node
}

#[cfg(test)]
mod tests {
    use libssz_merkle::{Sha2Hasher, merkleize_progressive, mix_in_active_fields};

    use super::*;

    #[test]
    fn state_tree_matches_progressive_merkleization() {
        let chunks = (0..STATE_FIELD_COUNT as u8)
            .map(|field| [field; 32])
            .collect::<Vec<_>>();
        let tree = state_tree(&Sha2Hasher, &chunks);

        assert_eq!(
            tree[&1],
            mix_in_active_fields(
                &Sha2Hasher,
                &merkleize_progressive(&Sha2Hasher, &chunks),
                &[true; STATE_FIELD_COUNT]
            )
        );
        for (field, chunk) in chunks.iter().enumerate() {
            assert_eq!(tree[&gindex_of_field(field)], *chunk, "field {field}");
        }
        for (gindex, field) in [
            (GINDEX_GENESIS_TIME, 0),
            (GINDEX_SLOT, 2),
            (GINDEX_LATEST_BLOCK_HEADER, 4),
            (GINDEX_LATEST_BLOCK_HASH, 24),
            (GINDEX_LATEST_EXECUTION_PAYLOAD_BID, 43),
            (GINDEX_PAYLOAD_EXPECTED_WITHDRAWALS, 44),
        ] {
            assert_eq!(gindex, gindex_of_field(field));
        }
    }
}
