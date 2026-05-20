//! ZisK Reth stateless validator guest program.

#![no_main]

use ere_platform_zisk::{ZiskPlatform, ziskos};
use stateless_validator_reth::guest::{Guest, StatelessValidatorRethGuest, zkvm_interface};

ziskos::entrypoint!(main);

fn main() {
    zkvm_interface::install_crypto();

    StatelessValidatorRethGuest::run_output_sha256::<ZiskPlatform>();
}

#[unsafe(no_mangle)]
fn _critical_section_1_0_acquire() -> u64 {
    return 0;
}

#[unsafe(no_mangle)]
fn _critical_section_1_0_release(_: u64) {}
