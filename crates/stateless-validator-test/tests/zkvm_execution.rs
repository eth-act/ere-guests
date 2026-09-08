//! `tests-zkevm@v0.8.4` execution tests for release-backed guests.
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
    fixture::{StatelessValidatorFixture, devnet_preset_fixtures, eest_fixtures},
};

// These fixtures exceed the Ethrex guest memory available on OpenVM and ZisK. SP1 executes them.
const ETHREX_EXPECTED_RESOURCE_FAILURES: &[&str] = &[
    "tests/ported_static/stQuadraticComplexityTest/test_return50000.py::test_return50000[fork_Amsterdam-blockchain_test_from_state_test--g1]#block0",
    "tests/ported_static/stQuadraticComplexityTest/test_return50000_2.py::test_return50000_2[fork_Amsterdam-blockchain_test_from_state_test--g1]#block0",
    "tests/ported_static/stStaticCall/test_static_return50000_2.py::test_static_return50000_2[fork_Amsterdam-blockchain_test_from_state_test]#block0",
];

fn expected_failures(
    stateless_validator: StatelessValidatorKind,
    zkvm: zkVMKind,
) -> &'static [&'static str] {
    match (stateless_validator, zkvm) {
        (StatelessValidatorKind::Ethrex, zkVMKind::OpenVM | zkVMKind::Zisk) => {
            ETHREX_EXPECTED_RESOURCE_FAILURES
        }
        (StatelessValidatorKind::Ethrex, zkVMKind::SP1) => &[],
        (StatelessValidatorKind::Reth | StatelessValidatorKind::Zesu, _) => {
            panic!("{stateless_validator} has no active registry artifacts")
        }
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
        expected_failures(stateless_validator, zkvm),
    );
}

#[test]
fn executes_registered_guest_devnet_preset() {
    init_tracing();
    let (stateless_validator, zkvm) = registered_pair();
    assert_execution(stateless_validator, zkvm, devnet_preset_fixtures(), &[]);
}
