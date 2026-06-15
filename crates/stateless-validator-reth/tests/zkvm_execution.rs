//! Execution tests for `stateless-validator-reth` guest program

use ere_dockerized::zkVMKind;
use stateless_validator_reth::guest::run_stateless_guest;
use stateless_validator_test::zkvm::{StdoutNoopPlatform, test_stateless_validator_execution};

fn test_execution(zkvm_kind: zkVMKind) {
    test_stateless_validator_execution(
        "stateless-validator-reth",
        zkvm_kind,
        run_stateless_guest::<StdoutNoopPlatform>,
    );
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
