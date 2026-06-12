//! Airbender Reth stateless validator guest program.

#![no_main]

use ere_platform_airbender::AirbenderPlatform;
use stateless_validator_reth::guest::entrypoint;

ere_platform_airbender::entrypoint!(main);

fn main() {
    entrypoint::<AirbenderPlatform>();
}

#[unsafe(no_mangle)]
fn _critical_section_1_0_acquire() -> u32 {
    return 0;
}

#[unsafe(no_mangle)]
fn _critical_section_1_0_release(_: u32) {}
