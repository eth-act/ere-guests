//! Shared EIP-8025 recursive execution-proof guest state machine.
//!
//! The consensus types implemented here follow Lighthouse's `unstable` branch at
//! commit `e6a90c168436d8b8d6b5c779c9b0550bd56fb8c7`. Proof-system verification and
//! client-specific stateless execution enter through narrow traits so both guest
//! implementations use the same authenticated beacon-lineage logic.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

use libssz_derive::{HashTreeRoot, SszDecode, SszEncode};
use libssz_merkle::{HashTreeRoot, Sha256Hasher, hash_nodes, mix_in_active_fields};
use libssz_types::SszList;
use stateless_validator_common::guest::input::{
    ChainConfig, ExecutionWitness, MAX_PUBLIC_KEYS, PUBLIC_KEY_BYTES,
    new_payload_request::{
        ExecutionPayloadV4, ExecutionRequestsGloas, NewPayloadRequest, NewPayloadRequestGloas,
        VersionedHashes,
    },
};
use stateless_validator_common::guest::{StatelessInput, StatelessValidationResult};
use stateless_validator_common::{ProgressiveList, merkleize_progressive};

/// A 32-byte SSZ Merkle node or consensus root.
pub type Root = [u8; 32];
/// A KZG commitment carried by an execution payload bid.
pub type KzgCommitment = [u8; 48];
/// An unbounded progressive list of KZG commitments carried by one Gloas block.
pub type BlobKzgCommitments = ProgressiveList<KzgCommitment>;

/// Maximum opaque execution-proof size from EIP-8025 (4 MiB).
pub const MAX_PROOF_SIZE: usize = 4 * 1024 * 1024;
/// Generalized index of `BeaconBlockBody.signed_execution_payload_bid`.
pub const SIGNED_EXECUTION_PAYLOAD_BID_GINDEX: usize = 357;
/// Merkle-branch depth implied by [`SIGNED_EXECUTION_PAYLOAD_BID_GINDEX`].
pub const SIGNED_EXECUTION_PAYLOAD_BID_DEPTH: usize = 8;
/// Subtree index implied by [`SIGNED_EXECUTION_PAYLOAD_BID_GINDEX`].
pub const SIGNED_EXECUTION_PAYLOAD_BID_INDEX: usize = 101;

/// Trusted or proven point in the execution-proof beacon lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct ExecutionCheckpoint {
    /// Beacon slot of the checkpoint.
    pub slot: u64,
    /// Hash-tree root of the checkpoint beacon block header.
    pub beacon_block_root: Root,
}

/// Public commitment made by a recursive execution proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct PublicInput {
    /// Immutable trusted checkpoint from which recursion began.
    pub origin: ExecutionCheckpoint,
    /// Latest full beacon block proven by this recursive step.
    pub head: ExecutionCheckpoint,
}

/// Opaque prior execution proof and the public input it commits to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionProof {
    /// Proof-system-specific proof bytes.
    pub proof_data: Vec<u8>,
    /// Proof-system-specific identifier which the verifier binds to this guest.
    pub proof_type: u8,
    /// Public input committed by the proof.
    pub public_input: PublicInput,
}

/// Minimal consensus beacon block header used by the guest.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct BeaconBlockHeader {
    /// Beacon slot.
    pub slot: u64,
    /// Proposer validator index.
    pub proposer_index: u64,
    /// Parent beacon block root.
    pub parent_root: Root,
    /// Post-state root.
    pub state_root: Root,
    /// Beacon block body root.
    pub body_root: Root,
}

