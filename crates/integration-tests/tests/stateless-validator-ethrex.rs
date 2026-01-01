//! Execution tests for `stateless-validator-ethrex` guest program

use ere_dockerized::zkVMKind;
use integration_tests::test_execution;

const GUEST: &str = "stateless-validator-ethrex";

#[test]
fn test_execution_risc0() {
    test_execution(GUEST, zkVMKind::Risc0);
}

#[test]
fn test_execution_sp1() {
    test_execution(GUEST, zkVMKind::SP1);
}

#[test]
fn test_execution_zisk() {
    test_execution(GUEST, zkVMKind::Zisk);
}
