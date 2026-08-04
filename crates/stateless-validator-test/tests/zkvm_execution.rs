//! Execution tests for stateless validator guest program
//!
//! Run with env `ERE_IMAGE_REGISTRY=ghcr.io/eth-act/ere` to use the pre-built
//! image as the executor.
//!
//! Run with env `OPENVM_RUST_TOOLCHAIN=nightly-2026-01-18` for OpenVM to
//! compile Reth guest.

use ere_dockerized::zkVMKind;
use stateless_validator_catalog::StatelessValidatorKind;
use stateless_validator_test::{
    execution::{ExecutionFailures, zkvm::run_zkvm_execution},
    fixture::{FixturePreset, preset_fixtures},
};

fn test_execution(
    stateless_validator: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
    preset: FixturePreset,
    expected_failures: usize,
) {
    let failures = run_zkvm_execution(stateless_validator, zkvm_kind, preset_fixtures(preset));
    assert_eq!(
        failures.len(),
        expected_failures,
        "expected {expected_failures} failures, got {}:\n{}",
        failures.len(),
        ExecutionFailures(&failures),
    );
}

macro_rules! declare_test {
    ($stateless_validator:ident, $zkvm_kind:ident, $preset:ident, failures = $expected_failures:expr) => {
        paste::paste! {
            #[test]
            fn [<test_execution_ $stateless_validator:lower _ $zkvm_kind:lower _ $preset:snake>]() {
                test_execution(
                    StatelessValidatorKind::$stateless_validator,
                    zkVMKind::$zkvm_kind,
                    FixturePreset::$preset,
                    $expected_failures,
                );
            }
        }
    };
    ($stateless_validator:ident, $zkvm_kind:ident, $preset:ident) => {
        declare_test!($stateless_validator, $zkvm_kind, $preset, failures = 0);
    };
}

// Ethrex

declare_test!(Ethrex, OpenVM, RpcBpo2);
declare_test!(Ethrex, OpenVM, RpcGlamsterdamDevnet7);
// Ethrex arithmetic overflow on 32-bit targets + OOM.
declare_test!(Ethrex, OpenVM, EestGlamsterdamDevnet7, failures = 5);
declare_test!(Ethrex, SP1, RpcBpo2);
declare_test!(Ethrex, SP1, RpcGlamsterdamDevnet7);
declare_test!(Ethrex, SP1, EestGlamsterdamDevnet7);
declare_test!(Ethrex, Zisk, RpcBpo2);
declare_test!(Ethrex, Zisk, RpcGlamsterdamDevnet7);
// Ethrex OOM + ZisK `zkvm-interface` impl bug.
declare_test!(Ethrex, Zisk, EestGlamsterdamDevnet7, failures = 26);

// Reth

declare_test!(Reth, OpenVM, RpcBpo2);
// Reth divergences (in-block created-code resolution from EIP-8025).
declare_test!(Reth, OpenVM, RpcGlamsterdamDevnet7, failures = 1);
// Reth divergences.
declare_test!(Reth, OpenVM, EestGlamsterdamDevnet7, failures = 13);
declare_test!(Reth, SP1, RpcBpo2);
// Reth divergences (in-block created-code resolution from EIP-8025).
declare_test!(Reth, SP1, RpcGlamsterdamDevnet7, failures = 1);
// Reth divergences.
declare_test!(Reth, SP1, EestGlamsterdamDevnet7, failures = 13);
declare_test!(Reth, Zisk, RpcBpo2);
// Reth divergences (in-block created-code resolution from EIP-8025).
declare_test!(Reth, Zisk, RpcGlamsterdamDevnet7, failures = 1);
// Reth divergences + ZisK `zkvm-interface` impl bug.
declare_test!(Reth, Zisk, EestGlamsterdamDevnet7, failures = 35);

// Zesu

declare_test!(Zesu, Zisk, RpcBpo2);
declare_test!(Zesu, Zisk, RpcGlamsterdamDevnet7);
// ZisK `zkvm-interface` impl bug.
declare_test!(Zesu, Zisk, EestGlamsterdamDevnet7, failures = 22);
