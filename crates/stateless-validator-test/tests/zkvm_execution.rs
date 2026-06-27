//! Execution tests for stateless validator guest program

use ere_dockerized::zkVMKind;
use stateless_validator_test::{
    execution::{ExecutionFailures, GuestKind, zkvm::run_stateless_validator_execution},
    fixture::FixturePreset,
};

fn test_execution(guest_kind: GuestKind, zkvm_kind: zkVMKind, preset: FixturePreset) {
    let failures = run_stateless_validator_execution(guest_kind, zkvm_kind, preset);
    assert!(failures.is_empty(), "{}", ExecutionFailures(&failures));
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

// Reth (EEST is skipped)

declare_test!(Reth, Airbender, RpcBpo2);
declare_test!(Reth, Airbender, RpcGlamsterdamDevnet5);
declare_test!(Reth, OpenVM, RpcBpo2);
declare_test!(Reth, OpenVM, RpcGlamsterdamDevnet5);
declare_test!(Reth, Risc0, RpcBpo2);
declare_test!(Reth, Risc0, RpcGlamsterdamDevnet5);
declare_test!(Reth, SP1, RpcBpo2);
declare_test!(Reth, SP1, RpcGlamsterdamDevnet5);
declare_test!(Reth, Zisk, RpcBpo2);
declare_test!(Reth, Zisk, RpcGlamsterdamDevnet5);

// Zesu (only bal-devnet-7)

// NOTE: `should_panic` should be unnecessary when upgraded to `zisk@v1.0.0-beta`
declare_test!(Zesu, Zisk, EestBalDevnet7, should_panic);
// FIXME: Check why Zesu doesn't pass RpcGlamsterdamDevnet5.
// declare_test!(Zesu, Zisk, RpcGlamsterdamDevnet5);

// Nethermind

declare_test!(Nethermind, Zisk, RpcBpo2);
declare_test!(Nethermind, Zisk, RpcGlamsterdamDevnet5);
declare_test!(Nethermind, Zisk, EestBalDevnet7, should_panic);
