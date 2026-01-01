//! Execution tests for `block-encoding-length` guest program

use std::fs;

use block_encoding_length::guest::{
    BlockEncodingFormat, BlockEncodingLengthGuest, BlockEncodingLengthInput,
};
use ere_dockerized::zkVMKind;
use integration_tests::workspace;
use reth_stateless::StatelessInput;

fn test_execution(zkvm_kind: zkVMKind) {
    let path = workspace().join("crates/integration-tests/assets/block/mainnet-22974575.json");
    let stateless_input: StatelessInput = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    for format in [BlockEncodingFormat::Rlp, BlockEncodingFormat::Ssz] {
        integration_tests::test_execution::<BlockEncodingLengthGuest>(
            "block-encoding-length",
            zkvm_kind,
            [BlockEncodingLengthInput::new(&stateless_input.block, 10, format).unwrap()],
            false,
        );
    }
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
