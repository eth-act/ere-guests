//! Risc0 Ethrex stateless validator guest program.

use ere_platform_risc0::Risc0Platform;
use stateless_validator_ethrex::guest::entrypoint;

fn main() {
    entrypoint::<Risc0Platform>();
}
