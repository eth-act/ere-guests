//! Execution tests for `panic` guest program

use ere_dockerized::zkVMKind;
use ere_zkvm_interface::{Input, zkVM};
use integration_tests::compile_and_init_zkvm;

fn test_execution(zkvm_kind: zkVMKind) {
    let zkvm = compile_and_init_zkvm("panic", zkvm_kind);

    assert!(zkvm.execute(&Input::new()).is_err());
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
