//! Execution tests for `stateless-validator-ethrex` guest program

use std::fs;

use ere_dockerized::zkVMKind;
use integration_tests::{
    TestCase, fixtures_dir, stateless_validator::StatelessValidatorFixture, untar_fixtures,
};
use stateless_validator_ethrex::guest::{
    StatelessValidatorEthrexGuest, StatelessValidatorEthrexInput, StatelessValidatorOutput,
};

fn test_execution(zkvm_kind: zkVMKind) {
    untar_fixtures().unwrap();
    let inputs = fs::read_dir(fixtures_dir().join("block"))
        .unwrap()
        .map(|file| {
            let bytes = fs::read(file.unwrap().path()).unwrap();
            let fixture: StatelessValidatorFixture = serde_json::from_slice(&bytes).unwrap();
            let input = StatelessValidatorEthrexInput::new(&fixture.stateless_input).unwrap();
            let output = StatelessValidatorOutput::new(
                fixture.stateless_input.block.hash_slow(),
                fixture.stateless_input.block.parent_hash,
                fixture.success,
            );
            TestCase::new::<StatelessValidatorEthrexGuest>(fixture.name, input, output)
                .output_sha256()
        });
    integration_tests::test_execution("stateless-validator-ethrex", zkvm_kind, inputs);
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
