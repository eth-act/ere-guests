//! `tests-zkevm@v0.8.2` execution tests for release-backed guests.
//!
//! Set `STATELESS_VALIDATOR` and `ZKVM` to one pair from `artifact-registry.json`.
//! Run with `ERE_IMAGE_REGISTRY=ghcr.io/eth-act/ere` to use pre-built Ere images.

use ere_dockerized::zkVMKind;
use stateless_validator_catalog::StatelessValidatorKind;
use stateless_validator_test::{
    execution::{
        ExecutionFailures, init_tracing,
        zkvm::{is_guest_compatible, run_zkvm_execution},
    },
    fixture::eest_fixtures,
};

const RETH_EXPECTED_FAILURES: &[&str] = &[
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_opcode[fork_Amsterdam-blockchain_test_from_state_test-opcode_CREATE-non-empty-balance-correct-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_opcode[fork_Amsterdam-blockchain_test_from_state_test-opcode_CREATE2-non-empty-balance-correct-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_tx[fork_Amsterdam-tx_type_0-blockchain_test_from_state_test-non-empty-balance-correct-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_tx[fork_Amsterdam-tx_type_0-blockchain_test_from_state_test-non-empty-balance-revert-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_tx[fork_Amsterdam-tx_type_1-blockchain_test_from_state_test-non-empty-balance-correct-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_tx[fork_Amsterdam-tx_type_1-blockchain_test_from_state_test-non-empty-balance-revert-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_tx[fork_Amsterdam-tx_type_2-blockchain_test_from_state_test-non-empty-balance-correct-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_initcollision.py::test_init_collision_create_tx[fork_Amsterdam-tx_type_2-blockchain_test_from_state_test-non-empty-balance-revert-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_revert_in_create.py::test_collision_with_create2_revert_in_initcode[fork_Amsterdam-blockchain_test_from_state_test]#block0",
    "tests/paris/eip7610_create_collision/test_revert_in_create.py::test_create2_collision_storage[fork_Amsterdam-blockchain_test_from_state_test-empty-initcode]#block0",
    "tests/paris/eip7610_create_collision/test_revert_in_create.py::test_create2_collision_storage[fork_Amsterdam-blockchain_test_from_state_test-initcode-with-deploy]#block0",
    "tests/paris/eip7610_create_collision/test_revert_in_create.py::test_create2_collision_storage[fork_Amsterdam-blockchain_test_from_state_test-sstore-initcode]#block0",
];

fn expected_failures(stateless_validator: StatelessValidatorKind) -> &'static [&'static str] {
    match stateless_validator {
        StatelessValidatorKind::Reth => RETH_EXPECTED_FAILURES,
    }
}

#[test]
fn executes_registered_guest() {
    init_tracing();
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

    let failures = run_zkvm_execution(stateless_validator, zkvm, eest_fixtures());
    let failure_names = failures
        .iter()
        .map(|failure| failure.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        failure_names,
        expected_failures(stateless_validator),
        "unexpected upstream failure set ({} failures):\n{}",
        failures.len(),
        ExecutionFailures(&failures),
    );
}
