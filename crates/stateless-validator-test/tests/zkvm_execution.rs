//! `tests-zkevm@v21.0.7` execution tests for release-backed guests, over the EEST fixtures and a
//! pinned Sepolia batch.
//!
//! Set `STATELESS_VALIDATOR` and `ZKVM` to one pair from `artifact-registry.json`.
//! Run with `ERE_IMAGE_REGISTRY=ghcr.io/eth-act/ere` to use pre-built Ere images.

use ere_dockerized::zkVMKind;
use stateless_validator_catalog::StatelessValidatorKind;
use stateless_validator_test::{
    execution::{
        ExecutionFailures, init_tracing,
        zkvm::{is_guest_compatible, run_zkvm_execution, run_zkvm_preprocess},
    },
    fixture::{StatelessValidatorFixture, eest_fixtures, sepolia_preset_fixtures},
};

// Reth computes a different block access list hash for these fixtures. alloy-evm runs the
// post-execution system calls before it credits withdrawals, and revm's BAL builder records a
// change when a value returns to its start-of-index value. See paradigmxyz/stateless#48.
const RETH_EXPECTED_BAL_FAILURES: &[&str] = &[
    "tests/amsterdam/eip7928_block_level_access_lists/test_block_access_lists_cross_index.py::test_bal_post_execution_calls_net_storage_at_last_index[fork_Amsterdam-blockchain_test_engine]#payload0",
    "tests/amsterdam/eip7928_block_level_access_lists/test_block_access_lists_cross_index.py::test_bal_withdrawals_and_dequeues_net_balance_at_last_index[fork_Amsterdam-blockchain_test_engine-forward_all]#payload0",
    "tests/amsterdam/eip7928_block_level_access_lists/test_block_access_lists_cross_index.py::test_bal_withdrawals_and_dequeues_net_balance_at_last_index[fork_Amsterdam-blockchain_test_engine-forward_half]#payload0",
];

// Reth fails these fixtures because alloy-evm runs the request system calls before it credits
// withdrawals, the reverse of EELS `apply_body`. See paradigmxyz/stateless#61.
const RETH_EXPECTED_REQUEST_FAILURES: &[&str] = &[
    "tests/prague/eip7685_general_purpose_el_requests/test_multi_type_requests.py::test_withdrawals_credited_before_request_system_call[fork_Amsterdam-request_class_BuilderDepositRequest-blockchain_test_engine-forward_all]#payload0",
    "tests/prague/eip7685_general_purpose_el_requests/test_multi_type_requests.py::test_withdrawals_credited_before_request_system_call[fork_Amsterdam-request_class_BuilderDepositRequest-blockchain_test_engine-forward_half]#payload0",
    "tests/prague/eip7685_general_purpose_el_requests/test_multi_type_requests.py::test_withdrawals_credited_before_request_system_call[fork_Amsterdam-request_class_BuilderExitRequest-blockchain_test_engine-forward_all]#payload0",
    "tests/prague/eip7685_general_purpose_el_requests/test_multi_type_requests.py::test_withdrawals_credited_before_request_system_call[fork_Amsterdam-request_class_BuilderExitRequest-blockchain_test_engine-forward_half]#payload0",
    "tests/prague/eip7685_general_purpose_el_requests/test_multi_type_requests.py::test_withdrawals_credited_before_request_system_call[fork_Amsterdam-request_class_ConsolidationRequest-blockchain_test_engine-forward_all]#payload0",
    "tests/prague/eip7685_general_purpose_el_requests/test_multi_type_requests.py::test_withdrawals_credited_before_request_system_call[fork_Amsterdam-request_class_ConsolidationRequest-blockchain_test_engine-forward_half]#payload0",
    "tests/prague/eip7685_general_purpose_el_requests/test_multi_type_requests.py::test_withdrawals_credited_before_request_system_call[fork_Amsterdam-request_class_WithdrawalRequest-blockchain_test_engine-forward_all]#payload0",
    "tests/prague/eip7685_general_purpose_el_requests/test_multi_type_requests.py::test_withdrawals_credited_before_request_system_call[fork_Amsterdam-request_class_WithdrawalRequest-blockchain_test_engine-forward_half]#payload0",
];

