//! ZisK Ethrex stateless validator guest program.

#![no_main]

use ere_platform_zisk::{ZiskPlatform, ziskos};
use stateless_validator_ethrex::guest::{Guest, StatelessValidatorEthrexGuest};

ziskos::entrypoint!(main);

fn main() {
    StatelessValidatorEthrexGuest::run_output_sha256::<ZiskPlatform>();
}
