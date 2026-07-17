//! Host-side execution of stateless validator guests.

use std::io::{self, Write};

use ere_platform_core::Platform;

use crate::{
    execution::{ExecutionFailures, GuestKind, run_execution},
    fixture::{FixturePreset, preset_fixtures},
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

/// Test execution on host.
pub fn test_host_execution(
    guest_kind: GuestKind,
    preset: FixturePreset,
    execute: fn(&[u8]) -> Vec<u8>,
    expected_failures: usize,
) {
    let failures = run_execution(guest_kind, preset_fixtures(preset), &|input| {
        Ok(execute(&input))
    });
    assert_eq!(
        failures.len(),
        expected_failures,
        "expected {expected_failures} failures, got {}:\n{}",
        failures.len(),
        ExecutionFailures(&failures),
    );
}

/// Declares a host execution test for a fixture preset and guest entrypoint.
#[macro_export]
macro_rules! declare_test_host_execution {
    ($guest:ident, $preset:ident, $execute:ident, failures = $expected_failures:expr) => {
        paste::paste! {
            #[test]
            fn [<test_host_execution_ $preset:snake>]() {
                use $crate::{
                    execution::{
                        GuestKind,
                        host::{HostPlatform, test_host_execution}
                    },
                    fixture::FixturePreset,
                };
                test_host_execution(
                    GuestKind::$guest,
                    FixturePreset::$preset,
                    $execute::<HostPlatform>,
                    $expected_failures,
                );
            }
        }
    };
    ($guest:ident, $preset:ident, $execute:ident) => {
        $crate::declare_test_host_execution!($guest, $preset, $execute, failures = 0);
    };
}
