//! Execution tests for `empty` guest program

use ere_dockerized::zkVMKind;
use ere_io::serde::{IoSerde, bincode::BincodeLegacy};
use guest::{Guest, GuestInput, GuestOutput, Platform};

fn test_execution(zkvm_kind: zkVMKind) {
    #[derive(Clone)]
    struct EmptyGuest;

    impl Guest for EmptyGuest {
        type Io = IoSerde<(), (), BincodeLegacy>;

        fn compute<P: Platform>(_: GuestInput<Self>) -> GuestOutput<Self> {}
    }

    integration_tests::test_execution::<EmptyGuest>("empty", zkvm_kind, [()], false);
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
