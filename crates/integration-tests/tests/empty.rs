//! Execution tests for `empty` guest program

use ere_dockerized::zkVMKind;
use integration_tests::TestCase;

fn test_execution(zkvm_kind: zkVMKind) {
    integration_tests::test_execution("empty", zkvm_kind, [TestCase::default()]);
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
