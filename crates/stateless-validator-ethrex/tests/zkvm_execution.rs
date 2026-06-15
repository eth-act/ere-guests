//! Execution tests for `stateless-validator-ethrex` guest program

use ere_dockerized::zkVMKind;
use stateless_validator_ethrex::guest::run_stateless_guest;
use stateless_validator_test::zkvm::{StdoutNoopPlatform, test_stateless_validator_execution};

fn test_execution(zkvm_kind: zkVMKind) {
    test_stateless_validator_execution(
        "stateless-validator-ethrex",
        zkvm_kind,
        run_stateless_guest::<StdoutNoopPlatform>,
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
