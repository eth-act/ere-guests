//! Execution tests for stateless validator guest program
//!
//! Run with env `ERE_IMAGE_REGISTRY=ghcr.io/eth-act/ere` to use the pre-built
//! image as the executor.
//!
//! Run with env `OPENVM_RUST_TOOLCHAIN=nightly-2026-01-18` for OpenVM to
//! compile Reth guest.

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
// Ethrex arithmetic overflow on 32-bit targets + OOM.
declare_test!(Ethrex, OpenVM, EestGlamsterdamDevnet7, failures = 5);
declare_test!(Ethrex, SP1, RpcBpo2);
declare_test!(Ethrex, SP1, EestGlamsterdamDevnet7);
declare_test!(Ethrex, Zisk, RpcBpo2);
// Ethrex OOM + ZisK `zkvm-interface` impl bug.
declare_test!(Ethrex, Zisk, EestGlamsterdamDevnet7, failures = 26);

// Reth

declare_test!(Reth, OpenVM, RpcBpo2);
// Reth divergences.
declare_test!(Reth, OpenVM, EestGlamsterdamDevnet7, failures = 17);
declare_test!(Reth, SP1, RpcBpo2);
// Reth divergences.
declare_test!(Reth, SP1, EestGlamsterdamDevnet7, failures = 17);
declare_test!(Reth, Zisk, RpcBpo2);
// Reth divergences + ZisK `zkvm-interface` impl bug.
declare_test!(Reth, Zisk, EestGlamsterdamDevnet7, failures = 39);

// Zesu

// ZisK `zkvm-interface` impl bug + Zesu alignment issue.
// declare_test!(Zesu, Zisk, EestGlamsterdamDevnet5, failures = 121);