/// Gloas execution payload bid committed by a beacon block.
#[derive(Debug, Clone, Default, PartialEq, Eq, SszEncode, SszDecode)]
pub struct ExecutionPayloadBid {
    /// Parent execution block hash.
    pub parent_block_hash: Root,
    /// Parent beacon block root.
    pub parent_block_root: Root,
    /// Proposed execution block hash.
    pub block_hash: Root,
    /// Payload randomness value.
    pub prev_randao: Root,
    /// Fee recipient.
    pub fee_recipient: [u8; 20],
    /// Payload gas limit.
    pub gas_limit: u64,
    /// Builder validator index.
    pub builder_index: u64,
    /// Beacon slot of the bid.
    pub slot: u64,
    /// Bid value.
    pub value: u64,
    /// Execution payment.
    pub execution_payment: u64,
    /// Blob commitments included by the payload.
    pub blob_kzg_commitments: BlobKzgCommitments,
    /// Progressive-container root of execution requests.
    pub execution_requests_root: Root,
}

impl HashTreeRoot for ExecutionPayloadBid {
    fn hash_tree_root(&self, hasher: &impl Sha256Hasher) -> Root {
        progressive_container_root(
            hasher,
            &[
                self.parent_block_hash.hash_tree_root(hasher),
                self.parent_block_root.hash_tree_root(hasher),
                self.block_hash.hash_tree_root(hasher),
                self.prev_randao.hash_tree_root(hasher),
                self.fee_recipient.hash_tree_root(hasher),
                self.gas_limit.hash_tree_root(hasher),
                self.builder_index.hash_tree_root(hasher),
                self.slot.hash_tree_root(hasher),
                self.value.hash_tree_root(hasher),
                self.execution_payment.hash_tree_root(hasher),
                self.blob_kzg_commitments.hash_tree_root(hasher),
                self.execution_requests_root.hash_tree_root(hasher),
            ],
        )
    }
}

/// Signed Gloas execution payload bid.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct SignedExecutionPayloadBid {
    /// Bid message authenticated through the beacon block body root.
    pub message: ExecutionPayloadBid,
    /// BLS signature. See the crate-level signature-boundary documentation.
    pub signature: [u8; 96],
}

/// Merkle proof that a signed bid is contained in a beacon block body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeaconBlockBidWitness {
    /// Header whose body commits to `signed_bid`.
    pub header: BeaconBlockHeader,
    /// Complete signed bid leaf.
    pub signed_bid: SignedExecutionPayloadBid,
    /// Bottom-up branch at generalized index 357.
    pub signed_bid_merkle_witness: [Root; SIGNED_EXECUTION_PAYLOAD_BID_DEPTH],
}

/// Gloas execution payload envelope message.
#[derive(Debug, Clone, Default, PartialEq, Eq, SszEncode, SszDecode)]
pub struct ExecutionPayloadEnvelope {
    /// Execution payload to validate.
    pub payload: ExecutionPayloadV4,
    /// Execution-layer requests returned by the payload.
    pub execution_requests: ExecutionRequestsGloas,
    /// Builder that authored the payload.
    pub builder_index: u64,
    /// Beacon block root receiving the payload.
    pub beacon_block_root: Root,
    /// Parent beacon block root supplied to `newPayload`.
    pub parent_beacon_block_root: Root,
}

impl HashTreeRoot for ExecutionPayloadEnvelope {
    fn hash_tree_root(&self, hasher: &impl Sha256Hasher) -> Root {
        progressive_container_root(
            hasher,
            &[
                self.payload.hash_tree_root(hasher),
                self.execution_requests.hash_tree_root(hasher),
                self.builder_index.hash_tree_root(hasher),
                self.beacon_block_root.hash_tree_root(hasher),
                self.parent_beacon_block_root.hash_tree_root(hasher),
            ],
        )
    }
}

/// Signed Gloas execution payload envelope.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct SignedExecutionPayloadEnvelope {
    /// Envelope message bound to the target bid.
    pub message: ExecutionPayloadEnvelope,
    /// BLS signature, intentionally not verified by this state machine.
    pub signature: [u8; 96],
}

