//! Execution tests for `stateless-validator-reth` guest program

use std::fs;

use ere_dockerized::zkVMKind;
use integration_tests::{
    TestCase, fixtures_dir, stateless_validator::StatelessValidatorFixture, untar_fixtures,
};
use stateless_validator_reth::guest::{StatelessValidatorRethGuest, StatelessValidatorRethInput};

fn test_execution(zkvm_kind: zkVMKind) {
    untar_fixtures().unwrap();
    let inputs = fs::read_dir(fixtures_dir().join("block"))
        .unwrap()
        .map(|file| {
            let bytes = fs::read(file.unwrap().path()).unwrap();
            let fixture: StatelessValidatorFixture = serde_json::from_slice(&bytes).unwrap();
            let input = StatelessValidatorRethInput::new(&fixture.stateless_input).unwrap();
            let output = (
                fixture.stateless_input.block.hash_slow().0,
                fixture.stateless_input.block.parent_hash.0,
                fixture.success,
            );
            TestCase::new::<StatelessValidatorRethGuest>(fixture.name, input, output)
                .output_sha256()
        });
    integration_tests::test_execution("stateless-validator-reth", zkvm_kind, inputs);
}

#[test]
fn test_execution_airbender() {
    test_execution(zkVMKind::Airbender);
}

#[test]
fn test_execution_openvm() {
    test_execution(zkVMKind::OpenVM);
}

#[test]
fn test_execution_pico() {
    test_execution(zkVMKind::Pico);
}

#[test]
fn test_execution_risc0() {
    test_execution(zkVMKind::Risc0);
}

#[test]
fn test_execution_sp1() {
    test_execution(zkVMKind::SP1);
}

#[test]
fn test_execution_zisk() {
    test_execution(zkVMKind::Zisk);
}
