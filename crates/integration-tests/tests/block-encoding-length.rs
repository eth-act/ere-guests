//! Execution tests for `block-encoding-length` guest program

use ere_dockerized::zkVMKind;
use integration_tests::test_execution;

const GUEST: &str = "block-encoding-length";

#[test]
fn test_execution_airbender() {
    test_execution(GUEST, zkVMKind::Airbender);
}

#[test]
fn test_execution_openvm() {
    test_execution(GUEST, zkVMKind::OpenVM);
}

#[test]
fn test_execution_pico() {
    test_execution(GUEST, zkVMKind::Pico);
}

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