/// Consensus-side witness connecting a prior checkpoint to a target payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeaconChainWitness {
    /// Trusted checkpoint for a base proof; mutually exclusive with `previous_proof`.
    pub origin: Option<ExecutionCheckpoint>,
    /// Prior recursive proof; mutually exclusive with `origin`.
    pub previous_proof: Option<ExecutionProof>,
    /// Checkpoint, empty-block lineage, and target block witnesses.
    pub beacon_lineage: Vec<BeaconBlockBidWitness>,
    /// Payload envelope for the final target block.
    pub signed_envelope: SignedExecutionPayloadEnvelope,
}

/// Complete implementation-level private input to one recursive guest step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateInput {
    /// Authenticated beacon lineage and target envelope.
    pub beacon_chain_witness: BeaconChainWitness,
    /// Execution witness consumed by the client stateless validator.
    pub execution_witness: ExecutionWitness,
    /// Execution chain configuration.
    pub chain_config: ChainConfig,
    /// Transaction public keys in payload order.
    pub public_keys: SszList<[u8; PUBLIC_KEY_BYTES], MAX_PUBLIC_KEYS>,
}

/// Proof-system boundary for verifying the preceding recursive proof.
pub trait ExecutionProofVerifier {
    /// Verifies the proof and its committed public input.
    ///
    /// Implementations must return `true` only when `proof_type` is bound to
    /// this exact recursive guest program.
    fn verify_execution_proof(&self, proof: &ExecutionProof) -> bool;
}

/// Client boundary for executing one canonical stateless payload transition.
pub trait StatelessNewPayloadVerifier {
    /// Executes the supplied canonical stateless input and returns its typed result.
    fn verify_stateless_new_payload(&self, input: StatelessInput) -> StatelessValidationResult;
}

/// Failure of an EIP-8025 recursive guest invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// `origin` and `previous_proof` were both present or both absent.
    InvalidProofStart,
    /// Opaque proof bytes exceed [`MAX_PROOF_SIZE`].
    PreviousProofTooLarge,
    /// The proof-system adapter rejected the prior proof.
    InvalidPreviousProof,
    /// The lineage does not contain both checkpoint and target witnesses.
    LineageTooShort,
    /// A bid slot does not equal the containing header slot.
    BidSlotMismatch,
    /// A bid's parent block root does not equal the header parent root.
    BidParentRootMismatch,
    /// A signed bid Merkle proof does not open to the header body root.
    InvalidBidMerkleProof,
    /// The first lineage witness does not equal the selected previous head.
    CheckpointMismatch,
    /// An included beacon block does not strictly advance the previous slot.
    SlotNotIncreasing,
    /// The beacon block ancestry is discontinuous.
    BeaconAncestryMismatch,
    /// A bid does not continue from the predecessor proof's execution head.
    ExecutionParentMismatch,
    /// An intermediate produced beacon block contained a full execution payload.
    NonEmptyIntermediateBlock,
    /// The envelope's target beacon block root is incorrect.
    EnvelopeBeaconRootMismatch,
    /// The envelope's parent beacon block root is incorrect.
    EnvelopeParentRootMismatch,
    /// The envelope and target bid name different builders.
    EnvelopeBuilderMismatch,
    /// The payload and target bid have different block hashes.
    PayloadBlockHashMismatch,
    /// The payload and target bid have different randomness values.
    PayloadPrevRandaoMismatch,
    /// The payload and target bid have different gas limits.
    PayloadGasLimitMismatch,
    /// The envelope execution requests do not match the target bid commitment.
    ExecutionRequestsRootMismatch,
    /// The payload slot does not equal the target header slot.
    PayloadSlotMismatch,
    /// The payload parent hash does not continue from the previous execution head.
    PayloadParentHashMismatch,
    /// Blob commitments could not fit the identically bounded versioned-hash list.
    VersionedHashesOverflow,
    /// The stateless validator returned a result for a different request.
    StatelessRequestRootMismatch,
    /// Stateless execution rejected the reconstructed payload transition.
    StatelessValidationFailed,
    /// The stateless validator did not echo the supplied chain configuration.
    StatelessChainConfigMismatch,
}

