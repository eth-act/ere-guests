//! ZisK Ethrex stateless validator guest program.

#![no_main]

use ere_platform_zisk::{ZiskPlatform, ziskos};
use stateless_validator_ethrex::guest::entrypoint;

ziskos::entrypoint!(main);

fn main() {
    entrypoint::<ZiskPlatform>();
}
