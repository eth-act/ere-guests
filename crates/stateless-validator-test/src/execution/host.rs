//! Host-side execution of stateless validator guests.

use std::io::{self, Write};

use ere_platform_core::Platform;

use crate::{
    execution::{ExecutionFailures, run_execution},
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
pub fn test_host_execution(preset: FixturePreset, execute: fn(&[u8]) -> Vec<u8>) {
    let failures = run_execution(preset_fixtures(preset), &|input| Ok(execute(&input)));
    assert!(failures.is_empty(), "{}", ExecutionFailures(&failures));
}

/// Declares a host execution test for a fixture preset and guest entrypoint.
#[macro_export]
macro_rules! declare_test_host_execution {
    ($preset:ident, $execute:ident) => {
        paste::paste! {
            #[test]
            fn [<test_host_execution_ $preset:snake>]() {
                use $crate::{
                    execution::host::{HostPlatform, test_host_execution},
                    fixture::FixturePreset,
                };
                test_host_execution(FixturePreset::$preset, $execute::<HostPlatform>);
            }
        }
    };
    ($preset:ident, $execute:ident, should_panic) => {
        paste::paste! {
            #[test]
            #[should_panic]
            fn [<test_host_execution_ $preset:snake>]() {
                use $crate::{
                    execution::host::{HostPlatform, test_host_execution},
                    fixture::FixturePreset,
                };
                test_host_execution(FixturePreset::$preset, $execute::<HostPlatform>);
            }
        }
    };
}
