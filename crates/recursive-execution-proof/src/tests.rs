use core::cell::{Cell, RefCell};

use hex_literal::hex;
use stateless_validator_common::guest::input::{
    ChainConfig, ExecutionWitness, ForkActivation, ForkConfig,
    new_payload_request::{
        ExecutionPayloadV4, ExecutionRequestsGloas, MAX_BLOB_COMMITMENTS_PER_BLOCK,
        NewPayloadRequest,
    },
};
use stateless_validator_common::guest::{StatelessInput, StatelessValidationResult};
use stateless_validator_common::{HashTreeRoot, Sha2Hasher, SszDecode, SszEncode, SszList};

use super::*;

#[derive(Debug)]
struct ProofVerifier {
    accepted: bool,
    calls: Cell<usize>,
}

impl ExecutionProofVerifier for ProofVerifier {
    fn verify_execution_proof(&self, _proof: &ExecutionProof) -> bool {
        self.calls.set(self.calls.get() + 1);
        self.accepted
    }
}

#[derive(Debug)]
struct StatelessVerifier {
    successful: bool,
    wrong_root: bool,
    input: RefCell<Option<StatelessInput>>,
}

impl StatelessNewPayloadVerifier for StatelessVerifier {
    fn verify_stateless_new_payload(&self, input: StatelessInput) -> StatelessValidationResult {
        let root = if self.wrong_root {
            [0xff; 32]
        } else {
            input.new_payload_request.hash_tree_root(&Sha2Hasher)
        };
        let result =
            StatelessValidationResult::new(root, self.successful, input.chain_config.clone());
        *self.input.borrow_mut() = Some(input);
        result
    }
}

fn root(byte: u8) -> Root {
    [byte; 32]
}

fn payload(parent_hash: Root, block_hash: Root, slot: u64) -> ExecutionPayloadV4 {
    ExecutionPayloadV4 {
        parent_hash,
        fee_recipient: [0; 20],
        state_root: root(1),
        receipts_root: root(2),
        logs_bloom: [0; 256],
        prev_randao: root(3),
        block_number: 1,
        gas_limit: 30_000_000,
        gas_used: 21_000,
        timestamp: 1,
        extra_data: SszList::default(),
        base_fee_per_gas: root(4),
        block_hash,
        transactions: Default::default(),
        withdrawals: Default::default(),
        blob_gas_used: 0,
        excess_blob_gas: 0,
        block_access_list: Default::default(),
        slot_number: slot,
    }
}

fn bid(
    parent_block_hash: Root,
    parent_block_root: Root,
    block_hash: Root,
    slot: u64,
    requests: &ExecutionRequestsGloas,
) -> ExecutionPayloadBid {
    ExecutionPayloadBid {
        parent_block_hash,
        parent_block_root,
        block_hash,
        prev_randao: root(3),
        fee_recipient: [0; 20],
        gas_limit: 30_000_000,
        builder_index: 42,
        slot,
        value: 0,
        execution_payment: 0,
        blob_kzg_commitments: vec![[5; 48]].into(),
        execution_requests_root: execution_requests_root(requests, &Sha2Hasher),
    }
}

fn witness(
    slot: u64,
    parent_root: Root,
    bid: ExecutionPayloadBid,
    branch_byte: u8,
) -> BeaconBlockBidWitness {
    let signed_bid = SignedExecutionPayloadBid {
        message: bid,
        signature: [0; 96],
    };
    let branch = core::array::from_fn(|level| root(branch_byte.wrapping_add(level as u8)));
    let mut body_root = signed_bid.hash_tree_root(&Sha2Hasher);
    for (level, sibling) in branch.iter().enumerate() {
        body_root = if ((SIGNED_EXECUTION_PAYLOAD_BID_INDEX >> level) & 1) == 0 {
            hash_nodes(&Sha2Hasher, &body_root, sibling)
        } else {
            hash_nodes(&Sha2Hasher, sibling, &body_root)
        };
    }
    BeaconBlockBidWitness {
        header: BeaconBlockHeader {
            slot,
            proposer_index: 1,
            parent_root,
            state_root: root(8),
            body_root,
        },
        signed_bid,
        signed_bid_merkle_witness: branch,
    }
}

