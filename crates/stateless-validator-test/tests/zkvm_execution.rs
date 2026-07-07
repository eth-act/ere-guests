//! Execution tests for stateless validator guest program

use ere_dockerized::zkVMKind;
use stateless_validator_test::{
    execution::{ExecutionFailures, GuestKind, zkvm::run_stateless_validator_execution},
    fixture::FixturePreset,
};

fn test_execution(
    guest_kind: GuestKind,
    zkvm_kind: zkVMKind,
    preset: FixturePreset,
    expected_failures: usize,
) {
    let failures = run_stateless_validator_execution(guest_kind, zkvm_kind, preset);
    assert_eq!(
        failures.len(),
        expected_failures,
        "expected {expected_failures} failures, got {}:\n{}",
        failures.len(),
        ExecutionFailures(&failures),
    );
}

macro_rules! declare_test {
    ($guest_kind:ident, $zkvm_kind:ident, $preset:ident, failures = $expected_failures:expr) => {
        paste::paste! {
            #[test]
            fn [<test_execution_ $guest_kind:lower _ $zkvm_kind:lower _ $preset:snake>]() {
                test_execution(
                    GuestKind::$guest_kind,
                    zkVMKind::$zkvm_kind,
                    FixturePreset::$preset,
                    $expected_failures,
                );
            }
        }
    };
    ($guest_kind:ident, $zkvm_kind:ident, $preset:ident) => {
        declare_test!($guest_kind, $zkvm_kind, $preset, failures = 0);
    };
}

// Ethrex

declare_test!(Ethrex, OpenVM, RpcBpo2);
declare_test!(Ethrex, OpenVM, RpcGlamsterdamDevnet5);
// Ethrex arithmetic overflow on 32-bit targets and calldata allocation OOM.
declare_test!(Ethrex, OpenVM, EestBalDevnet7, failures = 4);
declare_test!(Ethrex, SP1, RpcBpo2);
declare_test!(Ethrex, SP1, RpcGlamsterdamDevnet5);
declare_test!(Ethrex, SP1, EestBalDevnet7);
declare_test!(Ethrex, Zisk, RpcBpo2);
declare_test!(Ethrex, Zisk, RpcGlamsterdamDevnet5);
// Ethrex calldata allocation OOM + ZisK `zkvm-interface` impl bug.
declare_test!(Ethrex, Zisk, EestBalDevnet7, failures = 25);

// Reth

declare_test!(Reth, OpenVM, RpcBpo2);
declare_test!(Reth, OpenVM, RpcGlamsterdamDevnet5);
// Reth divergences.
declare_test!(Reth, OpenVM, EestBalDevnet7, failures = 16);
declare_test!(Reth, SP1, RpcBpo2);
declare_test!(Reth, SP1, RpcGlamsterdamDevnet5);
// Reth divergences.
declare_test!(Reth, SP1, EestBalDevnet7, failures = 16);
declare_test!(Reth, Zisk, RpcBpo2);
declare_test!(Reth, Zisk, RpcGlamsterdamDevnet5);
// Reth divergences + ZisK `zkvm-interface` impl bug.
declare_test!(Reth, Zisk, EestBalDevnet7, failures = 38);

// Zesu

// ZisK `zkvm-interface` impl bug.
declare_test!(Zesu, Zisk, EestBalDevnet7, failures = 121);
// Should be fixed by https://github.com/Consensys/zesu/pull/70.
declare_test!(Zesu, Zisk, RpcGlamsterdamDevnet5, failures = 50);