// Reth rejects this valid block, in which two post-execution system calls create and then fund one
// contract at the same block access index. revm's BAL builder drops the contract's nonce and code
// changes. See bluealloy/revm#3954.
const RETH_EXPECTED_SAME_INDEX_BAL_FAILURES: &[&str] = &[
    "tests/amsterdam/eip7928_block_level_access_lists/test_block_access_lists_cross_index.py::test_bal_same_index_call_keeps_one_nonce_and_code_change[fork_Amsterdam-blockchain_test_engine]#payload0",
];

// Reth rejects valid blocks whose witness leaves out code the block already set: an EIP-7702
// delegation marker shared with another account, or code that an earlier CREATE deployed.
const RETH_EXPECTED_WITNESS_FAILURES: &[&str] = &[
    "tests/amsterdam/eip8025_optional_proofs/test_witness_7702.py::test_witness_codes_delegation_same_marker_cleared_then_read[fork_Amsterdam-blockchain_test_engine-clear_in_later_tx]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_7702.py::test_witness_codes_delegation_same_marker_cleared_then_read[fork_Amsterdam-blockchain_test_engine-clear_in_same_tx]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_7702.py::test_witness_codes_delegation_same_marker_then_read[fork_Amsterdam-blockchain_test_engine-read_authorization]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_7702.py::test_witness_codes_delegation_same_marker_then_read[fork_Amsterdam-blockchain_test_engine-read_call]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_7702.py::test_witness_codes_delegation_same_marker_then_read[fork_Amsterdam-blockchain_test_engine-read_extcodecopy]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_7702.py::test_witness_codes_delegation_same_marker_then_read[fork_Amsterdam-blockchain_test_engine-read_extcodesize]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_7702.py::test_witness_codes_delegation_same_marker_then_read[fork_Amsterdam-blockchain_test_engine-read_sender]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_7702.py::test_witness_codes_delegation_same_marker_then_read[fork_Amsterdam-blockchain_test_engine-read_top_level]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_7702.py::test_witness_codes_delegation_same_marker_then_read_same_tx[fork_Amsterdam-blockchain_test_engine-read_authorization]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_7702.py::test_witness_codes_delegation_same_marker_then_read_same_tx[fork_Amsterdam-blockchain_test_engine-read_call]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_7702.py::test_witness_codes_delegation_same_marker_then_read_same_tx[fork_Amsterdam-blockchain_test_engine-read_extcodecopy]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_7702.py::test_witness_codes_delegation_same_marker_then_read_same_tx[fork_Amsterdam-blockchain_test_engine-read_extcodesize]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_7702.py::test_witness_codes_delegation_same_marker_then_read_same_tx[fork_Amsterdam-blockchain_test_engine-read_top_level]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_7702.py::test_witness_codes_delegation_same_marker_tx_outcome[fork_Amsterdam-blockchain_test_engine-outcome_execution_reverts]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_7702.py::test_witness_codes_delegation_same_marker_tx_outcome[fork_Amsterdam-blockchain_test_engine-outcome_preparation_exact_gas]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_bytecodes_contract_creation.py::test_witness_codes_create_same_hash_then_read_same_tx[fork_Amsterdam-blockchain_test_engine-read_op_call]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_bytecodes_contract_creation.py::test_witness_codes_create_same_hash_then_read_same_tx[fork_Amsterdam-blockchain_test_engine-read_op_extcodecopy]#payload0",
    "tests/amsterdam/eip8025_optional_proofs/test_witness_bytecodes_contract_creation.py::test_witness_codes_create_same_hash_then_read_same_tx[fork_Amsterdam-blockchain_test_engine-read_op_extcodesize]#payload0",
];

// Reth rejects these valid Sepolia blocks from the preset batch, and its guest reports no reason.
const RETH_EXPECTED_SEPOLIA_PRESET_FAILURES: &[&str] = &[
    "witness-generator-spec-cli::block_12049665_1dbbb95849fab60b230769a1b6c01084e553afa0ca6dd97e0048c46aaad59942#payload0",
    "witness-generator-spec-cli::block_12049671_90849d41fd2846b54b190cde8c3de2b17f9cc4bb727515f4200aed6e739020b6#payload0",
];