fn base_input() -> (PrivateInput, ExecutionCheckpoint) {
    let requests = ExecutionRequestsGloas::default();
    let checkpoint_bid = bid(root(6), root(7), root(9), 10, &requests);
    let checkpoint = witness(10, root(7), checkpoint_bid, 10);
    let checkpoint_root = checkpoint.header.hash_tree_root(&Sha2Hasher);
    let origin = ExecutionCheckpoint {
        slot: 10,
        beacon_block_root: checkpoint_root,
    };

    let target_payload = payload(root(9), root(11), 12);
    let target_bid = bid(root(9), checkpoint_root, root(11), 12, &requests);
    let target = witness(12, checkpoint_root, target_bid, 20);
    let target_root = target.header.hash_tree_root(&Sha2Hasher);

    let input = PrivateInput {
        beacon_chain_witness: BeaconChainWitness {
            origin: Some(origin),
            previous_proof: None,
            beacon_lineage: vec![checkpoint, target.clone()],
            signed_envelope: SignedExecutionPayloadEnvelope {
                message: ExecutionPayloadEnvelope {
                    payload: target_payload,
                    execution_requests: requests,
                    builder_index: 42,
                    beacon_block_root: target_root,
                    parent_beacon_block_root: checkpoint_root,
                },
                signature: [0; 96],
            },
        },
        execution_witness: ExecutionWitness::default(),
        chain_config: ChainConfig {
            chain_id: 1,
            active_fork: ForkConfig::new(ForkActivation::new(Some(0), None)),
        },
        public_keys: SszList::default(),
    };
    (input, origin)
}

fn accepting_stateless() -> StatelessVerifier {
    StatelessVerifier {
        successful: true,
        wrong_root: false,
        input: RefCell::new(None),
    }
}

#[test]
fn public_input_has_consensus_ssz_round_trip() {
    let public_input = PublicInput {
        origin: ExecutionCheckpoint {
            slot: 2,
            beacon_block_root: root(3),
        },
        head: ExecutionCheckpoint {
            slot: 10,
            beacon_block_root: root(11),
        },
    };

    assert_eq!(
        PublicInput::from_ssz_bytes(&public_input.to_ssz()).unwrap(),
        public_input
    );
}

#[test]
fn gloas_consensus_types_have_ssz_round_trips() {
    let (input, _) = base_input();
    let signed_bid = input.beacon_chain_witness.beacon_lineage[1]
        .signed_bid
        .clone();
    let signed_envelope = input.beacon_chain_witness.signed_envelope;

    assert_eq!(
        SignedExecutionPayloadBid::from_ssz_bytes(&signed_bid.to_ssz()).unwrap(),
        signed_bid
    );
    assert_eq!(
        SignedExecutionPayloadEnvelope::from_ssz_bytes(&signed_envelope.to_ssz()).unwrap(),
        signed_envelope
    );
}

#[test]
fn progressive_container_roots_match_lighthouse_unstable() {
    // Generated with Lighthouse `unstable` at
    // e6a90c168436d8b8d6b5c779c9b0550bd56fb8c7.
    let bid = ExecutionPayloadBid::default();
    assert_eq!(
        bid.hash_tree_root(&Sha2Hasher),
        hex!("83b932ee5875c06aa35328e3c3e3c976c703f2f4b1bc98e32991ceabbb2e4b63")
    );
    assert_eq!(
        ExecutionPayloadEnvelope::default().hash_tree_root(&Sha2Hasher),
        hex!("f4a18a53008d3c2c6a7c27a3ebf4c82c2c236e212be004f985f26907af0342ec")
    );
}

#[test]
fn base_proof_preserves_origin_and_advances_head() {
    let (input, origin) = base_input();
    let expected_head = input.beacon_chain_witness.beacon_lineage[1]
        .header
        .hash_tree_root(&Sha2Hasher);
    let proof = ProofVerifier {
        accepted: true,
        calls: Cell::new(0),
    };
    let stateless = accepting_stateless();

    let public_input = process_private_input(&proof, &stateless, input, &Sha2Hasher).unwrap();

    assert_eq!(public_input.origin, origin);
    assert_eq!(public_input.head.slot, 12);
    assert_eq!(public_input.head.beacon_block_root, expected_head);
    assert_eq!(proof.calls.get(), 0);

    let stateless_input = stateless.input.borrow();
    let NewPayloadRequest::Gloas(request) = &stateless_input.as_ref().unwrap().new_payload_request
    else {
        panic!("recursive processing must construct a Gloas request");
    };
    assert_eq!(request.execution_payload.parent_hash, root(9));
    assert_eq!(request.parent_beacon_block_root, origin.beacon_block_root);
    assert_eq!(request.versioned_hashes.len(), 1);
    assert_eq!(
        request.versioned_hashes[0],
        kzg_commitment_to_versioned_hash(&[5; 48], &Sha2Hasher)
    );
}

