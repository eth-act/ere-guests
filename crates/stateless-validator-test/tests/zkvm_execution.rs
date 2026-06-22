//! Execution tests for stateless validator guest program

use ere_dockerized::zkVMKind;
use stateless_validator_test::{
    execution::{ExecutionFailures, GuestKind, zkvm::run_stateless_validator_execution},
    fixture::FixturePreset,
};

fn test_execution(guest_kind: GuestKind, zkvm_kind: zkVMKind, preset: FixturePreset) {
    let execution_failures = run_stateless_validator_execution(guest_kind, zkvm_kind, preset);
    assert!(
        execution_failures.is_empty(),
        "{}",
        ExecutionFailures(&execution_failures)
    );
}

macro_rules! declare_test {
    ($guest_kind:ident, $zkvm_kind:ident, $preset:ident) => {
        paste::paste! {
            #[test]
            fn [<test_execution_ $guest_kind:lower _ $zkvm_kind:lower _ $preset:snake>]() {
                test_execution(GuestKind::$guest_kind, zkVMKind::$zkvm_kind, FixturePreset::$preset);
            }
        }
    };
    ($guest_kind:ident, $zkvm_kind:ident, $preset:ident, should_panic) => {
        paste::paste! {
            #[test]
            #[should_panic]
            fn [<test_execution_ $guest_kind:lower _ $zkvm_kind:lower _ $preset:snake>]() {
                test_execution(GuestKind::$guest_kind, zkVMKind::$zkvm_kind, FixturePreset::$preset);
            }
        }
    };
}

// Ethrex

declare_test!(Ethrex, Risc0, RpcBpo2);
declare_test!(Ethrex, Risc0, RpcGlamsterdamDevnet5);
declare_test!(Ethrex, Risc0, EestBalDevnet7);
declare_test!(Ethrex, SP1, RpcBpo2);
declare_test!(Ethrex, SP1, RpcGlamsterdamDevnet5);
declare_test!(Ethrex, SP1, EestBalDevnet7);
declare_test!(Ethrex, Zisk, RpcBpo2);
declare_test!(Ethrex, Zisk, RpcGlamsterdamDevnet5);
// NOTE: `should_panic` should be unnecessary when upgraded to `zisk@v1.0.0-beta`
declare_test!(Ethrex, Zisk, EestBalDevnet7, should_panic);

// Reth (only bpo2)

declare_test!(Reth, Airbender, RpcBpo2);
declare_test!(Reth, OpenVM, RpcBpo2);
declare_test!(Reth, Risc0, RpcBpo2);
declare_test!(Reth, SP1, RpcBpo2);
declare_test!(Reth, Zisk, RpcBpo2);

// Zesu (only bal-devnet-7)

// NOTE: `should_panic` should be unnecessary when upgraded to `zisk@v1.0.0-beta`
declare_test!(Zesu, Zisk, EestBalDevnet7, should_panic);