/// Authenticates a complete signed bid against its containing beacon header.
pub fn verify_beacon_block_bid_witness(
    witness: &BeaconBlockBidWitness,
    hasher: &impl Sha256Hasher,
) -> Result<(), Error> {
    if witness.signed_bid.message.slot != witness.header.slot {
        return Err(Error::BidSlotMismatch);
    }
    if witness.signed_bid.message.parent_block_root != witness.header.parent_root {
        return Err(Error::BidParentRootMismatch);
    }

    let mut root = witness.signed_bid.hash_tree_root(hasher);
    for (level, sibling) in witness.signed_bid_merkle_witness.iter().enumerate() {
        root = if ((SIGNED_EXECUTION_PAYLOAD_BID_INDEX >> level) & 1) == 0 {
            hash_nodes(hasher, &root, sibling)
        } else {
            hash_nodes(hasher, sibling, &root)
        };
    }
    if root != witness.header.body_root {
        return Err(Error::InvalidBidMerkleProof);
    }
    Ok(())
}

/// Processes one base or recursive EIP-8025 guest step.
pub fn process_private_input(
    proof_verifier: &impl ExecutionProofVerifier,
    stateless_verifier: &impl StatelessNewPayloadVerifier,
    private_input: PrivateInput,
    hasher: &impl Sha256Hasher,
) -> Result<PublicInput, Error> {
    let PrivateInput {
        beacon_chain_witness,
        execution_witness,
        chain_config,
        public_keys,
    } = private_input;
    let BeaconChainWitness {
        origin,
        previous_proof,
        beacon_lineage,
        signed_envelope,
    } = beacon_chain_witness;

    let (origin, previous_head) = match (origin, previous_proof) {
        (Some(origin), None) => (origin, origin),
        (None, Some(previous_proof)) => {
            if previous_proof.proof_data.len() > MAX_PROOF_SIZE {
                return Err(Error::PreviousProofTooLarge);
            }
            if !proof_verifier.verify_execution_proof(&previous_proof) {
                return Err(Error::InvalidPreviousProof);
            }
            (
                previous_proof.public_input.origin,
                previous_proof.public_input.head,
            )
        }
        _ => return Err(Error::InvalidProofStart),
    };

    if beacon_lineage.len() < 2 {
        return Err(Error::LineageTooShort);
    }

    let checkpoint_witness = &beacon_lineage[0];
    verify_beacon_block_bid_witness(checkpoint_witness, hasher)?;
    if checkpoint_witness.header.slot != previous_head.slot
        || checkpoint_witness.header.hash_tree_root(hasher) != previous_head.beacon_block_root
    {
        return Err(Error::CheckpointMismatch);
    }

    let mut parent_beacon_block_root = previous_head.beacon_block_root;
    let mut previous_slot = previous_head.slot;
    let parent_execution_block_hash = checkpoint_witness.signed_bid.message.block_hash;
    let mut previous_bid: Option<&ExecutionPayloadBid> = None;

    for witness in &beacon_lineage[1..] {
        if witness.header.slot <= previous_slot {
            return Err(Error::SlotNotIncreasing);
        }
        if witness.header.parent_root != parent_beacon_block_root {
            return Err(Error::BeaconAncestryMismatch);
        }
        verify_beacon_block_bid_witness(witness, hasher)?;

        let bid = &witness.signed_bid.message;
        if bid.parent_block_hash != parent_execution_block_hash {
            return Err(Error::ExecutionParentMismatch);
        }
        if previous_bid.is_some_and(|previous| bid.parent_block_hash == previous.block_hash) {
            return Err(Error::NonEmptyIntermediateBlock);
        }

        parent_beacon_block_root = witness.header.hash_tree_root(hasher);
        previous_slot = witness.header.slot;
        previous_bid = Some(bid);
    }

    let Some(target) = beacon_lineage.last() else {
        return Err(Error::LineageTooShort);
    };
    let target_header = &target.header;
    let target_bid = &target.signed_bid.message;
    let envelope = signed_envelope.message;
    let payload = &envelope.payload;
    let target_beacon_block_root = target_header.hash_tree_root(hasher);

    if envelope.beacon_block_root != target_beacon_block_root {
        return Err(Error::EnvelopeBeaconRootMismatch);
    }
    if envelope.parent_beacon_block_root != target_header.parent_root {
        return Err(Error::EnvelopeParentRootMismatch);
    }
    if envelope.builder_index != target_bid.builder_index {
        return Err(Error::EnvelopeBuilderMismatch);
    }
    if payload.block_hash != target_bid.block_hash {
        return Err(Error::PayloadBlockHashMismatch);
    }
    if payload.prev_randao != target_bid.prev_randao {
        return Err(Error::PayloadPrevRandaoMismatch);
    }
    if payload.gas_limit != target_bid.gas_limit {
        return Err(Error::PayloadGasLimitMismatch);
    }
    if execution_requests_root(&envelope.execution_requests, hasher)
        != target_bid.execution_requests_root
    {
        return Err(Error::ExecutionRequestsRootMismatch);
    }
    if payload.slot_number != target_header.slot {
        return Err(Error::PayloadSlotMismatch);
    }
    if payload.parent_hash != parent_execution_block_hash {
        return Err(Error::PayloadParentHashMismatch);
    }

    let versioned_hashes = VersionedHashes::try_from(
        target_bid
            .blob_kzg_commitments
            .iter()
            .map(|commitment| kzg_commitment_to_versioned_hash(commitment, hasher))
            .collect::<Vec<_>>(),
    )
    .map_err(|_| Error::VersionedHashesOverflow)?;
    let new_payload_request = NewPayloadRequest::Gloas(NewPayloadRequestGloas {
        execution_payload: envelope.payload,
        versioned_hashes,
        parent_beacon_block_root: envelope.parent_beacon_block_root,
        execution_requests: envelope.execution_requests,
    });
    let expected_request_root = new_payload_request.hash_tree_root(hasher);
    let result = stateless_verifier.verify_stateless_new_payload(StatelessInput {
        new_payload_request,
        witness: execution_witness,
        chain_config: chain_config.clone(),
        public_keys,
    });

    if result.new_payload_request_root != expected_request_root {
        return Err(Error::StatelessRequestRootMismatch);
    }
    if !result.successful_validation {
        return Err(Error::StatelessValidationFailed);
    }
    if result.chain_config != chain_config {
        return Err(Error::StatelessChainConfigMismatch);
    }

    Ok(PublicInput {
        origin,
        head: ExecutionCheckpoint {
            slot: target_header.slot,
            beacon_block_root: target_beacon_block_root,
        },
    })
}

/// Computes the Gloas progressive-container root of execution requests.
pub fn execution_requests_root(
    requests: &ExecutionRequestsGloas,
    hasher: &impl Sha256Hasher,
) -> Root {
    progressive_container_root(
        hasher,
        &[
            requests.deposits.hash_tree_root(hasher),
            requests.withdrawals.hash_tree_root(hasher),
            requests.consolidations.hash_tree_root(hasher),
            requests.builder_deposits.hash_tree_root(hasher),
            requests.builder_exits.hash_tree_root(hasher),
        ],
    )
}

/// Computes an EIP-4844 versioned hash from a KZG commitment.
pub fn kzg_commitment_to_versioned_hash(
    commitment: &KzgCommitment,
    hasher: &impl Sha256Hasher,
) -> Root {
    let mut versioned_hash = hasher.hash(commitment);
    versioned_hash[0] = 1;
    versioned_hash
}

fn progressive_container_root(hasher: &impl Sha256Hasher, field_roots: &[Root]) -> Root {
    let root = merkleize_progressive(hasher, field_roots);
    mix_in_active_fields(hasher, &root, &alloc::vec![true; field_roots.len()])
}

#[cfg(test)]
mod tests;