#[test]
fn recursive_proof_verifies_prior_proof_and_keeps_original_origin() {
    let (mut input, checkpoint) = base_input();
    let original_origin = ExecutionCheckpoint {
        slot: 2,
        beacon_block_root: root(99),
    };
    input.beacon_chain_witness.origin = None;
    input.beacon_chain_witness.previous_proof = Some(ExecutionProof {
        proof_data: vec![1, 2, 3],
        proof_type: 7,
        public_input: PublicInput {
            origin: original_origin,
            head: checkpoint,
        },
    });
    let proof = ProofVerifier {
        accepted: true,
        calls: Cell::new(0),
    };

    let output = process_private_input(&proof, &accepting_stateless(), input, &Sha2Hasher).unwrap();

    assert_eq!(proof.calls.get(), 1);
    assert_eq!(output.origin, original_origin);
    assert_eq!(output.head.slot, 12);
}

#[test]
fn rejects_unverified_prior_proof_before_execution() {
    let (mut input, checkpoint) = base_input();
    input.beacon_chain_witness.origin = None;
    input.beacon_chain_witness.previous_proof = Some(ExecutionProof {
        proof_data: Vec::new(),
        proof_type: 7,
        public_input: PublicInput {
            origin: checkpoint,
            head: checkpoint,
        },
    });
    let proof = ProofVerifier {
        accepted: false,
        calls: Cell::new(0),
    };
    let stateless = accepting_stateless();

    assert_eq!(
        process_private_input(&proof, &stateless, input, &Sha2Hasher),
        Err(Error::InvalidPreviousProof)
    );
    assert_eq!(proof.calls.get(), 1);
    assert!(stateless.input.borrow().is_none());
}

#[test]
fn rejects_tampered_bid_merkle_proof() {
    let (mut input, _) = base_input();
    input.beacon_chain_witness.beacon_lineage[1].signed_bid_merkle_witness[0] = root(200);

    assert_eq!(
        process_private_input(
            &ProofVerifier {
                accepted: true,
                calls: Cell::new(0),
            },
            &accepting_stateless(),
            input,
            &Sha2Hasher,
        ),
        Err(Error::InvalidBidMerkleProof)
    );
}

#[test]
fn rejects_full_intermediate_beacon_block() {
    let (mut input, _) = base_input();
    let requests = ExecutionRequestsGloas::default();
    let checkpoint_root = input.beacon_chain_witness.beacon_lineage[0]
        .header
        .hash_tree_root(&Sha2Hasher);
    let full_intermediate = witness(
        11,
        checkpoint_root,
        bid(root(9), checkpoint_root, root(9), 11, &requests),
        40,
    );
    let intermediate_root = full_intermediate.header.hash_tree_root(&Sha2Hasher);
    let target = witness(
        12,
        intermediate_root,
        bid(root(9), intermediate_root, root(11), 12, &requests),
        50,
    );
    input.beacon_chain_witness.beacon_lineage = vec![
        input.beacon_chain_witness.beacon_lineage[0].clone(),
        full_intermediate,
        target,
    ];

    assert_eq!(
        process_private_input(
            &ProofVerifier {
                accepted: true,
                calls: Cell::new(0),
            },
            &accepting_stateless(),
            input,
            &Sha2Hasher,
        ),
        Err(Error::NonEmptyIntermediateBlock)
    );
}

#[test]
fn rejects_stateless_output_for_a_different_request() {
    let (input, _) = base_input();
    let stateless = StatelessVerifier {
        successful: true,
        wrong_root: true,
        input: RefCell::new(None),
    };

    assert_eq!(
        process_private_input(
            &ProofVerifier {
                accepted: true,
                calls: Cell::new(0),
            },
            &stateless,
            input,
            &Sha2Hasher,
        ),
        Err(Error::StatelessRequestRootMismatch)
    );
}

#[test]
fn rejects_progressive_blob_commitments_above_runtime_limit() {
    let (mut input, _) = base_input();
    let target = input
        .beacon_chain_witness
        .beacon_lineage
        .last_mut()
        .unwrap();
    target.signed_bid.message.blob_kzg_commitments =
        vec![[5; 48]; MAX_BLOB_COMMITMENTS_PER_BLOCK + 1].into();

    let mut body_root = target.signed_bid.hash_tree_root(&Sha2Hasher);
    for (level, sibling) in target.signed_bid_merkle_witness.iter().enumerate() {
        body_root = if ((SIGNED_EXECUTION_PAYLOAD_BID_INDEX >> level) & 1) == 0 {
            hash_nodes(&Sha2Hasher, &body_root, sibling)
        } else {
            hash_nodes(&Sha2Hasher, sibling, &body_root)
        };
    }
    target.header.body_root = body_root;
    input
        .beacon_chain_witness
        .signed_envelope
        .message
        .beacon_block_root = target.header.hash_tree_root(&Sha2Hasher);

    assert_eq!(
        process_private_input(
            &ProofVerifier {
                accepted: true,
                calls: Cell::new(0),
            },
            &accepting_stateless(),
            input,
            &Sha2Hasher,
        ),
        Err(Error::VersionedHashesOverflow)
    );
}
