//! Host-side execution of stateless validator guests.

use std::io::{self, Write};

use ere_platform_core::Platform;

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

/// Declares a host execution test for a fixture preset and guest entrypoint.
#[macro_export]
macro_rules! declare_test_execution_host {
    ($preset:ident, $execution:ident) => {
        paste::paste! {
            #[test]
            fn [<test_execution_host_ $preset:snake>]() {
                use $crate::{
                    execution::{ExecutionFailures, ExecutionOutput, host::HostPlatform, run_execution},
                    fixture::{FixturePreset, preset_fixtures},
                };

                let preset = FixturePreset::$preset;
                let failures = run_execution(preset_fixtures(preset), &|input| {
                    let output = $execution::<HostPlatform>(&input);
                    Ok(ExecutionOutput::Bytes(output))
                });
                assert!(
                    failures.is_empty(),
                    "{}",
                    ExecutionFailures(&failures)
                );
            }
        }
    };
}
