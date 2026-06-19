//! Execution tests for `stateless-validator-ethrex` guest program

use ere_dockerized::zkVMKind;
use stateless_validator_ethrex::guest::run_stateless_guest;
use stateless_validator_test::{
    fixture::Fork,
    zkvm::{StdoutNoopPlatform, test_stateless_validator_execution},
};

fn test_execution(fork: Fork, zkvm_kind: zkVMKind) {
    test_stateless_validator_execution(
        fork,
        "stateless-validator-ethrex",
        zkvm_kind,
        run_stateless_guest::<StdoutNoopPlatform>,
    );
}

#[test]
fn test_execution_fusaka_risc0() {
    test_execution(Fork::Fusaka, zkVMKind::Risc0);
}

#[test]
fn test_execution_glamsterdam_risc0() {
    test_execution(Fork::Glamsterdam, zkVMKind::Risc0);
}

#[test]
fn test_execution_fusaka_sp1() {
    test_execution(Fork::Fusaka, zkVMKind::SP1);
}

#[test]
fn test_execution_glamsterdam_sp1() {
    test_execution(Fork::Glamsterdam, zkVMKind::SP1);
}

#[test]
fn test_execution_fusaka_zisk() {
    test_execution(Fork::Fusaka, zkVMKind::Zisk);
}

#[test]
fn test_execution_glamsterdam_zisk() {
    test_execution(Fork::Glamsterdam, zkVMKind::Zisk);
}