fn expected_failures(
    stateless_validator: StatelessValidatorKind,
    zkvm: zkVMKind,
) -> Vec<&'static str> {
    let groups: &[&[&str]] = match (stateless_validator, zkvm) {
        (StatelessValidatorKind::Ethrex, _) => &[],
        (StatelessValidatorKind::Reth, _) => &[
            RETH_EXPECTED_BAL_FAILURES,
            RETH_EXPECTED_REQUEST_FAILURES,
            RETH_EXPECTED_SAME_INDEX_BAL_FAILURES,
            RETH_EXPECTED_WITNESS_FAILURES,
        ],
        (StatelessValidatorKind::Zesu, zkVMKind::OpenVM | zkVMKind::SP1 | zkVMKind::Zisk) => &[],
        (StatelessValidatorKind::Nimbus, zkVMKind::SP1 | zkVMKind::Zisk) => &[],
        (StatelessValidatorKind::Nethermind, zkVMKind::SP1 | zkVMKind::Zisk) => &[],
        (StatelessValidatorKind::Zilkworm, zkVMKind::SP1) => &[],
        (
            StatelessValidatorKind::Zesu
            | StatelessValidatorKind::Nimbus
            | StatelessValidatorKind::Nethermind
            | StatelessValidatorKind::Zilkworm,
            _,
        ) => {
            panic!("{stateless_validator}-{zkvm} has no active registry artifacts")
        }
    };
    // `run_zkvm_execution` sorts failures by name.
    let mut failures = groups.concat();
    failures.sort_unstable();
    failures
}

// The Sepolia preset failures are the same on every zkVM, and listed in name order.
fn expected_sepolia_preset_failures(
    stateless_validator: StatelessValidatorKind,
) -> &'static [&'static str] {
    match stateless_validator {
        StatelessValidatorKind::Reth => RETH_EXPECTED_SEPOLIA_PRESET_FAILURES,
        _ => &[],
    }
}

fn registered_pair() -> (StatelessValidatorKind, zkVMKind) {
    let stateless_validator = std::env::var("STATELESS_VALIDATOR")
        .expect("STATELESS_VALIDATOR must name an artifact-registry.json guest")
        .parse::<StatelessValidatorKind>()
        .unwrap();
    let zkvm = std::env::var("ZKVM")
        .expect("ZKVM must name an artifact-registry.json zkVM")
        .parse::<zkVMKind>()
        .unwrap();
    assert!(
        is_guest_compatible(stateless_validator, zkvm),
        "{stateless_validator}-{zkvm} is incompatible with Ere SDK {}",
        zkvm.sdk_version()
    );
    (stateless_validator, zkvm)
}

fn assert_execution(
    stateless_validator: StatelessValidatorKind,
    zkvm: zkVMKind,
    fixtures: Vec<StatelessValidatorFixture>,
    expected_failures: &[&str],
) {
    let failures = run_zkvm_execution(stateless_validator, zkvm, fixtures);
    let failure_names = failures
        .iter()
        .map(|failure| failure.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        failure_names,
        expected_failures,
        "unexpected upstream failure set ({} failures):\n{}",
        failures.len(),
        ExecutionFailures(&failures),
    );
}

#[test]
fn executes_registered_guest() {
    init_tracing();
    let (stateless_validator, zkvm) = registered_pair();
    assert_execution(
        stateless_validator,
        zkvm,
        eest_fixtures(),
        &expected_failures(stateless_validator, zkvm),
    );
}

#[test]
fn executes_registered_guest_sepolia_preset() {
    init_tracing();
    let (stateless_validator, zkvm) = registered_pair();
    assert_execution(
        stateless_validator,
        zkvm,
        sepolia_preset_fixtures(),
        expected_sepolia_preset_failures(stateless_validator),
    );
}

#[test]
fn preprocesses_registered_guest() {
    init_tracing();
    let (stateless_validator, zkvm) = registered_pair();
    run_zkvm_preprocess(stateless_validator, zkvm);
}
