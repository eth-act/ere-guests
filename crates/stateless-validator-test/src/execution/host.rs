//! Host-side execution of stateless validator guests.

use std::io::{self, Write};

use ere_platform_core::Platform;
use libssz_merkle::hash_nodes;
use recursive_execution_proof::{
    BeaconBlockBidWitness, BeaconBlockHeader, BeaconChainWitness, Error, ExecutionCheckpoint,
    ExecutionPayloadBid, ExecutionPayloadEnvelope, ExecutionProof, ExecutionProofVerifier,
    PrivateInput, SIGNED_EXECUTION_PAYLOAD_BID_INDEX, SignedExecutionPayloadBid,
    SignedExecutionPayloadEnvelope,
};
use stateless_validator_catalog::StatelessValidatorKind;
use stateless_validator_common::guest::input::{
    ChainConfig, ExecutionWitness, ForkActivation, ForkConfig,
    new_payload_request::{ExecutionPayloadV4, ExecutionRequestsGloas},
};
use stateless_validator_common::{HashTreeRoot, Sha2Hasher, SszList};

use crate::{
    execution::{ExecutionFailure, ExecutionFailures, run_execution},
    fixture::{FixturePreset, StatelessValidatorFixture, preset_fixtures},
};

/// A platform for host-side guest execution.
#[derive(Debug)]
pub struct HostPlatform;

impl Platform for HostPlatform {
    #[allow(unreachable_code)]
    fn read_input() -> impl std::ops::Deref<Target = [u8]> {
        unreachable!();
        &[] as &[u8]
    }

    fn write_output(_: &[u8]) {
        unreachable!();
    }

    fn print(message: &str) {
        print!("{message}");
        let _ = io::stdout().flush();
    }
}

/// Resolves the native guest entrypoint for `stateless_validator_kind`, then runs `fixtures`
/// through it on the host, returning the failures.
pub fn run_host_execution(
    stateless_validator_kind: StatelessValidatorKind,
    fixtures: impl IntoIterator<Item = StatelessValidatorFixture>,
) -> Vec<ExecutionFailure> {
    let execute: fn(&[u8]) -> Vec<u8> = match stateless_validator_kind {
        StatelessValidatorKind::Ethrex => {
            stateless_validator_ethrex::guest::run_stateless_guest::<HostPlatform>
        }
        StatelessValidatorKind::Reth => {
            stateless_validator_reth::guest::run_stateless_guest::<HostPlatform>
        }
        StatelessValidatorKind::Zesu => {
            panic!("host execution is not supported for the zesu guest")
        }
    };
    run_execution(fixtures, &|input| Ok(execute(&input)))
}

/// Runs `preset` on the host through the `stateless_validator_kind` guest, asserting the failure
/// count matches `expected_failures`.
pub fn test_host_execution(
    stateless_validator_kind: StatelessValidatorKind,
    preset: FixturePreset,
    expected_failures: usize,
) {
    let failures = run_host_execution(stateless_validator_kind, preset_fixtures(preset));
    assert_eq!(
        failures.len(),
        expected_failures,
        "expected {expected_failures} failures, got {}:\n{}",
        failures.len(),
        ExecutionFailures(&failures),
    );
}

/// Exercises a client's recursive adapter through the native stateless executor.
///
/// The synthetic payload is deliberately not executable. All consensus bindings
/// are valid, so the expected stateless rejection proves that the client adapter
/// reached its existing execution-validation path.
pub fn test_host_recursive_execution(stateless_validator_kind: StatelessValidatorKind) {
    let input = synthetic_recursive_input();
    let result = match stateless_validator_kind {
        StatelessValidatorKind::Ethrex => {
            stateless_validator_ethrex::guest::process_recursive_input::<HostPlatform>(
                input,
                &AcceptProof,
            )
        }
        StatelessValidatorKind::Reth => stateless_validator_reth::guest::process_recursive_input::<
            HostPlatform,
        >(input, &AcceptProof),
        StatelessValidatorKind::Zesu => {
            panic!("host execution is not supported for the zesu guest")
        }
    };
    assert_eq!(result, Err(Error::StatelessValidationFailed));
}

#[derive(Debug)]
struct AcceptProof;

