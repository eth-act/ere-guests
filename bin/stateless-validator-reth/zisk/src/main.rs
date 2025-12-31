//! ZisK Reth stateless validator guest program.

#![no_main]

use ere_platform_zisk::{ZiskPlatform, ziskos};
use stateless_validator_reth::guest::{Guest, StatelessValidatorRethGuest};

ziskos::entrypoint!(main);

fn main() {
    StatelessValidatorRethGuest::run_output_sha256::<ZiskPlatform>();
}
