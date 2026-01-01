//! Execution tests for `stateless-validator-ethrex` guest program

use std::fs;

use ere_dockerized::zkVMKind;
use integration_tests::workspace;
use stateless_validator_ethrex::guest::{
    StatelessValidatorEthrexGuest, StatelessValidatorEthrexInput,
};

fn test_execution(zkvm_kind: zkVMKind) {
    let inputs = fs::read_dir(workspace().join("crates/integration-tests/assets/block"))
        .unwrap()
        .map(|file| {
            let bytes = fs::read(file.unwrap().path()).unwrap();
            let stateless_input = serde_json::from_slice(&bytes).unwrap();
            StatelessValidatorEthrexInput::new(&stateless_input).unwrap()
        });
    integration_tests::test_execution::<StatelessValidatorEthrexGuest>(
        "stateless-validator-ethrex",
        zkvm_kind,
        inputs,
        true,
    );
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
