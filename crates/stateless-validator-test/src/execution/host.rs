//! Host-side execution of stateless validator guests.

use std::io::{self, Write};

use ere_platform_core::Platform;
use stateless_validator_catalog::StatelessValidatorKind;

use crate::{
    execution::{ExecutionFailure, ExecutionFailures, run_execution},
    fixture::{FixturePreset, StatelessValidatorFixture, preset_fixtures},
};

/// A platform for host-side guest execution.
#[derive(Debug)]
pub struct HostPlatform;

impl Platform for HostPlatform {
    #[allow(unreachable_code)]
    fn read_input() -> impl std::ops::Deref<Target = [u8]> {
        unreachable!();
        &[] as &[u8]
    }

    fn write_output(_: &[u8]) {
        unreachable!();
    }

    fn print(message: &str) {
        print!("{message}");
        let _ = io::stdout().flush();
    }
}

/// Resolves the native guest entrypoint for `stateless_validator_kind`, then runs `fixtures`
/// through it on the host, returning the failures.
pub fn run_host_execution(
    stateless_validator_kind: StatelessValidatorKind,
    fixtures: impl IntoIterator<Item = StatelessValidatorFixture>,
) -> Vec<ExecutionFailure> {
    let execute: fn(&[u8]) -> Vec<u8> = match stateless_validator_kind {
        StatelessValidatorKind::Ethrex => {
            stateless_validator_ethrex::guest::run_stateless_guest::<HostPlatform>
        }
        StatelessValidatorKind::Reth => {
            stateless_validator_reth::guest::run_stateless_guest::<HostPlatform>
        }
        StatelessValidatorKind::Zesu => {
            panic!("host execution is not supported for the zesu guest")
        }
    };
    run_execution(fixtures, &|input| Ok(execute(&input)))
}

/// Runs `preset` on the host through the `stateless_validator_kind` guest, asserting the failure
/// count matches `expected_failures`.
pub fn test_host_execution(
    stateless_validator_kind: StatelessValidatorKind,
    preset: FixturePreset,
    expected_failures: usize,
) {
    let failures = run_host_execution(stateless_validator_kind, preset_fixtures(preset));
    assert_eq!(
        failures.len(),
        expected_failures,
        "expected {expected_failures} failures, got {}:\n{}",
        failures.len(),
        ExecutionFailures(&failures),
    );
}

/// Declares a host execution test for a guest kind and fixture preset.
#[macro_export]
macro_rules! declare_test_host_execution {
    ($kind:ident, $preset:ident, failures = $expected_failures:expr) => {
        paste::paste! {
            #[test]
            fn [<test_host_execution_ $preset:snake>]() {
                $crate::execution::host::test_host_execution(
                    $crate::StatelessValidatorKind::$kind,
                    $crate::fixture::FixturePreset::$preset,
                    $expected_failures,
                );
            }
        }
    };
    ($kind:ident, $preset:ident) => {
        $crate::declare_test_host_execution!($kind, $preset, failures = 0);
    };
}