impl ExecutionProofVerifier for AcceptProof {
    fn verify_execution_proof(&self, _proof: &ExecutionProof) -> bool {
        true
    }
}

fn synthetic_recursive_input() -> PrivateInput {
    let requests = ExecutionRequestsGloas::default();
    let checkpoint_bid = synthetic_bid([6; 32], [7; 32], [9; 32], 10, &requests);
    let checkpoint = synthetic_bid_witness(10, [7; 32], checkpoint_bid, 10);
    let checkpoint_root = checkpoint.header.hash_tree_root(&Sha2Hasher);
    let origin = ExecutionCheckpoint {
        slot: 10,
        beacon_block_root: checkpoint_root,
    };

    let payload = ExecutionPayloadV4 {
        parent_hash: [9; 32],
        fee_recipient: [0; 20],
        state_root: [1; 32],
        receipts_root: [2; 32],
        logs_bloom: [0; 256],
        prev_randao: [3; 32],
        block_number: 1,
        gas_limit: 30_000_000,
        gas_used: 21_000,
        timestamp: 1,
        extra_data: SszList::default(),
        base_fee_per_gas: [4; 32],
        block_hash: [11; 32],
        transactions: Default::default(),
        withdrawals: Default::default(),
        blob_gas_used: 0,
        excess_blob_gas: 0,
        block_access_list: Default::default(),
        slot_number: 12,
    };
    let target_bid = synthetic_bid([9; 32], checkpoint_root, [11; 32], 12, &requests);
    let target = synthetic_bid_witness(12, checkpoint_root, target_bid, 20);
    let target_root = target.header.hash_tree_root(&Sha2Hasher);

    PrivateInput {
        beacon_chain_witness: BeaconChainWitness {
            origin: Some(origin),
            previous_proof: None,
            beacon_lineage: vec![checkpoint, target],
            signed_envelope: SignedExecutionPayloadEnvelope {
                message: ExecutionPayloadEnvelope {
                    payload,
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
    }
}

fn synthetic_bid(
    parent_block_hash: [u8; 32],
    parent_block_root: [u8; 32],
    block_hash: [u8; 32],
    slot: u64,
    requests: &ExecutionRequestsGloas,
) -> ExecutionPayloadBid {
    ExecutionPayloadBid {
        parent_block_hash,
        parent_block_root,
        block_hash,
        prev_randao: [3; 32],
        fee_recipient: [0; 20],
        gas_limit: 30_000_000,
        builder_index: 42,
        slot,
        value: 0,
        execution_payment: 0,
        blob_kzg_commitments: Default::default(),
        execution_requests_root: recursive_execution_proof::execution_requests_root(
            requests,
            &Sha2Hasher,
        ),
    }
}

fn synthetic_bid_witness(
    slot: u64,
    parent_root: [u8; 32],
    bid: ExecutionPayloadBid,
    branch_byte: u8,
) -> BeaconBlockBidWitness {
    let signed_bid = SignedExecutionPayloadBid {
        message: bid,
        signature: [0; 96],
    };
    let branch = core::array::from_fn(|level| [branch_byte.wrapping_add(level as u8); 32]);
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
            state_root: [8; 32],
            body_root,
        },
        signed_bid,
        signed_bid_merkle_witness: branch,
    }
}

/// Declares a host execution test for a guest kind and fixture preset.
#[macro_export]
macro_rules! declare_test_host_execution {
    ($kind:ident, $preset:ident, failures = $expected_failures:expr) => {
        paste::paste! {
            #[test]
            fn [<test_host_execution_ $preset:snake>]() {
                $crate::execution::host::test_host_execution(
                    $crate::StatelessValidatorKind::$kind,
                    $crate::fixture::FixturePreset::$preset,
                    $expected_failures,
                );
            }
        }
    };
    ($kind:ident, $preset:ident) => {
        $crate::declare_test_host_execution!($kind, $preset, failures = 0);
    };
}

/// Declares a focused host-side recursive adapter test for a guest kind.
#[macro_export]
macro_rules! declare_test_host_recursive_execution {
    ($kind:ident) => {
        paste::paste! {
            #[test]
            fn [<test_host_recursive_execution_ $kind:snake>]() {
                $crate::execution::host::test_host_recursive_execution(
                    $crate::StatelessValidatorKind::$kind,
                );
            }
        }
    };
}
