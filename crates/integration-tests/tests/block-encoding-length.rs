//! Execution tests for `block-encoding-length` guest program

use std::fs;

use block_encoding_length::guest::{
    BlockEncodingFormat, BlockEncodingLengthGuest, BlockEncodingLengthInput,
};
use ere_dockerized::zkVMKind;
use integration_tests::{
    TestCase, fixtures_dir, stateless_validator::StatelessValidatorFixture, untar_fixtures,
};

fn test_execution(zkvm_kind: zkVMKind) {
    untar_fixtures().unwrap();
    let path = fixtures_dir().join("block/rpc_block_22974575.json");
    let fixture: StatelessValidatorFixture =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let block = fixture.stateless_input.block;
    let loop_count = 10;
    let test_cases = [BlockEncodingFormat::Rlp, BlockEncodingFormat::Ssz].map(|format| {
        let input = BlockEncodingLengthInput::new(&block, loop_count, format).unwrap();
        TestCase::new::<BlockEncodingLengthGuest>(format!("{format:?}"), input, ())
    });
    integration_tests::test_execution("block-encoding-length", zkvm_kind, test_cases);
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
